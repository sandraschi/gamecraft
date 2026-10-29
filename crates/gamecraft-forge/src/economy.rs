//! Phase 8: monetization as a deterministic state machine, plus store assets.
//!
//! Two halves, both testable:
//!
//! 1. [`economy_lint`] simulates a currency sink/faucet economy day by day with a
//!    seeded RNG and reports whether it actually creates scarcity, whether pity
//!    guarantees are reachable, and whether the tiers are decoupled. These are
//!    numbers, not opinions.
//! 2. [`write_steam_bundle`] emits the asset bundle a store page needs: trading
//!    cards, emoticons, profile backgrounds, a badge definition, the Supporter
//!    Edition manifest, a cloud-save schema, and a soundtrack file.
//!
//! Scope note: the soundtrack is **WAV**, not FLAC. Producing real FLAC needs an
//! encoder this crate does not carry; claiming `.flac` while writing PCM would be
//! a lie in a filename, so the manifest names what actually exists.

use image::{ImageBuffer, Rgb, Rgba};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One lint finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Machine code (`ERR_...`).
    pub code: String,
    /// Human explanation.
    pub message: String,
}

impl Finding {
    /// Build a finding.
    fn new(code: &str, message: String) -> Self {
        Self {
            code: code.to_string(),
            message,
        }
    }
}

/// Three-tier currency model: real money, premium, energy.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct EconomySpec {
    /// Premium currency name.
    pub premium: String,
    /// Soft currency name.
    pub soft: String,
    /// Energy/action currency name.
    pub energy: String,
    /// Soft currency earned per day from all faucets.
    pub faucet_per_day: f64,
    /// Soft currency a player must spend per day to keep pace.
    pub sink_per_day: f64,
    /// Premium currency handed out free per day (should be small).
    pub premium_faucet_per_day: f64,
    /// Energy regenerated per day.
    pub energy_per_day: f64,
    /// Energy consumed per pull.
    pub energy_per_pull: f64,
    /// Base probability of the top rarity per pull.
    pub base_rate: f64,
    /// Pulls after which the rate starts ramping.
    pub soft_pity: u32,
    /// Pulls at which the top rarity is guaranteed.
    pub hard_pity: u32,
    /// Days the economy is simulated for.
    pub days: u32,
    /// Whether the $0.99 starter anchor exists.
    pub starter_anchor: bool,
}

impl Default for EconomySpec {
    fn default() -> Self {
        Self {
            premium: "Gems".to_string(),
            soft: "Coins".to_string(),
            energy: "Energy".to_string(),
            faucet_per_day: 120.0,
            sink_per_day: 180.0,
            premium_faucet_per_day: 2.0,
            energy_per_day: 240.0,
            energy_per_pull: 60.0,
            base_rate: 0.006,
            soft_pity: 74,
            hard_pity: 90,
            days: 7,
            starter_anchor: true,
        }
    }
}

/// Result of one simulated run.
#[derive(Debug, Clone, Serialize)]
pub struct EconomyReport {
    /// Balance at the end of each day.
    pub balances: Vec<f64>,
    /// First day the player cannot afford a full day of sinks (`None` = never).
    pub scarcity_day: Option<u32>,
    /// Pulls performed across the run.
    pub pulls: u32,
    /// Top-rarity hits across the run.
    pub hits: u32,
    /// Longest gap between hits (a pity sanity check).
    pub worst_gap: u32,
}

/// Seeded xorshift so the simulation is reproducible.
struct Rng(u64);

impl Rng {
    /// Next float in `0.0..1.0`.
    ///
    /// The shift above keeps 53 bits, exactly `f64`'s mantissa, so the narrowing
    /// is lossless by construction rather than by luck.
    #[allow(clippy::cast_precision_loss)]
    fn next(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        let scaled = (x >> 11) as f64;
        scaled / (1_u64 << 53) as f64
    }
}

/// Simulate the economy for `spec.days` days.
#[must_use]
pub fn simulate(spec: &EconomySpec, seed: u64) -> EconomyReport {
    let mut rng = Rng(seed | 1);
    let mut balance = 0.0_f64;
    let mut balances: Vec<f64> = Vec::new();
    let mut scarcity_day: Option<u32> = None;
    let mut pulls = 0_u32;
    let mut hits = 0_u32;
    let mut worst_gap = 0_u32;
    let mut since_hit = 0_u32;
    let mut energy = 0.0_f64;

    for day in 1..=spec.days {
        balance += spec.faucet_per_day;
        let spendable = spec.sink_per_day.min(balance);
        balance -= spendable;
        if spendable < spec.sink_per_day && scarcity_day.is_none() {
            scarcity_day = Some(day);
        }
        balances.push(balance);

        energy += spec.energy_per_day;
        let available = if spec.energy_per_pull > 0.0 {
            (energy / spec.energy_per_pull).floor()
        } else {
            0.0
        };
        // Clamped to the plausible range before narrowing; a negative or huge
        // count means the spec is nonsense, and lint reports that separately.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let today = available.clamp(0.0, 1000.0) as u32;
        energy -= f64::from(today) * spec.energy_per_pull;
        for _ in 0..today {
            pulls = pulls.saturating_add(1);
            since_hit = since_hit.saturating_add(1);
            let rate = pity_rate(spec, since_hit);
            if rng.next() < rate {
                hits = hits.saturating_add(1);
                worst_gap = worst_gap.max(since_hit);
                since_hit = 0;
            }
        }
    }
    worst_gap = worst_gap.max(since_hit);
    EconomyReport {
        balances,
        scarcity_day,
        pulls,
        hits,
        worst_gap,
    }
}

/// Rate for the `pull`-th pull since the last hit, with soft/hard pity.
#[must_use]
pub fn pity_rate(spec: &EconomySpec, pull: u32) -> f64 {
    if pull >= spec.hard_pity {
        return 1.0;
    }
    if pull > spec.soft_pity {
        let ramp = f64::from(pull - spec.soft_pity) * 0.06;
        return (spec.base_rate + ramp).min(1.0);
    }
    spec.base_rate
}

/// Lint the economy. Empty result means the design holds.
#[must_use]
pub fn economy_lint(spec: &EconomySpec) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    if spec.faucet_per_day >= spec.sink_per_day {
        findings.push(Finding::new(
            "ERR_FAUCET_DOMINANT",
            format!(
                "faucet {:.0}/day >= sink {:.0}/day: the balance never runs dry, so nothing is scarce",
                spec.faucet_per_day, spec.sink_per_day
            ),
        ));
    }
    let report = simulate(spec, 0x5eed);
    match report.scarcity_day {
        Some(day) if day <= 4 => {}
        Some(day) => findings.push(Finding::new(
            "ERR_SCARCITY_TOO_LATE",
            format!("scarcity only arrives on day {day}; the target is day 4"),
        )),
        None => findings.push(Finding::new(
            "ERR_NO_SCARCITY",
            "the player never runs short; sinks are decorative".to_string(),
        )),
    }
    if spec.premium == spec.soft || spec.soft == spec.energy {
        findings.push(Finding::new(
            "ERR_CURRENCY_NOT_DECOUPLED",
            "two tiers share a name, so the premium/soft/energy split is cosmetic".to_string(),
        ));
    }
    if spec.hard_pity > 90 {
        findings.push(Finding::new(
            "ERR_PITY_TOO_LATE",
            format!(
                "hard pity at {} pulls is past the 90-pull expectation",
                spec.hard_pity
            ),
        ));
    }
    if spec.soft_pity >= spec.hard_pity {
        findings.push(Finding::new(
            "ERR_PITY_INVERTED",
            "soft pity must start before hard pity".to_string(),
        ));
    }
    if !spec.starter_anchor {
        findings.push(Finding::new(
            "ERR_NO_ANCHOR",
            "no $0.99 starter pack: first-purchase friction stays high".to_string(),
        ));
    }
    findings
}

/// Write a Steam-ready asset bundle for `slug`.
///
/// # Errors
///
/// Returns a message when a directory or file cannot be written.
pub fn write_steam_bundle(slug: &str, title: &str, out: &Path) -> Result<Vec<String>, String> {
    std::fs::create_dir_all(out).map_err(|err| err.to_string())?;
    let mut written: Vec<String> = Vec::new();

    // 5 trading cards, 5 emoticons, 3 profile backgrounds.
    for index in 0..5_u32 {
        let name = format!("card_{index}.png");
        write_png(&out.join(&name), &card(index))?;
        written.push(name);
    }
    for index in 0..5_u32 {
        let name = format!("emoticon_{index}.png");
        write_png(&out.join(&name), &emoticon(index))?;
        written.push(name);
    }
    for index in 0..3_u32 {
        let name = format!("background_{index}.png");
        write_png(&out.join(&name), &background(index))?;
        written.push(name);
    }

    let badge = serde_json::json!({
        "badge": format!("{slug}_seal"),
        "title": format!("{title} - Seal of Quality"),
        "levels": [
            {"level": 1, "xp": 50, "description": "Played the first level"},
            {"level": 2, "xp": 250, "description": "Cleared the game"},
            {"level": 3, "xp": 1000, "description": "Cleared without dying"}
        ],
        "cards": 5
    });
    write_json(&out.join("badge.json"), &badge)?;
    written.push("badge.json".to_string());

    let bundle = serde_json::json!({
        "name": format!("{title} Supporter Edition"),
        "includes": [
            {"item": "base_game", "kind": "game"},
            {"item": "soundtrack.wav", "kind": "audio", "note": "WAV; FLAC needs an external encoder"},
            {"item": "wallpaper_0.png", "kind": "image", "resolution": "3840x2160"},
            {"item": "wallpaper_1.png", "kind": "image", "resolution": "3840x2160"},
            {"item": "wallpaper_2.png", "kind": "image", "resolution": "3840x2160"},
            {"item": "skin_exclusive", "kind": "cosmetic"}
        ],
        "price_tier": "supporter"
    });
    write_json(&out.join("supporter_bundle.json"), &bundle)?;
    written.push("supporter_bundle.json".to_string());

    for index in 0..3_u32 {
        let name = format!("wallpaper_{index}.png");
        write_png(&out.join(&name), &wallpaper(index))?;
        written.push(name);
    }

    let schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": format!("{slug} cloud save"),
        "type": "object",
        "required": ["version", "progress"],
        "properties": {
            "version": {"type": "integer", "minimum": 1},
            "progress": {
                "type": "object",
                "required": ["level", "coins"],
                "properties": {
                    "level": {"type": "integer", "minimum": 0},
                    "coins": {"type": "integer", "minimum": 0},
                    "unlocked": {"type": "array", "items": {"type": "string"}}
                }
            },
            "checksum": {"type": "string", "pattern": "^[a-f0-9]{16}$"}
        }
    });
    write_json(&out.join("cloudsave.schema.json"), &schema)?;
    written.push("cloudsave.schema.json".to_string());

    let audio = out.join("soundtrack.wav");
    std::fs::write(&audio, wav_theme()).map_err(|err| err.to_string())?;
    written.push("soundtrack.wav".to_string());

    Ok(written)
}

/// Write an RGBA image as PNG.
fn write_png(path: &Path, image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<(), String> {
    image.save(path).map_err(|err| err.to_string())
}

/// Write pretty JSON.
fn write_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    std::fs::write(path, text).map_err(|err| err.to_string())
}

/// Clamp a computed channel to  ..=255.
///
/// Clamped first, so the narrowing cannot lose information that matters; the
/// rounding step is the only loss and that is the point of a channel value.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_u8(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// Deterministic palette entry per index.
fn hue(index: u32) -> Rgb<u8> {
    const PALETTE: [[u8; 3]; 5] = [
        [206, 92, 84],
        [92, 146, 206],
        [122, 196, 124],
        [214, 178, 92],
        [158, 118, 210],
    ];
    let slot = usize::try_from(index % 5).unwrap_or(0);
    Rgb(PALETTE[slot])
}

/// Trading card: coloured field with a darker frame.
fn card(index: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let base = hue(index);
    ImageBuffer::from_fn(400, 560, |x, y| {
        let edge = !(12..=387).contains(&x) || !(12..=547).contains(&y);
        if edge {
            Rgba([20, 22, 28, 255])
        } else {
            Rgba([base[0], base[1], base[2], 255])
        }
    })
}

/// Emoticon: circle on transparency.
fn emoticon(index: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let base = hue(index);
    ImageBuffer::from_fn(64, 64, |x, y| {
        let dx = f64::from(x) - 31.5;
        let dy = f64::from(y) - 31.5;
        if dx * dx + dy * dy <= 30.0 * 30.0 {
            Rgba([base[0], base[1], base[2], 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    })
}

/// Profile background: vertical gradient in the index's hue.
fn background(index: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let base = hue(index);
    ImageBuffer::from_fn(1920, 620, |_x, y| {
        let t = f64::from(y) / 620.0;
        let fade = |channel: u8| {
            let start = f64::from(channel) * 0.25;
            let end = f64::from(channel);
            let value = start + (end - start) * t;
            to_u8(value)
        };
        Rgba([fade(base[0]), fade(base[1]), fade(base[2]), 255])
    })
}

/// 4K wallpaper: soft diagonal bands.
fn wallpaper(index: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let base = hue(index);
    ImageBuffer::from_fn(3840, 2160, |x, y| {
        let band = (x / 240 + y / 240) % 2 == 0;
        let shade = if band { 1.0 } else { 0.72 };
        let scale = |channel: u8| to_u8(f64::from(channel) * shade);
        Rgba([scale(base[0]), scale(base[1]), scale(base[2]), 255])
    })
}

/// 8-second mono PCM WAV: a short pentatonic arpeggio.
fn wav_theme() -> Vec<u8> {
    const RATE: u32 = 22_050;
    const SECONDS: u32 = 8;
    let notes = [261.63_f64, 329.63, 392.0, 523.25, 392.0, 329.63];
    let per_note = RATE * SECONDS / u32::try_from(notes.len()).unwrap_or(1);
    let mut samples: Vec<i16> = Vec::with_capacity((RATE * SECONDS) as usize);
    for note in notes {
        for step in 0..per_note {
            let t = f64::from(step) / f64::from(RATE);
            let envelope = 1.0 - f64::from(step) / f64::from(per_note);
            let value = (t * note * std::f64::consts::TAU).sin() * envelope * 0.28;
            // Amplitude is bounded at 0.28 before scaling, then clamped, so the
            // narrowing to i16 is a wrap-free conversion.
            #[allow(clippy::cast_possible_truncation)]
            let sample = (value * f64::from(i16::MAX))
                .round()
                .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
            samples.push(sample);
        }
    }
    let data_len = u32::try_from(samples.len() * 2).unwrap_or(0);
    let mut out: Vec<u8> = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2_u16.to_le_bytes());
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_economy_lints_clean() {
        let findings = economy_lint(&EconomySpec::default());
        assert_eq!(findings, Vec::new(), "{findings:?}");
    }

    #[test]
    fn faucet_dominance_is_caught() {
        let spec = EconomySpec {
            faucet_per_day: 500.0,
            ..EconomySpec::default()
        };
        let codes: Vec<String> = economy_lint(&spec).into_iter().map(|f| f.code).collect();
        assert!(
            codes.contains(&"ERR_FAUCET_DOMINANT".to_string()),
            "{codes:?}"
        );
    }

    #[test]
    fn hard_pity_guarantees_an_hit() {
        let spec = EconomySpec::default();
        assert!((pity_rate(&spec, spec.hard_pity) - 1.0).abs() < f64::EPSILON);
        let report = simulate(&spec, 0xabc);
        assert!(report.hits > 0, "no hits in {report:?}");
        assert!(report.worst_gap <= spec.hard_pity, "{report:?}");
    }

    #[test]
    fn bundle_writes_every_asset() {
        let out = std::env::temp_dir().join("gamecraft-steam-test");
        let _ = std::fs::remove_dir_all(&out);
        let written = write_steam_bundle("demo", "Demo Game", out.as_path());
        assert!(written.is_ok(), "{:?}", written.err());
        if let Ok(written) = written {
            assert_eq!(written.len(), 20, "{written:?}");
            for name in &written {
                assert!(out.join(name).is_file(), "{name} missing");
            }
            let wav = std::fs::read(out.join("soundtrack.wav")).unwrap_or_default();
            assert_eq!(&wav[0..4], b"RIFF");
        }
        let _ = std::fs::remove_dir_all(&out);
    }
}
