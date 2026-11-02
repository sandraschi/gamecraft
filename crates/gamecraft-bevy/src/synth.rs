//! Procedural sound: every effect is computed from formulas, so the repo carries no
//! audio files and the same build always sounds the same.
//!
//! Effects are mono `f32` buffers at [`SAMPLE_RATE`]; [`wav_bytes`] wraps one in a
//! 16-bit PCM WAV that Bevy's WAV decoder plays. The module has no Bevy dependency, so
//! `cargo test` checks the waveforms (non-silent, never clipping, bit-identical on every
//! run, the ambient bed loops without a click) on a machine with no sound card.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::f32::consts::{PI, TAU};

/// Samples per second of every effect.
pub const SAMPLE_RATE: u32 = 22_050;

/// Hand-rolled xorshift noise in `[-1, 1)`, so no random crate is needed.
struct Noise(u32);

impl Noise {
    fn new(seed: u32) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn samples_for(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE as f32) as usize
}

/// One-pole low-pass step: `coeff` near 0 is dark, near 1 is bright.
fn smooth(prev: f32, input: f32, coeff: f32) -> f32 {
    prev + coeff * (input - prev)
}

/// Scale so the loudest sample sits at `peak`.
fn normalise(samples: &mut [f32], peak: f32) {
    let loudest = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    if loudest > 0.0 {
        let gain = peak / loudest;
        for s in samples {
            *s *= gain;
        }
    }
}

/// Wrap mono samples as a 16-bit PCM WAV file.
#[must_use]
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = u32::try_from(samples.len() * 2).unwrap_or(u32::MAX);
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1_u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2_u16.to_le_bytes());
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32_767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Evening wind and wing rustle, 8 s, built to loop: the sines complete whole cycles
/// and the end of the noise is cross-faded into the start.
#[must_use]
pub fn ambient() -> Vec<f32> {
    let seconds = 8.0;
    let fade = samples_for(0.5);
    let len = samples_for(seconds);
    let mut noise = Noise::new(0xA11C_E5ED);
    let mut poles = [0.0_f32; 3];
    let mut body = Vec::with_capacity(len + fade);
    for i in 0..len + fade {
        let t = i as f32 / SAMPLE_RATE as f32;
        let mut breeze = noise.next();
        for pole in &mut poles {
            *pole = smooth(*pole, breeze, 0.04);
            breeze = *pole;
        }
        let wing = 0.6 + 0.4 * (TAU * t * 0.25).sin();
        let gust = 0.7 + 0.3 * (TAU * t * 0.625).sin();
        let bed = breeze * 7.0 * wing * gust;
        let pad = 0.06 * (TAU * 110.0 * t).sin() + 0.04 * (TAU * 165.0 * t).sin();
        body.push(bed + pad);
    }
    let mut looped: Vec<f32> = body[..len].to_vec();
    for i in 0..fade {
        let a = i as f32 / fade as f32;
        looped[i] = body[i] * a + body[len + i] * (1.0 - a);
    }
    normalise(&mut looped, 0.45);
    looped
}

/// A hawk's cry: a falling sweep with a flutter, 0.5 s.
#[must_use]
pub fn screech() -> Vec<f32> {
    let len = samples_for(0.5);
    let mut noise = Noise::new(0x5C4E_EC11);
    let mut phase = 0.0;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let freq = 2200.0 + 1200.0 * (-5.0 * t).exp() + 60.0 * (TAU * 28.0 * t).sin();
        phase += TAU * freq / SAMPLE_RATE as f32;
        let env = (1.0 - (-t * 300.0).exp()) * (-t * 5.0).exp();
        out.push(env * (0.7 * phase.sin() + 0.25 * (2.0 * phase).sin() + 0.12 * noise.next()));
    }
    normalise(&mut out, 0.7);
    out
}

/// The flare: a rising whoosh of noise with a falling sparkle, 0.9 s.
#[must_use]
pub fn flare() -> Vec<f32> {
    let seconds = 0.9;
    let len = samples_for(seconds);
    let mut noise = Noise::new(0xF1A5_E001);
    let mut low = 0.0;
    let mut phase = 0.0;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let progress = t / seconds;
        low = smooth(low, noise.next(), 0.02 + 0.5 * progress);
        let env = (PI * progress).sin().powi(2);
        let freq = 1800.0 - 1200.0 * progress;
        phase += TAU * freq / SAMPLE_RATE as f32;
        out.push(env * (low * 2.5 + 0.12 * phase.sin()));
    }
    normalise(&mut out, 0.7);
    out
}

/// A rising four-note chime for a win, 2.2 s.
#[must_use]
pub fn win_chime() -> Vec<f32> {
    let len = samples_for(2.2);
    let notes = [
        (523.25_f32, 0.0_f32),
        (659.25, 0.18),
        (783.99, 0.36),
        (1046.5, 0.54),
    ];
    let mut out = vec![0.0_f32; len];
    for (freq, start) in notes {
        let first = samples_for(start);
        for (k, slot) in out.iter_mut().enumerate().skip(first) {
            let t = (k - first) as f32 / SAMPLE_RATE as f32;
            let env = (1.0 - (-t * 200.0).exp()) * (-t * 3.5).exp();
            *slot += env * ((TAU * freq * t).sin() + 0.3 * (TAU * 2.0 * freq * t).sin());
        }
    }
    normalise(&mut out, 0.75);
    out
}

/// A low falling tone for a loss, 1.4 s.
#[must_use]
pub fn loss_tone() -> Vec<f32> {
    let seconds = 1.4;
    let len = samples_for(seconds);
    let mut phase = 0.0;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32 / SAMPLE_RATE as f32;
        let freq = 196.0 - 49.0 * (t / seconds);
        phase += TAU * freq / SAMPLE_RATE as f32;
        let env = (1.0 - (-t * 40.0).exp()) * (-t * 2.2).exp();
        out.push(env * (phase.sin() + 0.4 * (0.5 * phase).sin()));
    }
    normalise(&mut out, 0.7);
    out
}

/// A short rising blip for one maze step, 70 ms.
#[must_use]
pub fn blip() -> Vec<f32> {
    let len = samples_for(0.07);
    let mut phase = 0.0;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let progress = i as f32 / len as f32;
        phase += TAU * (520.0 + 400.0 * progress) / SAMPLE_RATE as f32;
        out.push((1.0 - progress).powi(2) * phase.sin());
    }
    normalise(&mut out, 0.5);
    out
}

/// A dull thump for walking into a wall, 110 ms.
#[must_use]
pub fn bump() -> Vec<f32> {
    let len = samples_for(0.11);
    let mut phase = 0.0;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let progress = i as f32 / len as f32;
        phase += TAU * (110.0 - 40.0 * progress) / SAMPLE_RATE as f32;
        out.push((1.0 - progress).powi(2) * phase.sin());
    }
    normalise(&mut out, 0.6);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects() -> Vec<(&'static str, Vec<f32>)> {
        vec![
            ("ambient", ambient()),
            ("screech", screech()),
            ("flare", flare()),
            ("win_chime", win_chime()),
            ("loss_tone", loss_tone()),
            ("blip", blip()),
            ("bump", bump()),
        ]
    }

    #[test]
    fn every_effect_is_audible_and_never_clips() {
        for (name, samples) in effects() {
            assert!(!samples.is_empty(), "{name} is empty");
            assert!(samples.iter().all(|s| s.is_finite()), "{name} has NaN");
            let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.3, "{name} is nearly silent (peak {peak})");
            assert!(peak <= 0.95, "{name} is too hot (peak {peak})");
        }
    }

    #[test]
    fn effects_are_deterministic() {
        assert_eq!(ambient(), ambient());
        assert_eq!(screech(), screech());
        assert_eq!(wav_bytes(&flare()), wav_bytes(&flare()));
    }

    #[test]
    fn effects_have_sensible_lengths() {
        let secs = |s: &[f32]| s.len() as f32 / SAMPLE_RATE as f32;
        assert!((secs(&ambient()) - 8.0).abs() < 0.01);
        assert!(secs(&blip()) < 0.1);
        assert!(secs(&win_chime()) > 2.0);
    }

    #[test]
    fn ambient_loops_without_a_click() {
        let bed = ambient();
        let seam = (bed[0] - bed[bed.len() - 1]).abs();
        let busiest = bed
            .windows(2)
            .fold(0.0_f32, |m, w| m.max((w[1] - w[0]).abs()));
        assert!(
            seam <= busiest,
            "loop seam jumps {seam}, louder than the bed's own steps {busiest}"
        );
    }

    #[test]
    fn wav_header_describes_the_data() {
        let samples = blip();
        let wav = wav_bytes(&samples);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(
            u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]),
            SAMPLE_RATE
        );
        let data_len = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
        assert_eq!(data_len, samples.len() * 2);
        assert_eq!(wav.len(), 44 + data_len);
    }
}
