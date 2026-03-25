/// LLM configuration for prose-to-diagram conversion.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: String::new(),
            model: "gpt-4o".to_string(),
        }
    }
}

impl LlmConfig {
    /// Create a config pre-set for the Anthropic Messages API.
    pub fn anthropic(api_key: String, model: Option<String>) -> Self {
        Self {
            endpoint: "https://api.anthropic.com/v1/messages".to_string(),
            api_key,
            model: model.unwrap_or_else(|| "claude-opus-4-5".to_string()),
        }
    }

    /// True when the endpoint targets the Anthropic Messages API.
    ///
    /// Detection is URL-based: any endpoint containing "anthropic.com" is treated
    /// as the Anthropic Messages API (different request/response shape from OpenAI).
    /// A local proxy or mirror at a non-anthropic.com URL will be treated as
    /// OpenAI-compatible — use the OpenAI-compatible format in that case.
    pub fn is_anthropic(&self) -> bool {
        self.endpoint.contains("anthropic.com")
    }
}

// ── curl config helper ──────────────────────────────────────────────────────

/// Write curl auth headers to a temp file to avoid API key exposure in `ps`.
/// Returns the file path. Caller must delete it when done.
fn write_curl_config(config: &LlmConfig) -> Result<std::path::PathBuf, String> {
    use std::io::Write;
    let tmp = std::env::temp_dir()
        .join(format!("lf_curl_{}.cfg", uuid::Uuid::new_v4()));
    // Open with 0o600 at creation time (no TOCTOU window).
    #[cfg(unix)]
    let mut f = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(|e| format!("Failed to create curl config: {}", e))?
    };
    #[cfg(not(unix))]
    let mut f = std::fs::File::create(&tmp)
        .map_err(|e| format!("Failed to create curl config: {}", e))?;
    if config.is_anthropic() {
        writeln!(f, "header = \"x-api-key: {}\"", config.api_key)
            .map_err(|e| format!("Failed to write curl config: {}", e))?;
        writeln!(f, "header = \"anthropic-version: 2023-06-01\"")
            .map_err(|e| format!("Failed to write curl config: {}", e))?;
    } else {
        writeln!(f, "header = \"Authorization: Bearer {}\"", config.api_key)
            .map_err(|e| format!("Failed to write curl config: {}", e))?;
    }
    Ok(tmp)
}

/// Truncate `s` to at most `max_chars` Unicode scalar values for display.
/// Unlike `&s[..n]`, this never panics on a multi-byte UTF-8 boundary.
fn truncate_for_display(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

// ── SpecGraph YAML (OpenAI-compatible) ──────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"You are a diagram specification converter. Convert the user's natural language description into SpecGraph YAML format.

Output ONLY valid YAML, no markdown fences, no explanation.

Format:
```
specgraph: "1.0"
title: <inferred title>
mode: flowchart

nodes:
  - id: n1
    kind: shape
    shape: rounded_rect
    label: "Node label"

  - id: n2
    kind: shape
    shape: diamond
    label: "Decision?"

edges:
  - from: { node: n1, side: bottom }
    to: { node: n2, side: top }
    label: "connection label"
```

Node shapes: rectangle, rounded_rect, diamond, circle, parallelogram
Node kinds: shape, entity, sticky, text
Port sides: top, bottom, left, right
Sticky colors: yellow, pink, green, blue, purple

For entity nodes:
  - id: e1
    kind: entity
    name: "TableName"
    attributes:
      - { name: "id", pk: true, fk: false }
      - { name: "email", pk: false, fk: false }

Infer the best diagram structure from the description. Use meaningful node IDs.
Do NOT include position or size fields — they will be auto-laid out.
Do NOT wrap output in markdown code fences."#;

/// Convert prose text to SpecGraph YAML using any LLM — Anthropic or OpenAI-compatible.
pub fn prose_to_yaml(prose: &str, config: &LlmConfig) -> Result<String, String> {
    if config.api_key.is_empty() {
        return Err("No API key configured. Go to LLM Settings to set your API key.".to_string());
    }

    let body = if config.is_anthropic() {
        serde_json::json!({
            "model": config.model,
            "max_tokens": 2048,
            "system": SYSTEM_PROMPT,
            "messages": [{"role": "user", "content": prose}]
        })
    } else {
        serde_json::json!({
            "model": config.model,
            "temperature": 0.2,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": prose}
            ]
        })
    };
    let body_str = serde_json::to_string(&body)
        .map_err(|e| format!("JSON serialize error: {}", e))?;

    let cfg_path = write_curl_config(config)?;
    let cfg_str = cfg_path.to_str()
        .ok_or_else(|| "Temp config path contains non-UTF-8 characters".to_string())?;
    let output = std::process::Command::new("curl")
        .args([
            "-s", "-X", "POST",
            &config.endpoint,
            "--config", cfg_str,
            "-H", "Content-Type: application/json",
            "-d", &body_str,
        ])
        .output();
    let _ = std::fs::remove_file(&cfg_path);

    let output = output.map_err(|e| format!("Failed to call LLM API: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("LLM API request failed: {}", stderr));
    }

    let response_str = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| format!(
            "Failed to parse LLM response: {} — raw: {}",
            e,
            truncate_for_display(&response_str, 200)
        ))?;

    if let Some(err) = response["error"]["message"].as_str() {
        return Err(format!("LLM API error: {}", err));
    }

    let content = if config.is_anthropic() {
        response["content"][0]["text"].as_str()
    } else {
        response["choices"][0]["message"]["content"].as_str()
    };
    let content = content.ok_or_else(|| {
        format!(
            "Unexpected LLM response format: {}",
            truncate_for_display(&response_str, 300)
        )
    })?;

    // Strip markdown fences if the LLM included them
    let yaml = content
        .trim()
        .strip_prefix("```yaml")
        .or_else(|| content.trim().strip_prefix("```"))
        .unwrap_or(content.trim());
    let yaml = yaml.strip_suffix("```").unwrap_or(yaml).trim();

    Ok(yaml.to_string())
}

// ── HRF (Anthropic or OpenAI-compatible) ────────────────────────────────────

const HRF_SYSTEM_PROMPT: &str = r#"You are a diagram generator. Convert the user's description into HRF (Human-Readable Format) for the openDraftly diagramming tool.

Output ONLY valid HRF text. No markdown fences. No explanation.

HRF format example:
## Config
flow = LR

## Nodes
- [api] REST API {shape:cylinder}
- [db] Database {shape:cylinder} {done}
- [ui] Frontend {wip}

## Flow
ui --> api: requests
api --> db: queries

For roadmaps use ## Timeline sections with {phase:Q1} {milestone} tags.
For GTM use ## Swimlane: Name sections with {metric:N} tags.
Use {done} {wip} {todo} for status. Use {owner:@name} for ownership."#;

/// Convert prose to HRF using any LLM — Anthropic or OpenAI-compatible.
///
/// Routing:
/// - `config.is_anthropic()` → Anthropic Messages API format
/// - otherwise              → OpenAI chat-completions format
///
/// The API key is never passed as a command-line argument; it is written to a
/// temp file and referenced via `curl --config` to prevent `ps` exposure.
pub fn prose_to_hrf(prose: &str, template: &str, config: &LlmConfig) -> Result<String, String> {
    if config.api_key.is_empty() {
        return Err(
            "No API key. Set ANTHROPIC_API_KEY env var or use --api-key flag.".to_string()
        );
    }

    let system = if template.is_empty() {
        HRF_SYSTEM_PROMPT.to_string()
    } else {
        format!("{}\n\nTemplate hint: {}", HRF_SYSTEM_PROMPT, template)
    };

    let body = if config.is_anthropic() {
        serde_json::json!({
            "model": config.model,
            "max_tokens": 2048,
            "system": system,
            "messages": [{"role": "user", "content": prose}]
        })
    } else {
        serde_json::json!({
            "model": config.model,
            "temperature": 0.2,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": prose}
            ]
        })
    };

    let body_str = serde_json::to_string(&body)
        .map_err(|e| format!("JSON serialize error: {}", e))?;

    let cfg_path = write_curl_config(config)?;
    let cfg_str = cfg_path.to_str()
        .ok_or_else(|| "Temp config path contains non-UTF-8 characters".to_string())?;
    let output = std::process::Command::new("curl")
        .args([
            "-s", "-X", "POST",
            &config.endpoint,
            "--config", cfg_str,
            "-H", "Content-Type: application/json",
            "-d", &body_str,
        ])
        .output();
    let _ = std::fs::remove_file(&cfg_path);

    let output = output.map_err(|e| format!("Failed to call LLM API: {}", e))?;
    if !output.status.success() {
        return Err(format!(
            "API request failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| format!("Parse error: {}", e))?;

    if let Some(err) = response["error"]["message"].as_str() {
        return Err(format!("API error: {}", err));
    }

    let content = if config.is_anthropic() {
        response["content"][0]["text"].as_str()
    } else {
        response["choices"][0]["message"]["content"].as_str()
    };

    content
        .map(|s| s.trim().to_string())
        .ok_or_else(|| format!("Unexpected API response: {}", truncate_for_display(&raw, 200)))
}
