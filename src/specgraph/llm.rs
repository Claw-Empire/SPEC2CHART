/// LLM configuration for prose-to-YAML conversion.
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

/// Convert prose text to SpecGraph YAML using an LLM API (blocking HTTP).
pub fn prose_to_yaml(prose: &str, config: &LlmConfig) -> Result<String, String> {
    if config.api_key.is_empty() {
        return Err("No API key configured. Go to LLM Settings to set your API key.".to_string());
    }

    let body = serde_json::json!({
        "model": config.model,
        "temperature": 0.2,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": prose}
        ]
    });

    // Use ureq-style blocking HTTP via std (minimal dependency)
    // We'll use a raw TCP+TLS approach via the system curl command
    // to avoid adding reqwest/tokio as dependencies
    let body_str = serde_json::to_string(&body)
        .map_err(|e| format!("JSON serialize error: {}", e))?;

    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X", "POST",
            &config.endpoint,
            "-H", "Content-Type: application/json",
            "-H", &format!("Authorization: Bearer {}", config.api_key),
            "-d", &body_str,
        ])
        .output()
        .map_err(|e| format!("Failed to call LLM API: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("LLM API request failed: {}", stderr));
    }

    let response_str = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| format!("Failed to parse LLM response: {} — raw: {}", e, &response_str[..200.min(response_str.len())]))?;

    // Extract content from OpenAI-compatible response
    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| {
            // Check for error response
            if let Some(err) = response["error"]["message"].as_str() {
                format!("LLM API error: {}", err)
            } else {
                format!("Unexpected LLM response format: {}", &response_str[..300.min(response_str.len())])
            }
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

const HRF_SYSTEM_PROMPT: &str = r#"You are a diagram generator. Convert the user's description into HRF (Human-Readable Format) for the light-figma diagramming tool.

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

/// Convert prose to HRF using Anthropic API (blocking via curl).
pub fn prose_to_hrf(prose: &str, template: &str, api_key: &str) -> Result<String, String> {
    let system = if template.is_empty() {
        HRF_SYSTEM_PROMPT.to_string()
    } else {
        format!("{}\n\nTemplate hint: {}", HRF_SYSTEM_PROMPT, template)
    };
    let body = serde_json::json!({
        "model": "claude-opus-4-5",
        "max_tokens": 2048,
        "system": system,
        "messages": [{"role": "user", "content": prose}]
    });
    let body_str = serde_json::to_string(&body)
        .map_err(|e| format!("JSON serialize error: {}", e))?;
    // Note: API key is passed as a header arg to curl subprocess; it may be visible in `ps` output.
    let output = std::process::Command::new("curl")
        .args([
            "-s", "-X", "POST",
            "https://api.anthropic.com/v1/messages",
            "-H", "Content-Type: application/json",
            "-H", "anthropic-version: 2023-06-01",
            "-H", &format!("x-api-key: {}", api_key),
            "-d", &body_str,
        ])
        .output()
        .map_err(|e| format!("Failed to call Anthropic API: {}", e))?;
    if !output.status.success() {
        return Err(format!("API request failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    let response: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| format!("Parse error: {}", e))?;
    if let Some(err) = response["error"]["message"].as_str() {
        return Err(format!("API error: {}", err));
    }
    response["content"][0]["text"].as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| {
            let raw = String::from_utf8_lossy(&output.stdout);
            format!("Unexpected API response: {}", &raw[..200.min(raw.len())])
        })
}
