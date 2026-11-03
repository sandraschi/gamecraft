//! Thin TRELLIS.2 sidecar client (Bucket D wrap, never a bundle).
//!
//! TRELLIS.2 is a 4B-parameter Python/CUDA image-to-3D model
//! (`github.com/microsoft/TRELLIS.2`, research-only licence). This crate
//! carries no weights, no torch and no GPU code: it builds a deterministic
//! *plan* (exact argv or HTTP request plus a licence-ledger row), checks the
//! sidecar is present with [`probe`], and validates an imported GLB against
//! the asset budget with [`check_poly_budget`] and [`check_extent`].
//!
//! Execution lives in the CLI behind an explicit `--execute` flag with a
//! timeout; the MCP tool and dry runs only ever return the plan.

#![allow(clippy::missing_errors_doc)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// TRELLIS.2 upstream repository (pinned by the operator, not by this crate).
pub const TRELLIS_REPO: &str = "https://github.com/microsoft/TRELLIS.2";

/// Licence truth, repeated everywhere a row is written: research-only.
/// See `docs/TRELLIS2.md`.
pub const LICENSE_NOTE: &str =
    "TRELLIS.2 is research-only (non-commercial); human approval required before shipping.";

/// Which sidecar transport to plan for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    /// `python <repo>/run.py --image <png> --outdir <dir>` (operator-owned env).
    Cli,
    /// `POST <endpoint>/generate` with multipart image (operator-owned server).
    Http,
}

/// Operator configuration: environment first, flags override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrellisConfig {
    /// Sidecar transport.
    pub backend: Backend,
    /// TRELLIS.2 checkout for [`Backend::Cli`].
    pub repo_dir: PathBuf,
    /// Python that owns the TRELLIS.2 env for [`Backend::Cli`].
    pub python: PathBuf,
    /// Base URL for [`Backend::Http`], e.g. `http://127.0.0.1:7860`.
    pub endpoint: String,
    /// Where generated GLBs land (`<out>/<slug>.glb`).
    pub out_dir: PathBuf,
}

impl Default for TrellisConfig {
    fn default() -> Self {
        Self {
            backend: Backend::Cli,
            repo_dir: PathBuf::from("D:/Tools/TRELLIS.2"),
            python: PathBuf::from("D:/Tools/TRELLIS.2/.venv/Scripts/python.exe"),
            endpoint: String::new(),
            out_dir: PathBuf::from("assets/trellis"),
        }
    }
}

impl TrellisConfig {
    /// Read `GAMECRAFT_TRELLIS_*` overrides over the defaults.
    #[must_use]
    pub fn from_env() -> Self {
        let base = Self::default();
        let get = |key: &str| std::env::var(key).ok().filter(|v| !v.trim().is_empty());
        Self {
            backend: get("GAMECRAFT_TRELLIS_BACKEND")
                .and_then(|v| {
                    if v.eq_ignore_ascii_case("http") {
                        Some(Backend::Http)
                    } else if v.eq_ignore_ascii_case("cli") {
                        Some(Backend::Cli)
                    } else {
                        None
                    }
                })
                .unwrap_or(base.backend),
            repo_dir: get("GAMECRAFT_TRELLIS_REPO").map_or(base.repo_dir, PathBuf::from),
            python: get("GAMECRAFT_TRELLIS_PYTHON").map_or(base.python, PathBuf::from),
            endpoint: get("GAMECRAFT_TRELLIS_ENDPOINT").unwrap_or(base.endpoint),
            out_dir: get("GAMECRAFT_TRELLIS_OUT").map_or(base.out_dir, PathBuf::from),
        }
    }
}

/// One asset request: an image in, a GLB out. TRELLIS.2 is image-to-3D;
/// `prompt_ref` is ledger context only, never a generation input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrellisRequest {
    /// Ledger slug (`farmhouse-hero`, ASCII, `-`/`_` ok).
    pub slug: String,
    /// Source image the model reconstructs from.
    pub image: PathBuf,
    /// Free-text style reference for the ledger (not an input).
    pub prompt_ref: String,
    /// Triangle budget the import must meet.
    pub poly_budget: u64,
    /// Max world extent in metres the import must meet.
    pub extent_limit_m: f64,
}

/// The deterministic plan: exact transport invocation plus ledger row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrellisPlan {
    /// Exact argv for [`Backend::Cli`], or method+URL for [`Backend::Http`].
    pub argv: Vec<String>,
    /// GLB this plan would write.
    pub out_glb: PathBuf,
    /// Ledger row to record before a human approves the asset.
    pub ledger: TrellisLedger,
}

/// Licence-ledger row, per `docs/LLM_AS_GAME_DESIGNER.md` section 7.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrellisLedger {
    /// Asset slug.
    pub slug: String,
    /// Source image path.
    pub source_image: String,
    /// Style reference (not a generation input).
    pub prompt_ref: String,
    /// Upstream repo this sidecar was built from.
    pub trellis_repo: String,
    /// Always [`LICENSE_NOTE`].
    pub license: String,
    /// Triangle budget for the import gate.
    pub poly_budget: u64,
    /// `pending-human-approval` until a person signs off.
    pub status: String,
}

/// Build the deterministic plan without touching the network or spawning.
///
/// # Errors
///
/// Returns a message when the slug is empty/unsafe, the image path is empty,
/// the budget is zero, or the HTTP backend has no endpoint.
pub fn plan(config: &TrellisConfig, request: &TrellisRequest) -> Result<TrellisPlan, String> {
    if request.slug.trim().is_empty() {
        return Err("trellis request needs a slug".to_string());
    }
    if !request
        .slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "unsafe slug '{}': ASCII letters, digits, '-' and '_' only",
            request.slug
        ));
    }
    if request.image.as_os_str().is_empty() {
        return Err("trellis request needs an image path".to_string());
    }
    if request.poly_budget == 0 {
        return Err("poly_budget must be above zero".to_string());
    }
    let out_glb = config.out_dir.join(format!("{}.glb", request.slug));
    let ledger = TrellisLedger {
        slug: request.slug.clone(),
        source_image: request.image.to_string_lossy().into_owned(),
        prompt_ref: request.prompt_ref.clone(),
        trellis_repo: TRELLIS_REPO.to_string(),
        license: LICENSE_NOTE.to_string(),
        poly_budget: request.poly_budget,
        status: "pending-human-approval".to_string(),
    };
    let argv = match config.backend {
        Backend::Cli => vec![
            config.python.to_string_lossy().into_owned(),
            config
                .repo_dir
                .join("run.py")
                .to_string_lossy()
                .into_owned(),
            "--image".to_string(),
            request.image.to_string_lossy().into_owned(),
            "--outdir".to_string(),
            config.out_dir.to_string_lossy().into_owned(),
        ],
        Backend::Http => {
            if config.endpoint.trim().is_empty() {
                return Err(
                    "HTTP backend needs GAMECRAFT_TRELLIS_ENDPOINT (or --endpoint)".to_string(),
                );
            }
            vec![
                "POST".to_string(),
                format!("{}/generate", config.endpoint.trim_end_matches('/')),
                request.image.to_string_lossy().into_owned(),
            ]
        }
    };
    Ok(TrellisPlan {
        argv,
        out_glb,
        ledger,
    })
}

/// Check the sidecar is present without spawning anything.
///
/// # Errors
///
/// CLI: the repo dir is missing. HTTP: no endpoint is configured.
pub fn probe(config: &TrellisConfig) -> Result<String, String> {
    match config.backend {
        Backend::Cli => {
            if config.repo_dir.is_dir() {
                Ok(format!(
                    "TRELLIS.2 repo at {}",
                    config.repo_dir.to_string_lossy()
                ))
            } else {
                Err(format!(
                    "TRELLIS.2 repo not found at {}; clone {} and set GAMECRAFT_TRELLIS_REPO",
                    config.repo_dir.to_string_lossy(),
                    TRELLIS_REPO
                ))
            }
        }
        Backend::Http => {
            if config.endpoint.trim().is_empty() {
                Err("HTTP backend needs GAMECRAFT_TRELLIS_ENDPOINT".to_string())
            } else {
                Ok(format!("TRELLIS.2 HTTP at {}", config.endpoint))
            }
        }
    }
}

/// Triangle-budget gate for the imported GLB.
///
/// # Errors
///
/// Returns a message when the mesh exceeds the budget.
pub fn check_poly_budget(triangles: u64, budget: u64) -> Result<(), String> {
    if triangles > budget {
        return Err(format!(
            "asset has {triangles} triangles, budget is {budget}: decimate or raise the budget"
        ));
    }
    Ok(())
}

/// World-extent gate: hero props must fit the scene scale.
///
/// # Errors
///
/// Returns a message when the longest AABB edge exceeds the limit.
pub fn check_extent(max_edge_m: f64, limit_m: f64) -> Result<(), String> {
    if max_edge_m > limit_m {
        return Err(format!(
            "asset extent {max_edge_m:.2} m exceeds limit {limit_m:.2} m: rescale on import"
        ));
    }
    Ok(())
}

/// Ledger completeness gate: every field a human needs to approve.
///
/// # Errors
///
/// Returns a message naming the first missing field.
pub fn ledger_complete(ledger: &TrellisLedger) -> Result<(), String> {
    if ledger.slug.trim().is_empty() {
        return Err("ledger needs a slug".to_string());
    }
    if ledger.source_image.trim().is_empty() {
        return Err("ledger needs the source image".to_string());
    }
    if ledger.trellis_repo.trim().is_empty() {
        return Err("ledger needs the upstream repo".to_string());
    }
    if ledger.license.trim().is_empty() {
        return Err("ledger needs the licence note".to_string());
    }
    Ok(())
}

/// Write one ledger row as JSON next to the GLB (`.ledger.json`).
///
/// # Errors
///
/// Returns a message when the parent cannot be created or the file written.
pub fn write_ledger(out_glb: &Path, ledger: &TrellisLedger) -> Result<PathBuf, String> {
    let path = out_glb.with_extension("ledger.json");
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
    }
    let text = serde_json::to_string_pretty(ledger).map_err(|err| err.to_string())?;
    std::fs::write(&path, text).map_err(|err| err.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> TrellisRequest {
        TrellisRequest {
            slug: "farmhouse-hero".to_string(),
            image: PathBuf::from("shots/farmhouse.png"),
            prompt_ref: "cream walls, red pitched roof".to_string(),
            poly_budget: 20_000,
            extent_limit_m: 8.0,
        }
    }

    #[test]
    fn cli_plan_is_deterministic() {
        let config = TrellisConfig::default();
        let first = plan(&config, &request()).unwrap_or_else(|_| unreachable!("fixture is valid"));
        let second = plan(&config, &request()).unwrap_or_else(|_| unreachable!("fixture is valid"));
        assert_eq!(first.argv, second.argv);
        assert_eq!(first.out_glb, second.out_glb);
        assert_eq!(first.ledger.license, LICENSE_NOTE);
        assert_eq!(first.ledger.status, "pending-human-approval");
        assert!(first.argv.iter().any(|a| a == "--image"));
    }

    #[test]
    fn http_plan_needs_endpoint() {
        let config = TrellisConfig {
            backend: Backend::Http,
            ..TrellisConfig::default()
        };
        assert!(plan(&config, &request()).is_err());
        let config = TrellisConfig {
            backend: Backend::Http,
            endpoint: "http://127.0.0.1:7860".to_string(),
            ..TrellisConfig::default()
        };
        let p = plan(&config, &request()).unwrap_or_else(|_| unreachable!("endpoint set"));
        assert_eq!(p.argv[0], "POST");
    }

    #[test]
    fn rejects_unsafe_slugs_and_empty_inputs() {
        let config = TrellisConfig::default();
        let mut bad = request();
        bad.slug = "../evil".to_string();
        assert!(plan(&config, &bad).is_err());
        let mut bad = request();
        bad.poly_budget = 0;
        assert!(plan(&config, &bad).is_err());
    }

    #[test]
    fn budget_and_extent_gates() {
        assert!(check_poly_budget(20_000, 20_000).is_ok());
        assert!(check_poly_budget(20_001, 20_000).is_err());
        assert!(check_extent(8.0, 8.0).is_ok());
        assert!(check_extent(8.1, 8.0).is_err());
    }

    #[test]
    fn probe_reports_missing_repo() {
        let config = TrellisConfig {
            repo_dir: PathBuf::from("D:/no/such/trellis-checkout"),
            ..TrellisConfig::default()
        };
        assert!(probe(&config).is_err());
    }
}
