//! Scene input resolution: exactly one of `scene_text` or `scene_file`.
//!
//! A stdio MCP server has no project root, so `res://` only means something when
//! the caller supplies `root`. Every resolved path must stay inside that root -
//! a tool call must never be able to read an arbitrary file by asking nicely.

use std::path::PathBuf;

/// Resolve the scene source for one tool call.
///
/// # Errors
///
/// Returns a message when both or neither input is supplied, when `res://` is
/// used without `root`, when the file is unreadable, or when the resolved path
/// escapes `root`.
pub fn resolve_scene_text(
    scene_text: Option<&str>,
    scene_file: Option<&str>,
    root: Option<&str>,
) -> Result<String, String> {
    match (scene_text, scene_file) {
        (Some(_), Some(_)) => Err("pass either scene_text or scene_file, not both".to_string()),
        (None, None) => Err("missing scene input: pass scene_text or scene_file".to_string()),
        (Some(text), None) => Ok(text.to_string()),
        (None, Some(file)) => read_scene_file(file, root),
    }
}

/// Read a scene file, enforcing `root` containment when a root is involved.
fn read_scene_file(file: &str, root: Option<&str>) -> Result<String, String> {
    let is_res = file.starts_with("res://");
    let relative = file.strip_prefix("res://").unwrap_or(file);

    let (path, containment) = if is_res {
        let Some(root) = root else {
            return Err(
                "scene_file uses res:// - pass root (the absolute project directory)".to_string(),
            );
        };
        let root_path = canonical_root(root)?;
        (root_path.join(relative), Some(root_path))
    } else {
        let path = PathBuf::from(relative);
        (path, root.map(canonical_root).transpose()?)
    };

    let resolved = std::fs::canonicalize(&path)
        .map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    if let Some(root_path) = containment {
        if !resolved.starts_with(&root_path) {
            return Err(format!(
                "{} resolves outside root {}",
                resolved.display(),
                root_path.display()
            ));
        }
    }
    std::fs::read_to_string(&resolved)
        .map_err(|err| format!("cannot read {}: {err}", resolved.display()))
}

/// Canonicalise a root directory, with a message naming the argument.
fn canonical_root(root: &str) -> Result<PathBuf, String> {
    std::fs::canonicalize(root).map_err(|err| format!("root {root} is not accessible: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_inputs_are_rejected() {
        let outcome = resolve_scene_text(Some("x"), Some("y"), None);
        assert!(outcome.is_err());
    }

    #[test]
    fn neither_input_is_rejected() {
        let outcome = resolve_scene_text(None, None, None);
        assert!(outcome.is_err());
    }

    #[test]
    fn text_passes_through() {
        let outcome = resolve_scene_text(Some("[gd_scene]"), None, None);
        assert_eq!(outcome, Ok("[gd_scene]".to_string()));
    }

    #[test]
    fn res_uri_requires_root() {
        let outcome = resolve_scene_text(None, Some("res://a.tscn"), None);
        assert!(outcome.is_err());
        if let Err(reason) = outcome {
            assert!(reason.contains("root"));
        }
    }

    #[test]
    fn res_uri_outside_root_is_rejected() {
        let outcome = resolve_scene_text(None, Some("res://../../etc/passwd"), Some("."));
        assert!(outcome.is_err() || outcome.is_ok_and(|text| !text.contains("root:")));
    }
}
