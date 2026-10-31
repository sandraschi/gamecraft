//! Minimal LLM client for the "make a new game" path.
//!
//! Three providers, one contract: send a prompt, get text back.
//! - `local`     -> Ollama (default `http://127.0.0.1:11434`)
//! - openai    -> any OpenAI-compatible endpoint (hosted API, LM Studio, vLLM)
//! - `anthropic` -> Anthropic messages API
//!
//! `curl.exe` is used instead of an HTTP crate on purpose: it ships with Windows,
//! it is invoked with an argument array (so no shell quoting), and it keeps the
//! dependency surface of the forge at zero. The tradeoff is one process spawn per
//! request, which is irrelevant for a handful of generations.

use serde::Deserialize;

/// Which API to speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// Ollama chat API.
    Ollama,
    /// OpenAI-compatible chat completions.
    OpenAi,
    /// Anthropic messages.
    Anthropic,
    /// `DeepSeek` chat completions (OpenAI-compatible wire format).
    DeepSeek,
    /// Meta Llama API (OpenAI-compatible wire format).
    Meta,
}

/// Connection settings.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Which API.
    pub kind: ProviderKind,
    /// Model id.
    pub model: String,
    /// Base URL (no trailing slash needed).
    pub endpoint: String,
    /// API key for cloud providers.
    pub api_key: Option<String>,
}

impl LlmConfig {
    /// Build a config from a provider name, with sane defaults.
    ///
    /// # Errors
    ///
    /// Returns a message for an unknown provider name.
    pub fn new(provider: &str, model: Option<&str>) -> Result<Self, String> {
        match provider {
            "local" | "ollama" => Ok(Self {
                kind: ProviderKind::Ollama,
                model: model.unwrap_or("gemma4:12b").to_string(),
                endpoint: "http://127.0.0.1:11434".to_string(),
                api_key: None,
            }),
            "openai" | "cloud" => Ok(Self {
                kind: ProviderKind::OpenAi,
                model: model.unwrap_or("gpt-4o-mini").to_string(),
                endpoint: "https://api.openai.com".to_string(),
                api_key: gamecraft_core::config::AppConfig::load().openai_key(),
            }),
            "anthropic" => Ok(Self {
                kind: ProviderKind::Anthropic,
                model: model.unwrap_or("claude-sonnet-4-5").to_string(),
                endpoint: "https://api.anthropic.com".to_string(),
                api_key: gamecraft_core::config::AppConfig::load().anthropic_key(),
            }),
            "deepseek" => Ok(Self {
                kind: ProviderKind::DeepSeek,
                model: model.unwrap_or("deepseek-chat").to_string(),
                endpoint: "https://api.deepseek.com".to_string(),
                api_key: gamecraft_core::config::AppConfig::load().deepseek_key(),
            }),
            "meta" => Ok(Self {
                kind: ProviderKind::Meta,
                model: model
                    .unwrap_or("Llama-4-Maverick-17B-128E-Instruct-FP8")
                    .to_string(),
                endpoint: "https://api.llama.com/compat/v1".to_string(),
                api_key: gamecraft_core::config::AppConfig::load().meta_key(),
            }),
            other => Err(format!(
                "unknown provider '{other}' (local, openai, anthropic, deepseek, meta)"
            )),
        }
    }

    /// Same config pointed at a different base URL.
    #[must_use]
    pub fn at(mut self, endpoint: &str) -> Self {
        self.endpoint = endpoint.to_string();
        self
    }

    /// Same config with an explicit key.
    #[must_use]
    pub fn with_key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_string());
        self
    }
}

/// Response shapes, one per provider. Kept separate so a shape change in one
/// provider cannot silently break another.
#[derive(Deserialize)]
struct OllamaReply {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiReply {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct AnthropicReply {
    content: Vec<AnthropicBlock>,
}

#[derive(Deserialize)]
struct AnthropicBlock {
    text: Option<String>,
}

/// POST a JSON body and return the response text.
fn post_json(url: &str, headers: &[(String, String)], body: &str) -> Result<String, String> {
    let mut command = std::process::Command::new("curl.exe");
    command
        .arg("-sS")
        .arg("-X")
        .arg("POST")
        .arg(url)
        .arg("-H")
        .arg("content-type: application/json")
        .arg("--max-time")
        .arg("240")
        .arg("--data-binary")
        .arg(body);
    for (name, value) in headers {
        command.arg("-H").arg(format!("{name}: {value}"));
    }
    let output = command
        .output()
        .map_err(|err| format!("cannot run curl.exe: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("request failed: {}", stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// GET a URL with curl and return the body, or `None` if it fails.
fn get_json(url: &str) -> Option<String> {
    let output = std::process::Command::new("curl.exe")
        .arg("-sS")
        .arg("--max-time")
        .arg("3")
        .arg(url)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Pick a chat model, skipping embedding-only models.
///
/// Prefers names that suggest instruction/JSON competence ("tools", "qwen",
/// "instruct") because a small base model is far more likely to answer with
/// prose that fails the spec schema. Falls back to the first chat model listed.
fn pick_model(names: &[String]) -> Option<String> {
    let mut best: Option<(u8, String)> = None;
    for name in names {
        let lower = name.to_lowercase();
        if lower.contains("embed") {
            continue;
        }
        // Pinned to the model measured working on this machine. qwen3.8:27b
        // resets the Ollama socket and returns nothing in 420 s, so it must never
        // be auto-selected; cloud models take over once keys are provisioned.
        if lower.contains("qwen3.8") {
            continue;
        }
        let score = if lower.contains("gemma4") {
            5
        } else if lower.contains("tools") {
            3
        } else if lower.contains("qwen") || lower.contains("instruct") {
            2
        } else {
            1
        };
        let better = best.as_ref().is_none_or(|(current, _)| score > *current);
        if better {
            best = Some((score, name.clone()));
        }
    }
    best.map(|(_, name)| name)
}

/// Parse `"27.8B"` / `"12B"` / `"980M"` into billions of parameters.
fn parameter_billions(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    if let Some(mb) = trimmed.strip_suffix('M') {
        return mb.parse::<f32>().ok().map(|value| value / 1000.0);
    }
    let billions = trimmed.strip_suffix('B').unwrap_or(trimmed);
    billions.parse::<f32>().ok()
}

/// Names advertised by an Ollama server, ordered smallest-first.
///
/// Size is read so `auto` can pick a model that will actually load: the 27B on
/// this machine crashes the server outright (connection reset, zero bytes), and a
/// model that cannot answer is worse than a small one that can.
fn ollama_models(base: &str) -> Option<Vec<String>> {
    let text = get_json(format!("{base}/api/tags").as_str())?;
    let value: serde_json::Value = serde_json::from_str(text.as_str()).ok()?;
    let models = value.get("models")?.as_array()?;
    let mut listed: Vec<(f32, String)> = models
        .iter()
        .filter_map(|model| {
            let name = model.get("name")?.as_str()?.to_string();
            let size = model
                .get("details")
                .and_then(|details| details.get("parameter_size"))
                .and_then(|size| size.as_str())
                .and_then(parameter_billions)
                .unwrap_or(0.0);
            Some((size, name))
        })
        .collect();
    listed.sort_by(|left, right| left.0.total_cmp(&right.0));
    Some(listed.into_iter().map(|(_, name)| name).collect())
}

/// Names advertised by an OpenAI-compatible server.
fn openai_models(base: &str) -> Option<Vec<String>> {
    let text = get_json(format!("{base}/v1/models").as_str())?;
    let value: serde_json::Value = serde_json::from_str(text.as_str()).ok()?;
    let models = value.get("data")?.as_array()?;
    Some(
        models
            .iter()
            .filter_map(|model| model.get("id")?.as_str().map(str::to_string))
            .collect(),
    )
}

/// Find whichever local server is actually running, and use it.
///
/// Ollama on 11434 and LM Studio on 1234 are probed in that order. Any other
/// OpenAI-compatible server is reached with `--endpoint`. This exists because
/// "Ollama is down again" is a recurring state, not an exception.
///
/// # Errors
///
/// Returns a message naming every endpoint probed when none answered, or naming
/// the available models when a requested model is not loaded.
pub fn auto(model: Option<&str>) -> Result<LlmConfig, String> {
    // A pinned model in the config file counts as an explicit request, so every
    // caller (CLI, viewer, MCP) honours it without re-resolving flags.
    let file_config = gamecraft_core::config::AppConfig::load();
    let model = model.or(file_config.effective_model());
    let probes = [
        (ProviderKind::Ollama, "http://127.0.0.1:11434"),
        (ProviderKind::OpenAi, "http://127.0.0.1:1234"),
    ];
    let mut tried: Vec<String> = Vec::new();
    for (kind, base) in probes {
        let names = if kind == ProviderKind::Ollama {
            ollama_models(base)
        } else {
            openai_models(base)
        };
        let Some(names) = names else {
            tried.push(format!("{base} (no answer)"));
            continue;
        };
        let chosen = match model {
            Some(requested) if names.iter().any(|name| name == requested) => requested.to_string(),
            Some(requested) => {
                return Err(format!(
                    "{requested} is not loaded on {base}; available: {}",
                    names.join(", ")
                ));
            }
            None => {
                let Some(picked) = pick_model(&names) else {
                    tried.push(format!("{base} (no chat model)"));
                    continue;
                };
                picked
            }
        };
        return Ok(LlmConfig {
            kind,
            model: chosen,
            endpoint: base.to_string(),
            api_key: Some("local".to_string()),
        });
    }
    Err(format!(
        "no local model server answered. Tried: {}. Start Ollama or LM Studio, or pass --provider openai --endpoint <url> --model <id>.",
        tried.join(", ")
    ))
}

/// Send one prompt and return the model's text.
///
/// # Errors
///
/// Returns a message when the request fails, the reply is not JSON, or the reply
/// carries no content.
pub fn complete(config: &LlmConfig, prompt: &str) -> Result<String, String> {
    match config.kind {
        ProviderKind::Ollama => {
            let body = serde_json::json!({
                "model": config.model,
                "stream": false,
                "messages": [{"role": "user", "content": prompt}],
                // An explicit budget: a truncated reply is the most common failure
                // (observed: JSON cut at 487 chars), and temperature 0.1 keeps the
                // shape stable between repair attempts.
                "format": "json",
                // Thinking models spend num_predict on hidden reasoning first, and
                // an empty `content` is indistinguishable from "model refused".
                // Measured: gemma4:12b truncated at 487 chars, then returned nothing.
                "think": false,
                "options": {"num_predict": 2500, "temperature": 0.1},
            })
            .to_string();
            let text = post_json(
                format!("{}/api/chat", config.endpoint.trim_end_matches('/')).as_str(),
                &[],
                body.as_str(),
            )?;
            let reply: OllamaReply = serde_json::from_str(text.as_str())
                .map_err(|err| format!("unexpected ollama reply: {err}: {text}"))?;
            Ok(reply.message.content)
        }
        ProviderKind::OpenAi | ProviderKind::DeepSeek | ProviderKind::Meta => {
            let want_key = match config.kind {
                ProviderKind::DeepSeek => "DEEPSEEK_API_KEY",
                ProviderKind::Meta => "META_API_KEY",
                _ => "OPENAI_API_KEY",
            };
            let key = config
                .api_key
                .as_ref()
                .ok_or_else(|| format!("{want_key} is not set"))?;
            let body = serde_json::json!({
                "model": config.model,
                "messages": [{"role": "user", "content": prompt}],
                // Without a cap, LM Studio generates until the request is
                // terminated and answers {"error":"terminated"} instead of choices.
                "max_tokens": 2048,
                "temperature": 0.2,
                // Ask the server to constrain decoding to JSON. Supported by
                // OpenAI, LM Studio and most compatible servers; a server that
                // ignores it still gets the plain prompt.
                "response_format": {"type": "json_object"},
            })
            .to_string();
            let text = post_json(
                format!(
                    "{}/v1/chat/completions",
                    config.endpoint.trim_end_matches('/')
                )
                .as_str(),
                &[("authorization".to_string(), format!("Bearer {key}"))],
                body.as_str(),
            )?;
            let reply: OpenAiReply = serde_json::from_str(text.as_str())
                .map_err(|err| format!("unexpected chat reply: {err}: {text}"))?;
            reply
                .choices
                .first()
                .map(|choice| choice.message.content.clone())
                .ok_or_else(|| "chat reply had no choices".to_string())
        }
        ProviderKind::Anthropic => {
            let key = config
                .api_key
                .as_ref()
                .ok_or_else(|| "ANTHROPIC_API_KEY is not set".to_string())?;
            let body = serde_json::json!({
                "model": config.model,
                "max_tokens": 4096,
                "messages": [{"role": "user", "content": prompt}],
            })
            .to_string();
            let text = post_json(
                format!("{}/v1/messages", config.endpoint.trim_end_matches('/')).as_str(),
                &[
                    ("x-api-key".to_string(), key.clone()),
                    ("anthropic-version".to_string(), "2023-06-01".to_string()),
                ],
                body.as_str(),
            )?;
            let reply: AnthropicReply = serde_json::from_str(text.as_str())
                .map_err(|err| format!("unexpected anthropic reply: {err}: {text}"))?;
            reply
                .content
                .iter()
                .find_map(|block| block.text.clone())
                .ok_or_else(|| "anthropic reply had no text block".to_string())
        }
    }
}

/// Pull the first JSON object out of a reply that may be wrapped in fences or prose.
#[must_use]
pub fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(&text[start..=end])
}

/// The instruction the model receives.
#[must_use]
pub fn spec_prompt(idea: &str) -> String {
    format!(
        "You design levels for a 2.5D platformer engine. Reply with ONE JSON object and nothing else - no prose, no markdown fences.\n\
Schema (all numbers are floats):\n\
{{\"slug\":\"a-z0-9-\",\"title\":\"Display Name\",\"archetype\":\"platformer\",\n\
\"theme\":{{\"sky_top\":[r,g,b],\"sky_horizon\":[r,g,b],\"ground\":[r,g,b],\"platform\":[r,g,b],\"player\":[r,g,b],\"coin\":[r,g,b]}},\n\
\"physics\":{{\"speed\":4.5,\"jump_velocity\":10.5,\"gravity\":24.0}},\n\
\"level\":{{\"grounds\":[{{\"x\":-6,\"w\":12}},{{\"x\":9,\"w\":12}}],\n\
\"platforms\":[{{\"x\":2,\"y\":1.0,\"w\":2.5}}],\n\
\"coins\":[[2.0,2.2]],\"goal_x\":18.0}}}}\n\
Other archetypes (reply with one of these shapes instead of the platformer one):\n\
- runner: {{\"slug\":\"..\",\"title\":\"..\",\"archetype\":\"runner\"}} (fixed auto-runner; theme applies)\n\
- pong: rolling ball game; send \"physics\":{{\"speed\":14.0}} as ball speed, 8..22\n\
- maze: {{\"slug\":\"..\",\"title\":\"..\",\"archetype\":\"maze\",\"maze\":{{\"size\":15,\"seed\":9}}}}; size odd 7..25\n\
Rules that decide whether the level is accepted:\n\
- The player starts on the first ground slab and runs right.\n\
- Gaps between grounds must be jumpable: width <= 3.2 units with the default jump.\n\
- goal_x must sit on the last ground slab.\n\
- Colours are 0..1 floats. Keep 1 to 3 platforms and 2 to 5 coins.\n\
Theme requested by the user: {idea}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_extracted_from_fences() {
        let text = "Sure!\n```json\n{\"a\": 1}\n```\n";
        assert_eq!(extract_json(text), Some("{\"a\": 1}"));
    }

    #[test]
    fn unknown_provider_is_rejected() {
        assert!(LlmConfig::new("banana", None).is_err());
    }

    #[test]
    fn cloud_defaults_point_at_their_apis() {
        for (name, host) in [("deepseek", "api.deepseek.com"), ("meta", "api.llama.com")] {
            let config = LlmConfig::new(name, None);
            assert!(config.is_ok(), "{name} rejected");
            if let Ok(config) = config {
                assert!(
                    config.endpoint.contains(host),
                    "{name}: {}",
                    config.endpoint
                );
                assert!(config.api_key.is_none(), "{name} leaked a key");
            }
        }
    }

    #[test]
    fn local_defaults_to_ollama() {
        let config = LlmConfig::new("local", None);
        assert!(config.is_ok());
        if let Ok(config) = config {
            assert_eq!(config.kind, ProviderKind::Ollama);
            assert!(config.endpoint.contains("11434"));
        }
    }
}
