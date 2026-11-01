//! Seamless tiling by mirroring.
//!
//! Most procedural materials (noise based ones in particular) have a visible seam where
//! the texture repeats. Laying the image out as a 2x2 block, each neighbour a mirror of
//! the last, removes that seam by construction: the edges of the block meet their own
//! mirror image when it repeats. The price is visible symmetry, and the block is four
//! times the pixels.
//!
//! Normal maps need their channels corrected: a horizontal mirror reverses the slope
//! along the tangent (red), a vertical one reverses it along the bitangent (green).

use image::{imageops, Rgba, RgbaImage};

/// The image as a 2x2 mirrored block, twice as wide and as tall.
///
/// `normal_map` inverts red in the horizontally mirrored copies and green in the
/// vertically mirrored ones, so the lighting stays continuous across the mirror lines.
#[must_use]
pub fn mirror_tile(source: &RgbaImage, normal_map: bool) -> RgbaImage {
    let (w, h) = source.dimensions();
    let mut out = RgbaImage::new(w * 2, h * 2);
    let flip_h = imageops::flip_horizontal(source);
    let flip_v = imageops::flip_vertical(source);
    let flip_both = imageops::flip_vertical(&flip_h);
    let invert = |image: &RgbaImage, red: bool, green: bool| -> RgbaImage {
        if !normal_map {
            return image.clone();
        }
        let mut copy = image.clone();
        for Rgba([r, g, _, _]) in copy.pixels_mut() {
            if red {
                *r = 255 - *r;
            }
            if green {
                *g = 255 - *g;
            }
        }
        copy
    };
    imageops::replace(&mut out, source, 0, 0);
    imageops::replace(&mut out, &invert(&flip_h, true, false), i64::from(w), 0);
    imageops::replace(&mut out, &invert(&flip_v, false, true), 0, i64::from(h));
    imageops::replace(
        &mut out,
        &invert(&flip_both, true, true),
        i64::from(w),
        i64::from(h),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tile with no symmetry: every texel is distinct.
    fn gradient() -> RgbaImage {
        RgbaImage::from_fn(5, 3, |x, y| {
            Rgba([
                u8::try_from(x * 40 + 10).unwrap_or(255),
                u8::try_from(y * 70 + 20).unwrap_or(255),
                99,
                255,
            ])
        })
    }

    #[test]
    fn block_is_twice_the_size() {
        let out = mirror_tile(&gradient(), false);
        assert_eq!(out.dimensions(), (10, 6));
        // the original sits untouched in the top-left corner
        assert_eq!(out.get_pixel(2, 1), gradient().get_pixel(2, 1));
    }

    #[test]
    fn colour_maps_wrap_without_a_seam() {
        let out = mirror_tile(&gradient(), false);
        let (w, h) = out.dimensions();
        for y in 0..h {
            assert_eq!(
                out.get_pixel(0, y),
                out.get_pixel(w - 1, y),
                "column wrap at row {y}"
            );
        }
        for x in 0..w {
            assert_eq!(
                out.get_pixel(x, 0),
                out.get_pixel(x, h - 1),
                "row wrap at column {x}"
            );
        }
        // and the mirror line inside the block repeats the texel on either side
        assert_eq!(out.get_pixel(4, 0), out.get_pixel(5, 0));
    }

    #[test]
    fn normal_maps_flip_the_slope_across_each_mirror() {
        let source = gradient();
        let out = mirror_tile(&source, true);
        let (w, h) = out.dimensions();
        let a = source.get_pixel(0, 0).0;
        // across the wrap the right edge is the left edge mirrored: red inverted only
        let right = out.get_pixel(w - 1, 0).0;
        assert_eq!(right[0], 255 - a[0]);
        assert_eq!(right[1], a[1]);
        assert_eq!(right[2], a[2]);
        // across the vertical wrap the bottom edge is the top edge mirrored: green inverted only
        let bottom = out.get_pixel(0, h - 1).0;
        assert_eq!(bottom[0], a[0]);
        assert_eq!(bottom[1], 255 - a[1]);
        // the diagonal copy has both inverted
        let corner = out.get_pixel(w - 1, h - 1).0;
        assert_eq!((corner[0], corner[1]), (255 - a[0], 255 - a[1]));
        // blue (the surface-facing component) never changes
        assert_eq!(corner[2], a[2]);
    }
}
