use crate::model::FlowchartDocument;
use std::path::Path;

pub fn save_document(doc: &FlowchartDocument, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load_document(path: &Path) -> Result<FlowchartDocument, String> {
    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
