mod schema;
mod convert;
pub mod hrf;
pub mod layout;
pub mod llm;

pub use schema::SpecGraph;
pub use convert::{document_to_specgraph, specgraph_to_document};
pub use hrf::{parse_hrf, export_hrf};
pub use llm::LlmConfig;

use crate::model::FlowchartDocument;

/// Detected format of an import file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecFormat {
    Yaml,
    Hrf,
    Prose,
}

/// Auto-detect the format of the input text.
pub fn detect_format(text: &str) -> SpecFormat {
    let trimmed = text.trim();

    // YAML: starts with specgraph: or has YAML frontmatter
    if trimmed.starts_with("specgraph:") || trimmed.starts_with("---") {
        return SpecFormat::Yaml;
    }

    // HRF: has markdown-style headers with ## Nodes or ## Flow
    let lower = trimmed.to_lowercase();
    if lower.contains("## nodes") || lower.contains("## flow") || lower.contains("## edges") {
        return SpecFormat::Hrf;
    }

    // HRF: starts with # Title and has [id] patterns
    if trimmed.starts_with('#') && trimmed.contains('[') && trimmed.contains(']') {
        return SpecFormat::Hrf;
    }

    // Everything else is prose
    SpecFormat::Prose
}

/// Import from any supported format. Auto-detects format.
/// For prose, requires an LlmConfig.
pub fn import_auto(text: &str, llm_config: Option<&LlmConfig>) -> Result<FlowchartDocument, String> {
    match detect_format(text) {
        SpecFormat::Yaml => import_yaml(text),
        SpecFormat::Hrf => parse_hrf(text),
        SpecFormat::Prose => {
            let config = llm_config.ok_or_else(|| {
                "This looks like plain text. Configure LLM settings to convert prose to diagrams.".to_string()
            })?;
            let yaml = llm::prose_to_yaml(text, config)?;
            import_yaml(&yaml)
        }
    }
}

/// Parse a YAML string into a FlowchartDocument.
pub fn import_yaml(yaml: &str) -> Result<FlowchartDocument, String> {
    let sg: SpecGraph = serde_yaml::from_str(yaml).map_err(|e| format!("YAML parse error: {}", e))?;
    specgraph_to_document(&sg)
}

/// Serialize a FlowchartDocument to a YAML string.
pub fn export_yaml(doc: &FlowchartDocument, title: &str) -> Result<String, String> {
    let sg = document_to_specgraph(doc, title);
    serde_yaml::to_string(&sg).map_err(|e| format!("YAML serialize error: {}", e))
}
