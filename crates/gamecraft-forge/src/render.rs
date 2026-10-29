//! Deterministic software rasterizer for depot thumbnails and trace GIFs.
//!
//! World units map into a 320x180 frame with letterboxing. The only
//! float-to-pixel conversion lives in [`pixel`], which saturates.

use std::fs::File;
use std::io::BufWriter;

/// Frame width in pixels.
pub const W: u16 = 320;
/// Frame height in pixels.
pub const H: u16 = 180;

/// Palette indices: 0 ground, 1 platform, 2 player, 3 coin, 4 goal,
/// 5 kill, 6 enemy, 7 bullet, 8 tower, 9 paddle, 10 ball, 11 wall,
/// 12 target, 13 door, 14 npc, 15 cursor.
pub const PALETTE: [[u8; 3]; 16] = [
    [139, 92, 46],
    [74, 160, 90],
    [90, 150, 255],
    [255, 205, 60],
    [235, 80, 80],
    [120, 20, 20],
    [165, 110, 235],
    [240, 240, 245],
    [150, 150, 160],
    [110, 220, 255],
    [255, 150, 60],
    [60, 60, 70],
    [80, 200, 190],
    [150, 100, 60],
    [255, 150, 200],
    [255, 190, 60],
];

/// Sky background per game.
pub const SKY: [u8; 3] = [16, 20, 36];

/// One drawable: world-space rect or circle plus a palette color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    /// World x (center).
    pub x: f32,
    /// World y (center).
    pub y: f32,
    /// World width (or diameter for circles).
    pub w: f32,
    /// World height (ignored for circles).
    pub h: f32,
    /// Palette index.
    pub color: u8,
    /// True = filled circle instead of rect.
    pub circle: bool,
}

/// World rect visible in the frame: `(min_x, min_y, width, height)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Left edge.
    pub x: f32,
    /// Bottom edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

/// Float pixel coordinate to buffer index (saturating).
///
/// Rasterization fundamentally needs this conversion; it saturates instead
/// of wrapping, and it is the only `as` cast in the workspace-adjacent code.
fn pixel(value: f32, max: usize) -> usize {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let raw = value as usize;
    raw.min(max)
}

/// Render one RGBA frame (row-major, top row first).
#[must_use]
pub fn render_frame(view: View, statics: &[Glyph], dynamics: &[Glyph]) -> Vec<u8> {
    let width = usize::from(W);
    let height = usize::from(H);
    let mut buf = vec![0_u8; width * height * 4];
    for (index, byte) in buf.iter_mut().enumerate() {
        let channel = index % 4;
        *byte = if channel < 3 { SKY[channel] } else { 255 };
    }
    let scale = (f32::from(W) / view.w).min(f32::from(H) / view.h);
    let off_x = (f32::from(W) - view.w * scale) * 0.5 - view.x * scale;
    let off_y = (f32::from(H) - view.h * scale) * 0.5 + view.y * scale;
    let project = |wx: f32, wy: f32| -> (usize, usize) {
        let sx = off_x + wx * scale;
        let sy = f32::from(H) - (off_y + wy * scale);
        (pixel(sx, width - 1), pixel(sy, height - 1))
    };
    for glyph in statics.iter().chain(dynamics.iter()) {
        draw_glyph(&mut buf, glyph, &project, width, height, scale);
    }
    buf
}

/// Lossless pixel index to float (frame is 320x180, far below `u16::MAX`).
fn to_f32(index: usize) -> f32 {
    f32::from(u16::try_from(index).unwrap_or(0))
}

/// Stamp one glyph into the buffer.
fn draw_glyph(
    buf: &mut [u8],
    glyph: &Glyph,
    project: &dyn Fn(f32, f32) -> (usize, usize),
    width: usize,
    height: usize,
    scale: f32,
) {
    let color = PALETTE[usize::from(glyph.color) % PALETTE.len()];
    if glyph.circle {
        let (cx, cy) = project(glyph.x, glyph.y);
        let radius = (glyph.w * 0.5 * scale).max(1.0);
        let reach = pixel(radius + 1.0, width);
        let x0 = cx.saturating_sub(reach);
        let x1 = (cx + reach).min(width - 1);
        let y0 = cy.saturating_sub(reach);
        let y1 = (cy + reach).min(height - 1);
        for py in y0..=y1 {
            for px in x0..=x1 {
                let dx = to_f32(px) - to_f32(cx);
                let dy = to_f32(py) - to_f32(cy);
                if dx * dx + dy * dy <= radius * radius {
                    paint(buf, px, py, width, color);
                }
            }
        }
    } else {
        let (x0, y0) = project(glyph.x - glyph.w * 0.5, glyph.y + glyph.h * 0.5);
        let (x1, y1) = project(glyph.x + glyph.w * 0.5, glyph.y - glyph.h * 0.5);
        let (lox, hix) = (x0.min(x1), x0.max(x1));
        let (loy, hiy) = (y0.min(y1), y0.max(y1));
        for py in loy..=hiy {
            for px in lox..=hix {
                paint(buf, px, py, width, color);
            }
        }
    }
}

/// Write one RGBA pixel.
fn paint(buf: &mut [u8], x: usize, y: usize, width: usize, color: [u8; 3]) {
    let at = (y * width + x) * 4;
    if let Some(slot) = buf.get_mut(at..at + 4) {
        slot[0] = color[0];
        slot[1] = color[1];
        slot[2] = color[2];
        slot[3] = 255;
    }
}

/// Write an RGBA frame as PNG.
///
/// # Errors
///
/// Returns a message when the file cannot be written.
pub fn write_png(path: &str, rgba: &[u8]) -> Result<(), String> {
    use image::{Rgba, RgbaImage};
    let width = usize::from(W);
    let height = usize::from(H);
    if rgba.len() != width * height * 4 {
        return Err(format!(
            "bad frame size {} for {}x{width}",
            rgba.len(),
            height
        ));
    }
    let mut img = RgbaImage::new(u32::from(W), u32::from(H));
    for (index, quad) in rgba.as_chunks::<4>().0.iter().enumerate() {
        let x = index % width;
        let y = index / width;
        let xu = u32::try_from(x).map_err(|_| "frame too wide".to_string())?;
        let yu = u32::try_from(y).map_err(|_| "frame too tall".to_string())?;
        img.put_pixel(xu, yu, Rgba([quad[0], quad[1], quad[2], quad[3]]));
    }
    img.save(path).map_err(|err| err.to_string())
}

/// Write frames as a looping GIF.
///
/// # Errors
///
/// Returns a message when the file cannot be written.
pub fn write_gif(path: &str, frames: &[Vec<u8>], delay_cs: u16) -> Result<(), String> {
    use gif::{Encoder, Frame, Repeat};
    let file = File::create(path).map_err(|err| err.to_string())?;
    let mut encoder =
        Encoder::new(BufWriter::new(file), W, H, &[]).map_err(|err| err.to_string())?;
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|err| err.to_string())?;
    for rgba in frames {
        let mut owned = rgba.clone();
        let mut frame = Frame::from_rgba_speed(W, H, &mut owned, 10);
        frame.delay = delay_cs;
        encoder.write_frame(&frame).map_err(|err| err.to_string())?;
    }
    Ok(())
}
