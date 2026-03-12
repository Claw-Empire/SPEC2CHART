//! Export the current document as a Mermaid flowchart string.

use crate::model::{FlowchartDocument, NodeId};

/// Generate a Mermaid LR flowchart from the document.
pub fn to_mermaid(doc: &FlowchartDocument) -> String {
    let mut out = String::from("flowchart LR\n");

    // Sanitize labels: strip quotes and newlines
    let sanitize = |s: &str| -> String {
        s.replace('"', "'").replace('\n', " ").trim().to_string()
    };

    // Emit nodes
    for node in &doc.nodes {
        let id = mermaid_id(node.id);
        let label = sanitize(node.display_label());
        if label.is_empty() {
            out.push_str(&format!("    {id}\n"));
        } else {
            out.push_str(&format!("    {id}[\"{label}\"]\n"));
        }
    }

    // Emit edges
    for edge in &doc.edges {
        let src = mermaid_id(edge.source.node_id);
        let tgt = mermaid_id(edge.target.node_id);
        let label = sanitize(&edge.label);
        if label.is_empty() {
            out.push_str(&format!("    {src} --> {tgt}\n"));
        } else {
            out.push_str(&format!("    {src} -->|\"{label}\"| {tgt}\n"));
        }
    }

    out
}

/// Convert a NodeId UUID to a safe Mermaid node identifier (strip hyphens, prefix n_).
fn mermaid_id(id: NodeId) -> String {
    let s = id.0.to_string().replace('-', "");
    format!("n{}", &s[..8]) // use first 8 hex chars — unique enough for diagrams
}
