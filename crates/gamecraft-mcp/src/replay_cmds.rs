//! `replay.*` command bodies: record, verify and inspect `.gcraft` documents.
//!
//! Every path and number here comes from a caller and is treated as hostile: output files
//! must end in `.gcraft`, existing files are not overwritten unless asked, and the replay
//! crate range-checks the document before any simulation runs.

#![allow(clippy::missing_errors_doc)]

use crate::tools::{fail, ToolError, ToolOutcome};
use gamecraft_replay::maze::MazeConfig;
use gamecraft_replay::murmur::FlockSpec;
use gamecraft_replay::{Outcome, Replay, ReplayError, Status};
use serde_json::{json, Value};
use std::fmt::Write as _;
use std::path::Path;

/// Extension every replay document carries.
const EXT: &str = "gcraft";

fn text<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

fn number(args: &Value, key: &str) -> Result<Option<u64>, ToolError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_u64().map(Some).ok_or_else(|| {
            fail(
                "bad_number",
                format!("{key} must be a non-negative whole number."),
                "Pass an integer, not text or a fraction.",
                format!("replay.record {{\"{key}\": 7}}"),
            )
        }),
    }
}

fn gcraft_path(args: &Value) -> Result<&Path, ToolError> {
    let Some(p) = text(args, "path") else {
        return Err(fail(
            "missing_path",
            "path is required.",
            "Give the .gcraft file to read.",
            "replay.verify {\"path\": \"D:/replays/maze.gcraft\"}",
        ));
    };
    let path = Path::new(p);
    if path.extension().and_then(|e| e.to_str()) != Some(EXT) {
        return Err(fail(
            "wrong_extension",
            format!("{p} is not a .{EXT} file."),
            format!("Replay documents end in .{EXT}."),
            "replay.info {\"path\": \"D:/replays/maze.gcraft\"}",
        ));
    }
    Ok(path)
}

fn replay_error(e: &ReplayError) -> ToolError {
    match e {
        ReplayError::Mismatch { field, .. } => fail(
            "replay_mismatch",
            e.to_string(),
            format!("The recorded {field} does not match a fresh simulation: the file was edited, or the game rules changed since it was recorded."),
            "replay.info to read the claimed result, then record it again with replay.record",
        ),
        ReplayError::Invalid(_) | ReplayError::Unsupported(_) => fail(
            "replay_invalid",
            e.to_string(),
            "The document is outside what this build accepts.",
            "replay.info to see its header",
        ),
        ReplayError::Parse(_) => fail(
            "replay_unreadable",
            e.to_string(),
            "The file is not a .gcraft document.",
            "replay.record to make a fresh one",
        ),
        ReplayError::Io(_) => fail(
            "replay_io",
            e.to_string(),
            "Check the path and permissions.",
            "replay.info {\"path\": \"D:/replays/maze.gcraft\"}",
        ),
    }
}

fn status_word(s: Status) -> &'static str {
    match s {
        Status::Won => "won",
        Status::Lost => "lost",
        Status::Incomplete => "incomplete",
    }
}

fn describe(r: &Replay) -> String {
    let o = &r.result;
    format!(
        "{} {} in {} ticks (state {})",
        r.game(),
        status_word(o.status),
        o.ticks,
        o.state_hash
    )
}

/// `replay.record`: run the built-in autoplay or autopilot and keep the inputs.
pub fn replay_record(args: &Value) -> ToolOutcome {
    let game = text(args, "game").unwrap_or("");
    let seed = number(args, "seed")?;
    let replay = match game {
        "maze" => {
            let base = match text(args, "level").unwrap_or("challenge") {
                "depot" => MazeConfig::depot(),
                _ => MazeConfig::challenge(),
            };
            let size = number(args, "size")?;
            let cfg = MazeConfig {
                size: size
                    .and_then(|s| i32::try_from(s).ok())
                    .unwrap_or(base.size),
                seed: seed.unwrap_or(base.seed),
            };
            Replay::record_maze(cfg)
        }
        "murmuration" => {
            let mut spec = FlockSpec::standard();
            if let Some(c) = number(args, "count")? {
                spec.count = usize::try_from(c).unwrap_or(usize::MAX);
            }
            if let Some(s) = seed {
                spec.seed = s;
            }
            let ticks = number(args, "ticks")?
                .map_or(spec.time_limit, |t| u32::try_from(t).unwrap_or(u32::MAX));
            Replay::record_murmuration(spec, ticks)
        }
        other => {
            return Err(fail(
                "unknown_game",
                format!("'{other}' has no replay support."),
                "Replays exist for maze and murmuration.",
                "replay.record {\"game\": \"maze\", \"level\": \"depot\"}",
            ))
        }
    }
    .map_err(|e| replay_error(&e))?;
    let mut data = replay.info();
    let mut summary = format!("Recorded {}.", describe(&replay));
    if let Some(out) = text(args, "out") {
        let path = Path::new(out);
        if path.extension().and_then(|e| e.to_str()) != Some(EXT) {
            return Err(fail(
                "wrong_extension",
                format!("{out} does not end in .{EXT}."),
                format!("Replay documents end in .{EXT}."),
                "replay.record {\"game\": \"maze\", \"out\": \"D:/replays/maze.gcraft\"}",
            ));
        }
        let overwrite = args
            .get("overwrite")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if path.exists() && !overwrite {
            return Err(fail(
                "exists",
                format!("{out} already exists."),
                "Pick another name, or pass overwrite true to replace it.",
                "replay.record {\"game\": \"maze\", \"out\": \"D:/replays/maze2.gcraft\"}",
            ));
        }
        replay.save(path).map_err(|e| replay_error(&e))?;
        let bytes = std::fs::metadata(path).map_or(0, |m| m.len());
        let _ = write!(summary, " Wrote {out} ({bytes} bytes).");
        data["path"] = json!(out);
        data["bytes"] = json!(bytes);
    }
    Ok((summary, data))
}

/// `replay.verify`: re-simulate a `.gcraft` and compare with its recorded result.
pub fn replay_verify(args: &Value) -> ToolOutcome {
    let path = gcraft_path(args)?;
    let replay = Replay::load(path).map_err(|e| replay_error(&e))?;
    let actual: Outcome = replay.verify().map_err(|e| replay_error(&e))?;
    let mut data = replay.info();
    data["verified"] = json!(true);
    data["resimulated"] = json!(actual);
    Ok((
        format!(
            "Verified: {} matches a fresh simulation.",
            describe(&replay)
        ),
        data,
    ))
}

/// `replay.info`: header, config and recorded result, without simulating.
pub fn replay_info(args: &Value) -> ToolOutcome {
    let path = gcraft_path(args)?;
    let replay = Replay::load(path).map_err(|e| replay_error(&e))?;
    Ok((
        format!(
            "{} with {} inputs (not re-simulated).",
            describe(&replay),
            replay.info()["inputs"]
        ),
        replay.info(),
    ))
}
