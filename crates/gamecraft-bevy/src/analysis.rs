//! Looking at sound: spectrograms, levels and a dependency-free PNG writer.
//!
//! Used by `gamecraft-synth-dump`, which writes every effect as a WAV, a spectrogram
//! PNG and a loudness table into `docs/sounds/`. A spectrogram shows what a waveform
//! does over time (a falling sweep, four chime notes, rumble versus hiss), so the
//! effects can be reviewed and compared without listening. No Bevy, no image crate.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use crate::synth::SAMPLE_RATE;
use std::f32::consts::TAU;

/// FFT window length in samples (23 ms at 22 050 Hz).
pub const WINDOW: usize = 512;
/// Samples between two windows.
pub const HOP: usize = 256;
/// Quietest level drawn, in dB below the loudest bin.
pub const FLOOR_DB: f32 = -90.0;
/// Short sounds are stretched horizontally (whole columns) to at least this many pixels.
pub const MIN_WIDTH: usize = 480;
/// Pixels reserved under the picture for the time scale.
pub const FOOTER: usize = 14;

/// In-place radix-2 FFT. Both slices must have the same power-of-two length.
pub fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    debug_assert!(n.is_power_of_two() && im.len() == n);
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let step = -TAU / len as f32;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (sin, cos) = (step * k as f32).sin_cos();
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cos - im[b] * sin;
                let ti = re[b] * sin + im[b] * cos;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
}

/// A magnitude spectrogram in dB relative to its loudest cell.
#[derive(Debug, Clone)]
pub struct Spectrogram {
    /// Time steps.
    pub frames: usize,
    /// Frequency bins, 0 Hz to Nyquist.
    pub bins: usize,
    /// `db[frame * bins + bin]`, from [`FLOOR_DB`] to 0.
    pub db: Vec<f32>,
    /// Magnitude-weighted mean frequency over the whole sound, in Hz.
    pub centroid_hz: f32,
}

/// Short-time Fourier transform of `samples` with a Hann window.
#[must_use]
pub fn spectrogram(samples: &[f32]) -> Spectrogram {
    let bins = WINDOW / 2 + 1;
    let frames = if samples.len() < WINDOW {
        1
    } else {
        (samples.len() - WINDOW) / HOP + 1
    };
    let hann: Vec<f32> = (0..WINDOW)
        .map(|i| 0.5 - 0.5 * (TAU * i as f32 / WINDOW as f32).cos())
        .collect();
    let mut mags = vec![0.0_f32; frames * bins];
    let mut weighted = 0.0_f64;
    let mut total = 0.0_f64;
    let bin_hz = SAMPLE_RATE as f32 / WINDOW as f32;
    let mut re = vec![0.0_f32; WINDOW];
    let mut im = vec![0.0_f32; WINDOW];
    for frame in 0..frames {
        for i in 0..WINDOW {
            re[i] = samples.get(frame * HOP + i).copied().unwrap_or(0.0) * hann[i];
            im[i] = 0.0;
        }
        fft(&mut re, &mut im);
        for bin in 0..bins {
            let mag = re[bin].hypot(im[bin]) / (WINDOW as f32 / 4.0);
            mags[frame * bins + bin] = mag;
            weighted += f64::from(mag) * f64::from(bin as f32 * bin_hz);
            total += f64::from(mag);
        }
    }
    let loudest = mags.iter().fold(0.0_f32, |m, v| m.max(*v)).max(1e-9);
    let db = mags
        .iter()
        .map(|m| (20.0 * (m / loudest + 1e-9).log10()).clamp(FLOOR_DB, 0.0))
        .collect();
    let centroid_hz = if total > 0.0 {
        (weighted / total) as f32
    } else {
        0.0
    };
    Spectrogram {
        frames,
        bins,
        db,
        centroid_hz,
    }
}

/// Loudness summary of one effect.
#[derive(Debug, Clone, Copy)]
pub struct Levels {
    /// Length in seconds.
    pub seconds: f32,
    /// Loudest sample, dB below full scale.
    pub peak_db: f32,
    /// Root-mean-square level, dB below full scale.
    pub rms_db: f32,
}

/// Peak and RMS of `samples`.
#[must_use]
pub fn levels(samples: &[f32]) -> Levels {
    let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    let mean_square = if samples.is_empty() {
        0.0
    } else {
        samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32
    };
    Levels {
        seconds: samples.len() as f32 / SAMPLE_RATE as f32,
        peak_db: 20.0 * peak.max(1e-9).log10(),
        rms_db: 10.0 * mean_square.max(1e-18).log10(),
    }
}

/// Colour of one cell: black, purple, red, orange, pale yellow as the level rises.
fn heat(db: f32) -> [u8; 3] {
    const STOPS: [(f32, [f32; 3]); 5] = [
        (0.0, [0.0, 0.0, 4.0]),
        (0.25, [60.0, 15.0, 110.0]),
        (0.5, [180.0, 55.0, 90.0]),
        (0.75, [250.0, 140.0, 10.0]),
        (1.0, [252.0, 255.0, 160.0]),
    ];
    let level = ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0).powf(1.6);
    for pair in STOPS.windows(2) {
        let (lo, hi) = (pair[0], pair[1]);
        if level <= hi.0 {
            let t = (level - lo.0) / (hi.0 - lo.0);
            return [
                (lo.1[0] + (hi.1[0] - lo.1[0]) * t) as u8,
                (lo.1[1] + (hi.1[1] - lo.1[1]) * t) as u8,
                (lo.1[2] + (hi.1[2] - lo.1[2]) * t) as u8,
            ];
        }
    }
    [252, 255, 160]
}

/// Render a spectrogram as an RGB image: time left to right, 0 Hz at the bottom,
/// Nyquist at the top. White ticks on the left mark every 1 kHz (long every 5 kHz);
/// the footer marks every 0.5 s (long every 1 s).
#[must_use]
pub fn spectrogram_png(spec: &Spectrogram) -> Vec<u8> {
    let scale = MIN_WIDTH.div_ceil(spec.frames).max(1);
    let width = spec.frames * scale;
    let height = spec.bins + FOOTER;
    let mut rgb = vec![0_u8; width * height * 3];
    let mut put = |x: usize, y: usize, color: [u8; 3]| {
        if x < width && y < height {
            let at = (y * width + x) * 3;
            rgb[at..at + 3].copy_from_slice(&color);
        }
    };
    for frame in 0..spec.frames {
        for bin in 0..spec.bins {
            let color = heat(spec.db[frame * spec.bins + bin]);
            for dx in 0..scale {
                put(frame * scale + dx, spec.bins - 1 - bin, color);
            }
        }
    }
    let bin_hz = SAMPLE_RATE as f32 / WINDOW as f32;
    let mut khz = 1;
    while (khz as f32 * 1000.0 / bin_hz) < spec.bins as f32 {
        let row = spec.bins - 1 - (khz as f32 * 1000.0 / bin_hz) as usize;
        let tick = if khz % 5 == 0 { 14 } else { 6 };
        for x in 0..tick {
            put(x, row, [255, 255, 255]);
        }
        khz += 1;
    }
    let frames_per_second = SAMPLE_RATE as f32 / HOP as f32;
    let mut half = 0;
    while (half as f32 * 0.5 * frames_per_second) < spec.frames as f32 {
        let column = (half as f32 * 0.5 * frames_per_second) as usize * scale;
        let tick = if half % 2 == 0 { 12 } else { 6 };
        for y in 0..tick {
            put(column, spec.bins + y, [255, 255, 255]);
        }
        half += 1;
    }
    png_rgb(width as u32, height as u32, &rgb)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut low, mut high) = (1_u32, 0_u32);
    for byte in bytes {
        low = (low + u32::from(*byte)) % 65_521;
        high = (high + low) % 65_521;
    }
    (high << 16) | low
}

fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Encode 8-bit RGB pixels as a PNG (stored deflate blocks: big but dependency-free).
#[must_use]
pub fn png_rgb(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let row_len = width as usize * 3;
    let mut raw = Vec::with_capacity((row_len + 1) * height as usize);
    for row in rgb.chunks(row_len.max(1)) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (index, block) in blocks.iter().enumerate() {
        let last = index + 1 == blocks.len();
        zlib.push(u8::from(last));
        let len = u16::try_from(block.len()).unwrap_or(u16::MAX);
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    chunk(&mut out, *b"IHDR", &header);
    chunk(&mut out, *b"IDAT", &zlib);
    chunk(&mut out, *b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, seconds: f32) -> Vec<f32> {
        (0..(seconds * SAMPLE_RATE as f32) as usize)
            .map(|i| (TAU * freq * i as f32 / SAMPLE_RATE as f32).sin())
            .collect()
    }

    #[test]
    fn fft_finds_a_pure_tone() {
        let spec = spectrogram(&sine(2_000.0, 0.5));
        let bin_hz = SAMPLE_RATE as f32 / WINDOW as f32;
        let frame = spec.frames / 2;
        let row = &spec.db[frame * spec.bins..(frame + 1) * spec.bins];
        let loudest = row
            .iter()
            .enumerate()
            .fold(
                (0, FLOOR_DB),
                |best, (bin, db)| {
                    if *db > best.1 {
                        (bin, *db)
                    } else {
                        best
                    }
                },
            )
            .0;
        assert!((loudest as f32 * bin_hz - 2_000.0).abs() < bin_hz * 1.5);
        assert!(
            (spec.centroid_hz - 2_000.0).abs() < 150.0,
            "{}",
            spec.centroid_hz
        );
    }

    #[test]
    fn levels_of_a_half_scale_sine() {
        let half: Vec<f32> = sine(440.0, 1.0).iter().map(|s| s * 0.5).collect();
        let lv = levels(&half);
        assert!((lv.peak_db + 6.02).abs() < 0.1, "{}", lv.peak_db);
        assert!((lv.rms_db + 9.03).abs() < 0.1, "{}", lv.rms_db);
        assert!((lv.seconds - 1.0).abs() < 0.001);
    }

    #[test]
    fn png_is_well_formed() {
        let png = png_rgb(2, 2, &[255, 0, 0, 0, 255, 0, 0, 0, 255, 9, 9, 9]);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        // the CRC of a known string
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn spectrogram_image_has_the_expected_size() {
        let spec = spectrogram(&sine(1_000.0, 0.3));
        let png = spectrogram_png(&spec);
        let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]) as usize;
        let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]) as usize;
        assert_eq!(width, spec.frames * MIN_WIDTH.div_ceil(spec.frames));
        assert!(width >= MIN_WIDTH);
        assert_eq!(height, spec.bins + FOOTER);
    }
}
