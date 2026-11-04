//! Procedural texture depot: deterministic PNGs generated, not downloaded.
//!
//! Every texture is a pure function of `(x, y)`, so the bytes are identical on
//! every run. That keeps the depot reproducible: no binary assets checked in, no
//! source images to lose.

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use std::io::Cursor;

/// Texture edge length in pixels.
pub const SIZE: u32 = 64;

/// One generated texture: file name plus PNG bytes.
pub type TextureFile = (&'static str, Vec<u8>);

/// Deterministic per-pixel noise in `-spread..=spread`.
fn jitter(x: u32, y: u32, spread: i32) -> i32 {
    let hash = x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663);
    let folded = (hash >> 8) % (u32::try_from(spread * 2 + 1).unwrap_or(1));
    i32::try_from(folded).unwrap_or(0) - spread
}

/// Clamp a channel into `0..=255`.
fn channel(value: i32) -> u8 {
    u8::try_from(value.clamp(0, 255)).unwrap_or(0)
}

/// Build an image from a per-pixel colour function.
fn render(mut shade: impl FnMut(u32, u32) -> (u8, u8, u8)) -> Vec<u8> {
    let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(SIZE, SIZE, |x, y| {
        let (red, green, blue) = shade(x, y);
        Rgba([red, green, blue, 255])
    });
    let mut out: Vec<u8> = Vec::new();
    let _ = DynamicImage::ImageRgba8(buffer).write_to(&mut Cursor::new(&mut out), ImageFormat::Png);
    out
}

/// Running-bond brick with mortar lines.
#[must_use]
pub fn brick() -> Vec<u8> {
    render(|x, y| {
        let row = y / 16;
        let offset = if row % 2 == 0 { 0 } else { 16 };
        let mortar = y % 16 < 2 || (x + offset) % 32 < 2;
        if mortar {
            (198, 194, 186)
        } else {
            let n = jitter(x, y, 10);
            (channel(150 + n), channel(78 + n), channel(64 + n))
        }
    })
}

/// Painted plaster with fine noise.
#[must_use]
pub fn plaster() -> Vec<u8> {
    render(|x, y| {
        let n = jitter(x, y, 7);
        (channel(226 + n), channel(220 + n), channel(206 + n))
    })
}

/// Roof tiles: horizontal courses with a darker lower edge.
#[must_use]
pub fn roof_tile() -> Vec<u8> {
    render(|x, y| {
        let course = y % 10;
        let edge = course < 2;
        let n = jitter(x / 2, y, 6);
        let base = if edge { 118 } else { 158 };
        (
            channel(base + n),
            channel(base / 2 + 22 + n),
            channel(base / 2 + 14 + n),
        )
    })
}

/// Grass: two greens speckled.
#[must_use]
pub fn grass() -> Vec<u8> {
    render(|x, y| {
        let n = jitter(x / 2, y / 2, 14);
        (channel(96 + n), channel(132 + n), channel(74 + n))
    })
}

/// The whole depot, ready to write beside a scene.
#[must_use]
pub fn depot() -> Vec<TextureFile> {
    vec![
        ("brick.png", brick()),
        ("plaster.png", plaster()),
        ("roof_tile.png", roof_tile()),
        ("grass.png", grass()),
        ("barn_pbr.png", barn_pbr()),
    ]
}

/// Barn wood (TextureCraft PBR baseColor, baked from builtin:crate --sample).
/// Procedural brick stays for church/castle (brick-appropriate); barn gets wood.
#[must_use]
pub fn barn_pbr() -> Vec<u8> {
    include_bytes!("../assets/barn_pbr.png").to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_texture_is_a_png() {
        for (name, bytes) in depot() {
            assert!(bytes.len() > 100, "{name} is too small to be a png");
            // PNG magic.
            assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10], "{name}");
        }
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(brick(), brick());
        assert_eq!(grass(), grass());
    }

    #[test]
    fn textures_differ_from_each_other() {
        let hashes: HashSet<Vec<u8>> = depot().into_iter().map(|(_, bytes)| bytes).collect();
        assert_eq!(hashes.len(), 5, "patterns should not collide");
    }
}
