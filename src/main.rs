mod app;
mod export;
mod history;
mod io;
mod model;
mod specgraph;
mod templates;

use clap::{Parser, Subcommand};
use eframe::egui;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "open-draftly", about = "openDraftly — lightweight diagramming tool")]
struct Cli {
    /// Optional .spec or .hrf file to open on launch
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a .spec file to SVG/PNG/PDF/Mermaid without opening the GUI
    Render {
        input: PathBuf,
        #[arg(short, long)]
        out: PathBuf,
        /// Output format: svg, png, pdf, mermaid (default: svg)
        #[arg(long, default_value = "svg")]
        format: String,
    },
    /// Validate HRF syntax and report errors
    Validate {
        input: PathBuf,
        /// Output as JSON instead of human-readable text (for CI/IDE integration)
        #[arg(long)]
        json: bool,
    },
    /// Export HRF grammar for a template (for LLM context injection)
    Schema {
        #[arg(long, default_value = "")]
        template: String,
    },
    /// Diff two .spec files and report changed nodes/edges
    Diff {
        before: PathBuf,
        after: PathBuf,
        /// Output as JSON instead of human-readable text (for CI/IDE integration)
        #[arg(long)]
        json: bool,
    },
    /// Generate HRF from prose via LLM (reads stdin, writes HRF to stdout)
    Generate {
        #[arg(long, default_value = "")]
        template: String,
        /// LLM model (e.g. claude-opus-4-5, gpt-4o, llama-3.1-70b)
        #[arg(long, default_value = "")]
        model: String,
        /// Custom LLM API endpoint (default: Anthropic; pass OpenAI-compatible URL for other providers)
        #[arg(long, default_value = "")]
        endpoint: String,
        /// API key — overrides ANTHROPIC_API_KEY / LLM_API_KEY env vars
        #[arg(long, default_value = "")]
        api_key: String,
    },
    /// Watch a directory and regenerate output on file changes
    Watch {
        directory: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "")]
        template: String,
        /// Output format: svg, png, pdf, mermaid (default: svg)
        #[arg(long, default_value = "svg")]
        format: String,
    },
    /// List or export built-in diagram templates
    Templates {
        #[command(subcommand)]
        subcommand: TemplatesCmd,
    },
    /// Convert between HRF, spec (JSON), and Mermaid formats
    Convert {
        input: PathBuf,
        /// Target format: hrf, spec, mermaid
        #[arg(long)]
        to: String,
        /// Output file (omit to write to stdout for text formats)
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Start local HTTP render server (POST /render → SVG)
    Serve {
        #[arg(long, default_value = "8080")]
        port: u16,
    },
    /// Print diagram statistics (node/edge counts, shape distribution, connectivity)
    Stats {
        input: PathBuf,
        /// Output as JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },
    /// Check diagram for common quality issues
    Lint {
        input: PathBuf,
        /// Treat warnings as errors (exit 1 on any finding)
        #[arg(long)]
        strict: bool,
        /// Output as JSON instead of human-readable text (for CI/IDE integration)
        #[arg(long)]
        json: bool,
    },
    /// Merge two diagrams into one
    Merge {
        /// First diagram file (HRF or spec JSON)
        base: PathBuf,
        /// Second diagram file to merge in
        overlay: PathBuf,
        /// Output file (omit to write to stdout as HRF)
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum TemplatesCmd {
    /// List all built-in templates grouped by category
    List {
        /// Output as JSON array (for scripting / IDE integration)
        #[arg(long)]
        json: bool,
    },
    /// Print a template's HRF content (use --out to write to a file)
    Get {
        /// Template name (case-insensitive, e.g. "Architecture")
        name: String,
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Output metadata + content as JSON instead of raw HRF
        #[arg(long)]
        json: bool,
    },
}

fn main() -> eframe::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Render { input, out, format }) => {
            cli_render(input, out, &format);
            return Ok(());
        }
        Some(Commands::Validate { input, json }) => {
            cli_validate(input, json);
            return Ok(());
        }
        Some(Commands::Schema { template }) => {
            cli_schema(&template);
            return Ok(());
        }
        Some(Commands::Diff { before, after, json }) => {
            cli_diff(before, after, json);
            return Ok(());
        }
        Some(Commands::Generate { template, model, endpoint, api_key }) => {
            cli_generate(&template, &model, &endpoint, &api_key);
            return Ok(());
        }
        Some(Commands::Watch { directory, out, template, format }) => {
            cli_watch(directory, out, &template, &format);
            return Ok(());
        }
        Some(Commands::Templates { subcommand }) => {
            match subcommand {
                TemplatesCmd::List { json } => cli_templates_list(json),
                TemplatesCmd::Get { name, out, json } => {
                    cli_templates_get(&name, out.as_deref(), json)
                }
            }
            return Ok(());
        }
        Some(Commands::Convert { input, to, out }) => {
            cli_convert(input, &to, out.as_deref());
            return Ok(());
        }
        Some(Commands::Serve { port }) => {
            cli_serve(port);
            return Ok(());
        }
        Some(Commands::Stats { input, json }) => {
            cli_stats(input, json);
            return Ok(());
        }
        Some(Commands::Lint { input, strict, json }) => {
            cli_lint(input, strict, json);
            return Ok(());
        }
        Some(Commands::Merge { base, overlay, out }) => {
            cli_merge(base, overlay, out.as_deref());
            return Ok(());
        }
        None => {} // Fall through to GUI
    }

    // GUI mode (no subcommand)
    let startup_file = cli.file;

    // Load app icon so eframe doesn't replace the .icns with a blank icon at runtime.
    let icon = {
        let bytes = include_bytes!("../assets/icon.iconset/icon_256x256.png");
        let img = image::load_from_memory(bytes)
            .expect("bundled icon PNG is valid")
            .into_rgba8();
        let (w, h) = img.dimensions();
        egui::IconData { rgba: img.into_raw(), width: w, height: h }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 860.0])
            .with_title("openDraftly")
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };
    eframe::run_native(
        "openDraftly",
        options,
        Box::new(move |cc| Ok(Box::new(app::FlowchartApp::new_with_file(cc, startup_file)))),
    )
}

fn cli_render(input: PathBuf, out: PathBuf, format: &str) {
    let spec = std::fs::read_to_string(&input)
        .unwrap_or_else(|e| { eprintln!("Error reading {:?}: {}", input, e); std::process::exit(1); });
    let mut doc = crate::specgraph::hrf::parse_hrf(&spec)
        .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    crate::specgraph::layout::auto_layout(&mut doc);

    match format {
        "svg" => {
            crate::export::export_svg(&doc, &out)
                .unwrap_or_else(|e| { eprintln!("Export error: {}", e); std::process::exit(1); });
        }
        "png" => {
            crate::export::export_png(&doc, &out)
                .unwrap_or_else(|e| { eprintln!("Export error: {}", e); std::process::exit(1); });
        }
        "pdf" => {
            crate::export::export_pdf(&doc, &out)
                .unwrap_or_else(|e| { eprintln!("Export error: {}", e); std::process::exit(1); });
        }
        "mermaid" => {
            let mermaid = crate::app::export_mermaid::to_mermaid(&doc);
            std::fs::write(&out, mermaid)
                .unwrap_or_else(|e| { eprintln!("Write error: {}", e); std::process::exit(1); });
        }
        other => {
            eprintln!("Unknown format {:?}. Valid formats: svg, png, pdf, mermaid", other);
            std::process::exit(1);
        }
    }
    println!("Rendered {:?} → {:?} ({})", input, out, format);
}

fn cli_validate(input: PathBuf, json: bool) {
    let read_result = std::fs::read_to_string(&input);
    if json {
        // JSON mode: report all outcomes through structured output including
        // I/O errors, so CI pipelines never have to parse stderr.
        let spec = match read_result {
            Ok(s) => s,
            Err(e) => {
                let payload = serde_json::json!({
                    "file": input.display().to_string(),
                    "valid": false,
                    "error": format!("read error: {}", e),
                    "line": serde_json::Value::Null,
                    "node_count": 0,
                    "edge_count": 0,
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap());
                std::process::exit(1);
            }
        };
        match crate::specgraph::hrf::parse_hrf(&spec) {
            Ok(doc) => {
                let payload = serde_json::json!({
                    "file": input.display().to_string(),
                    "valid": true,
                    "error": serde_json::Value::Null,
                    "line": serde_json::Value::Null,
                    "node_count": doc.nodes.len(),
                    "edge_count": doc.edges.len(),
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap());
            }
            Err(e) => {
                let msg = e.to_string();
                let line = parse_error_line(&msg);
                // Strip the "Line N:" prefix from the stored error so callers
                // don't double-report the location. If no prefix was present,
                // `line` is null and the full message is kept as-is.
                let clean = match line {
                    Some(_) => msg
                        .split_once(':')
                        .map(|(_, rest)| rest.trim().to_string())
                        .unwrap_or_else(|| msg.clone()),
                    None => msg.clone(),
                };
                let payload = serde_json::json!({
                    "file": input.display().to_string(),
                    "valid": false,
                    "error": clean,
                    "line": line.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null),
                    "node_count": 0,
                    "edge_count": 0,
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap());
                std::process::exit(1);
            }
        }
        return;
    }

    // Human-readable mode
    let spec = read_result.unwrap_or_else(|e| {
        eprintln!("Error reading {:?}: {}", input, e);
        std::process::exit(1);
    });
    match crate::specgraph::hrf::parse_hrf(&spec) {
        Ok(doc) => {
            println!("✓ Valid — {} nodes, {} edges", doc.nodes.len(), doc.edges.len());
        }
        Err(e) => {
            eprintln!("✗ Invalid: {}", e);
            std::process::exit(1);
        }
    }
}

/// Extract the leading line number from a parser error message like
/// `"Line 2: missing closing ]"`. Returns `None` if no `Line N:` prefix is
/// present — some errors are not line-anchored (e.g. I/O or structural errors).
fn parse_error_line(msg: &str) -> Option<u32> {
    let rest = msg.strip_prefix("Line ")?;
    let (num_str, _) = rest.split_once(':')?;
    num_str.trim().parse::<u32>().ok()
}

fn cli_schema(template: &str) {
    let schema = format!(
        "HRF (Human-Readable Format) for openDraftly diagrams.\n\
         Template: {}\n\
         \n\
         SYNTAX:\n\
         ## Config\n  flow = TB|LR|RL|BT\n  template: roadmap|gtm|orgchart|pipeline|journey\n\
         \n\
         ## Nodes\n  - [id] Label {{tag}} {{tag}}\n    Description text\n\
         \n\
         ## Flow\n  id --> id: edge label\n\
         \n\
         COMMON TAGS:\n\
         {{done}} {{wip}} {{todo}} {{blocked}} — status\n\
         {{milestone}} {{phase:Q1}} {{date:2026-Q1}} — timeline\n\
         {{lane:Name}} — swimlane\n\
         {{metric:$2M}} {{owner:@alice}} — badges\n\
         {{dep:id}} — dependency edge\n\
         {{shape:person|screen|cylinder|cloud|document}} — shapes\n\
         {{revenue}} {{cost}} {{growth}} {{risk}} — business\n\
         {{icon:🚀}} {{glow}} {{dim}} {{bold}} — style\n\
         ",
        if template.is_empty() { "none" } else { template }
    );
    println!("{}", schema);
}

fn cli_diff(before: PathBuf, after: PathBuf, json: bool) {
    let spec_a = std::fs::read_to_string(&before)
        .unwrap_or_else(|e| { eprintln!("Error reading {:?}: {}", before, e); std::process::exit(1); });
    let spec_b = std::fs::read_to_string(&after)
        .unwrap_or_else(|e| { eprintln!("Error reading {:?}: {}", after, e); std::process::exit(1); });
    let doc_a = crate::specgraph::hrf::parse_hrf(&spec_a)
        .unwrap_or_else(|e| { eprintln!("Parse error in {:?}: {}", before, e); std::process::exit(1); });
    let doc_b = crate::specgraph::hrf::parse_hrf(&spec_b)
        .unwrap_or_else(|e| { eprintln!("Parse error in {:?}: {}", after, e); std::process::exit(1); });

    let node_key = |n: &crate::model::Node| -> String {
        if n.hrf_id.is_empty() { n.display_label().to_string() } else { n.hrf_id.clone() }
    };

    // Build maps for comparison
    let nodes_a: std::collections::HashMap<String, &crate::model::Node> =
        doc_a.nodes.iter().map(|n| (node_key(n), n)).collect();
    let nodes_b: std::collections::HashMap<String, &crate::model::Node> =
        doc_b.nodes.iter().map(|n| (node_key(n), n)).collect();

    let mut added_nodes: Vec<&String> = nodes_b.keys().filter(|k| !nodes_a.contains_key(k.as_str())).collect();
    let mut removed_nodes: Vec<&String> = nodes_a.keys().filter(|k| !nodes_b.contains_key(k.as_str())).collect();
    added_nodes.sort();
    removed_nodes.sort();

    // Detect modified nodes — same key but changed label, shape, or fill color
    let shape_name = |n: &crate::model::Node| -> &'static str {
        match &n.kind {
            crate::model::NodeKind::Shape { shape, .. } => match shape {
                crate::model::NodeShape::Rectangle => "rect",
                crate::model::NodeShape::RoundedRect => "rounded",
                crate::model::NodeShape::Diamond => "diamond",
                crate::model::NodeShape::Circle => "circle",
                crate::model::NodeShape::Parallelogram => "parallelogram",
                crate::model::NodeShape::Hexagon => "hexagon",
                crate::model::NodeShape::Triangle => "triangle",
                crate::model::NodeShape::Callout => "callout",
                crate::model::NodeShape::Person => "person",
                crate::model::NodeShape::Screen => "screen",
                crate::model::NodeShape::Cylinder => "cylinder",
                crate::model::NodeShape::Cloud => "cloud",
                crate::model::NodeShape::Document => "document",
                crate::model::NodeShape::Channel => "channel",
                crate::model::NodeShape::Segment => "segment",
                crate::model::NodeShape::Connector => "connector",
            },
            crate::model::NodeKind::StickyNote { .. } => "sticky",
            crate::model::NodeKind::Entity { .. } => "entity",
            crate::model::NodeKind::Text { .. } => "text",
        }
    };

    let mut modified_nodes: Vec<(String, Vec<String>)> = Vec::new();
    for (key, node_a) in &nodes_a {
        if let Some(node_b) = nodes_b.get(key) {
            let mut changes: Vec<String> = Vec::new();
            if node_a.display_label() != node_b.display_label() {
                changes.push(format!("label: {:?} → {:?}", node_a.display_label(), node_b.display_label()));
            }
            if shape_name(node_a) != shape_name(node_b) {
                changes.push(format!("shape: {} → {}", shape_name(node_a), shape_name(node_b)));
            }
            if node_a.style.fill_color != node_b.style.fill_color {
                changes.push(format!(
                    "fill: #{:02x}{:02x}{:02x} → #{:02x}{:02x}{:02x}",
                    node_a.style.fill_color[0], node_a.style.fill_color[1], node_a.style.fill_color[2],
                    node_b.style.fill_color[0], node_b.style.fill_color[1], node_b.style.fill_color[2],
                ));
            }
            if node_a.tag != node_b.tag {
                let a = node_a.tag.as_ref().map(|t| t.label()).unwrap_or("none");
                let b = node_b.tag.as_ref().map(|t| t.label()).unwrap_or("none");
                changes.push(format!("tag: {} → {}", a, b));
            }
            if !changes.is_empty() {
                modified_nodes.push((key.clone(), changes));
            }
        }
    }
    modified_nodes.sort_by(|a, b| a.0.cmp(&b.0));

    // Build NodeId → human-readable key maps so edge diffs show names, not UUIDs.
    let id_to_key_a: std::collections::HashMap<crate::model::NodeId, String> =
        doc_a.nodes.iter().map(|n| (n.id, node_key(n))).collect();
    let id_to_key_b: std::collections::HashMap<crate::model::NodeId, String> =
        doc_b.nodes.iter().map(|n| (n.id, node_key(n))).collect();

    // Edges are keyed by (source, target) so we can detect MODIFIED edges
    // (same endpoints but changed label or style) separately from pure
    // add/remove. A label change used to show as a paired `- edge:` and
    // `+ edge:` with no indication they were related.
    type EdgeKey = (String, String);
    let edge_key_a = |e: &crate::model::Edge| -> EdgeKey {
        (
            id_to_key_a.get(&e.source.node_id).cloned().unwrap_or_else(|| "?".into()),
            id_to_key_a.get(&e.target.node_id).cloned().unwrap_or_else(|| "?".into()),
        )
    };
    let edge_key_b = |e: &crate::model::Edge| -> EdgeKey {
        (
            id_to_key_b.get(&e.source.node_id).cloned().unwrap_or_else(|| "?".into()),
            id_to_key_b.get(&e.target.node_id).cloned().unwrap_or_else(|| "?".into()),
        )
    };

    // Summarize edge visual style in a human-readable form for diff output.
    let edge_style_summary = |e: &crate::model::Edge| -> String {
        let mut parts: Vec<String> = Vec::new();
        if e.style.dashed { parts.push("dashed".into()); }
        if e.style.glow { parts.push("glow".into()); }
        if e.style.animated { parts.push("animated".into()); }
        if e.style.width > 4.0 { parts.push("thick".into()); }
        if e.style.orthogonal { parts.push("ortho".into()); }
        parts.join(",")
    };

    // Build lookup maps. When the same (src,tgt) appears multiple times
    // (parallel edges), keep the first — unusual but not fatal.
    let mut edges_a_map: std::collections::HashMap<EdgeKey, &crate::model::Edge> =
        std::collections::HashMap::new();
    for e in &doc_a.edges {
        edges_a_map.entry(edge_key_a(e)).or_insert(e);
    }
    let mut edges_b_map: std::collections::HashMap<EdgeKey, &crate::model::Edge> =
        std::collections::HashMap::new();
    for e in &doc_b.edges {
        edges_b_map.entry(edge_key_b(e)).or_insert(e);
    }

    let mut added_edges: Vec<(EdgeKey, String)> = Vec::new();
    let mut removed_edges: Vec<(EdgeKey, String)> = Vec::new();
    let mut modified_edges: Vec<(EdgeKey, Vec<String>)> = Vec::new();

    for (key, edge_b) in &edges_b_map {
        match edges_a_map.get(key) {
            None => {
                let label = if edge_b.label.is_empty() { String::new() } else { format!(" [{}]", edge_b.label) };
                added_edges.push((key.clone(), label));
            }
            Some(edge_a) => {
                let mut changes: Vec<String> = Vec::new();
                if edge_a.label != edge_b.label {
                    changes.push(format!("label: {:?} → {:?}", edge_a.label, edge_b.label));
                }
                let style_a = edge_style_summary(edge_a);
                let style_b = edge_style_summary(edge_b);
                if style_a != style_b {
                    let fmt = |s: &str| if s.is_empty() { "plain".into() } else { s.to_string() };
                    changes.push(format!("style: {} → {}", fmt(&style_a), fmt(&style_b)));
                }
                if edge_a.style.color != edge_b.style.color {
                    changes.push(format!(
                        "color: #{:02x}{:02x}{:02x} → #{:02x}{:02x}{:02x}",
                        edge_a.style.color[0], edge_a.style.color[1], edge_a.style.color[2],
                        edge_b.style.color[0], edge_b.style.color[1], edge_b.style.color[2],
                    ));
                }
                if !changes.is_empty() {
                    modified_edges.push((key.clone(), changes));
                }
            }
        }
    }
    for (key, edge_a) in &edges_a_map {
        if !edges_b_map.contains_key(key) {
            let label = if edge_a.label.is_empty() { String::new() } else { format!(" [{}]", edge_a.label) };
            removed_edges.push((key.clone(), label));
        }
    }
    added_edges.sort_by(|a, b| a.0.cmp(&b.0));
    removed_edges.sort_by(|a, b| a.0.cmp(&b.0));
    modified_edges.sort_by(|a, b| a.0.cmp(&b.0));

    let total_changes = added_nodes.len() + removed_nodes.len() + modified_nodes.len()
        + added_edges.len() + removed_edges.len() + modified_edges.len();

    // JSON output path: mirror the shape used by `lint --json` / `stats --json`
    // so tooling can consume all three commands with the same JSON idioms.
    if json {
        let modified_nodes_json: Vec<serde_json::Value> = modified_nodes
            .iter()
            .map(|(key, changes)| serde_json::json!({ "key": key, "changes": changes }))
            .collect();
        let added_edges_json: Vec<serde_json::Value> = added_edges
            .iter()
            .map(|((s, t), label)| serde_json::json!({
                "source": s,
                "target": t,
                "label": label.trim_start_matches(' ').trim_start_matches('[').trim_end_matches(']'),
            }))
            .collect();
        let removed_edges_json: Vec<serde_json::Value> = removed_edges
            .iter()
            .map(|((s, t), label)| serde_json::json!({
                "source": s,
                "target": t,
                "label": label.trim_start_matches(' ').trim_start_matches('[').trim_end_matches(']'),
            }))
            .collect();
        let modified_edges_json: Vec<serde_json::Value> = modified_edges
            .iter()
            .map(|((s, t), changes)| serde_json::json!({
                "source": s,
                "target": t,
                "changes": changes,
            }))
            .collect();
        let payload = serde_json::json!({
            "before": before.display().to_string(),
            "after": after.display().to_string(),
            "added_nodes": added_nodes,
            "removed_nodes": removed_nodes,
            "modified_nodes": modified_nodes_json,
            "added_edges": added_edges_json,
            "removed_edges": removed_edges_json,
            "modified_edges": modified_edges_json,
            "summary": {
                "added_nodes": added_nodes.len(),
                "removed_nodes": removed_nodes.len(),
                "modified_nodes": modified_nodes.len(),
                "added_edges": added_edges.len(),
                "removed_edges": removed_edges.len(),
                "modified_edges": modified_edges.len(),
                "total_changes": total_changes,
            },
            "clean": total_changes == 0,
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
        return;
    }

    // Output in sorted order
    for id in &added_nodes {
        println!("+ node: {}", id);
    }
    for id in &removed_nodes {
        println!("- node: {}", id);
    }
    for (key, changes) in &modified_nodes {
        println!("~ node: {} ({})", key, changes.join(", "));
    }
    for ((s, t), label) in &added_edges {
        println!("+ edge: {} → {}{}", s, t, label);
    }
    for ((s, t), label) in &removed_edges {
        println!("- edge: {} → {}{}", s, t, label);
    }
    for ((s, t), changes) in &modified_edges {
        println!("~ edge: {} → {} ({})", s, t, changes.join(", "));
    }

    if total_changes == 0 {
        println!("✓ No differences");
    } else {
        println!();
        println!(
            "Summary: +{} nodes, -{} nodes, ~{} modified nodes, +{} edges, -{} edges, ~{} modified edges",
            added_nodes.len(), removed_nodes.len(), modified_nodes.len(),
            added_edges.len(), removed_edges.len(), modified_edges.len()
        );
    }
}

fn cli_generate(template: &str, model: &str, endpoint: &str, api_key_flag: &str) {
    use crate::specgraph::llm::LlmConfig;

    // Resolve API key: --api-key flag > ANTHROPIC_API_KEY > LLM_API_KEY
    let api_key = if !api_key_flag.is_empty() {
        api_key_flag.to_string()
    } else {
        std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("LLM_API_KEY"))
            .unwrap_or_else(|_| {
                eprintln!(
                    "Error: no API key found.\n\
                     Set ANTHROPIC_API_KEY (Anthropic) or LLM_API_KEY (other providers),\n\
                     or pass --api-key <key>."
                );
                std::process::exit(1);
            })
    };

    // Build config: explicit endpoint → custom; otherwise default to Anthropic.
    let config = if !endpoint.is_empty() {
        let resolved_model = if !model.is_empty() {
            model.to_string()
        } else {
            eprintln!("Note: --model not specified for custom endpoint; defaulting to gpt-4o. Pass --model to silence this.");
            "gpt-4o".to_string()
        };
        LlmConfig {
            endpoint: endpoint.to_string(),
            api_key,
            model: resolved_model,
        }
    } else {
        LlmConfig::anthropic(
            api_key,
            if !model.is_empty() { Some(model.to_string()) } else { None },
        )
    };

    let mut prose = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut prose)
        .unwrap_or_else(|e| { eprintln!("Error reading stdin: {}", e); std::process::exit(1); });

    match crate::specgraph::llm::prose_to_hrf(&prose, template, &config) {
        Ok(hrf) => print!("{}", hrf),
        Err(e) => {
            eprintln!("LLM error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cli_watch(directory: PathBuf, out: PathBuf, template: &str, format: &str) {
    use notify::{Watcher, RecursiveMode};
    use std::sync::mpsc::channel;

    println!("Watching {:?} → {:?} ({})", directory, out, format);
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx)
        .unwrap_or_else(|e| { eprintln!("Watch error: {}", e); std::process::exit(1); });
    watcher.watch(&directory, RecursiveMode::Recursive)
        .unwrap_or_else(|e| { eprintln!("Watch error: {}", e); std::process::exit(1); });

    // Initial render
    regenerate_watch(&directory, &out, template, format);

    for event in rx.into_iter().flatten() {
        if event.paths.iter().any(|p| p.extension().is_some_and(|e| e == "spec" || e == "hrf")) {
            println!("Change detected — regenerating...");
            regenerate_watch(&directory, &out, template, format);
        }
    }
}

fn regenerate_watch(dir: &std::path::Path, out: &std::path::Path, _template: &str, format: &str) {
    let mut spec_files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "spec" || x == "hrf"))
        .collect();
    spec_files.sort();
    if let Some(spec_path) = spec_files.into_iter().next() {
        cli_render(spec_path, out.to_path_buf(), format);
    }
}

fn cli_templates_list(json: bool) {
    use crate::templates::TEMPLATES;
    if json {
        // Deterministic ordering (category, name) so CI snapshots are stable.
        let mut sorted: Vec<&crate::templates::Template> = TEMPLATES.iter().collect();
        sorted.sort_by(|a, b| a.category.cmp(b.category).then_with(|| a.name.cmp(b.name)));
        let arr: Vec<serde_json::Value> = sorted
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "category": t.category,
                    "description": t.description,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr).unwrap());
        return;
    }
    // Group by category so each category header prints exactly once,
    // regardless of registration order in TEMPLATES.
    let mut by_category: std::collections::BTreeMap<&str, Vec<&crate::templates::Template>> =
        std::collections::BTreeMap::new();
    for t in TEMPLATES {
        by_category.entry(t.category).or_default().push(t);
    }
    for (category, items) in &by_category {
        println!("\n{}:", category);
        let mut sorted = items.clone();
        sorted.sort_by_key(|t| t.name);
        for t in sorted {
            println!("  {:20}  {}", t.name, t.description);
        }
    }
    println!();
}

fn cli_templates_get(name: &str, out: Option<&std::path::Path>, json: bool) {
    use crate::templates::TEMPLATES;
    let name_lower = name.to_lowercase();
    let template = TEMPLATES.iter().find(|t| t.name.to_lowercase() == name_lower)
        .unwrap_or_else(|| {
            if json {
                // Emit a structured not-found so IDE consumers don't have to
                // parse stderr to distinguish "bad name" from "actual error".
                let payload = serde_json::json!({
                    "error": format!("Template {:?} not found", name),
                    "available": TEMPLATES.iter().map(|t| t.name).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap());
            } else {
                eprintln!("Template {:?} not found. Run `templates list` to see available templates.", name);
            }
            std::process::exit(1);
        });
    if json {
        // JSON path: always emit metadata + content, regardless of --out.
        // If --out is also set, we still write the raw HRF to the file so
        // a single call can both populate a file and feed metadata to a script.
        if let Some(path) = out {
            std::fs::write(path, template.content).unwrap_or_else(|e| {
                let err = serde_json::json!({ "error": format!("write error: {}", e) });
                println!("{}", serde_json::to_string_pretty(&err).unwrap());
                std::process::exit(1);
            });
        }
        let payload = serde_json::json!({
            "name": template.name,
            "category": template.category,
            "description": template.description,
            "content": template.content,
            "written_to": out.map(|p| p.display().to_string()),
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
        return;
    }
    match out {
        Some(path) => {
            std::fs::write(path, template.content)
                .unwrap_or_else(|e| { eprintln!("Write error: {}", e); std::process::exit(1); });
            println!("Wrote {} template to {:?}", template.name, path);
        }
        None => print!("{}", template.content),
    }
}

fn cli_convert(input: PathBuf, to: &str, out: Option<&std::path::Path>) {
    let src = std::fs::read_to_string(&input)
        .unwrap_or_else(|e| { eprintln!("Error reading {:?}: {}", input, e); std::process::exit(1); });

    // Detect input format by extension
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");
    let doc = match ext {
        "hrf" | "spec" if ext == "hrf" => {
            crate::specgraph::hrf::parse_hrf(&src)
                .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); })
        }
        _ => {
            // Try HRF parse first, then JSON
            crate::specgraph::hrf::parse_hrf(&src)
                .or_else(|_| serde_json::from_str::<crate::model::FlowchartDocument>(&src).map_err(|e| e.to_string()))
                .unwrap_or_else(|_| {
                    eprintln!("Could not parse {:?} as HRF or spec JSON. Specify a .hrf or .spec file.", input);
                    std::process::exit(1);
                })
        }
    };

    let output_text = match to {
        "hrf" => crate::specgraph::hrf::export_hrf(&doc, ""),
        "spec" => serde_json::to_string_pretty(&doc)
            .unwrap_or_else(|e| { eprintln!("Serialization error: {}", e); std::process::exit(1); }),
        "mermaid" => crate::app::export_mermaid::to_mermaid(&doc),
        other => {
            eprintln!("Unknown target format {:?}. Valid formats: hrf, spec, mermaid", other);
            std::process::exit(1);
        }
    };

    match out {
        Some(path) => {
            std::fs::write(path, &output_text)
                .unwrap_or_else(|e| { eprintln!("Write error: {}", e); std::process::exit(1); });
            println!("Converted {:?} → {:?} ({})", input, path, to);
        }
        None => print!("{}", output_text),
    }
}

fn cli_serve(port: u16) {
    use tiny_http::{Server, Response, Header};
    let addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&addr).unwrap_or_else(|e| {
        eprintln!("Failed to start server on {}: {}", addr, e);
        std::process::exit(1);
    });
    println!("openDraftly render server listening on http://localhost:{}", port);
    println!("POST /render with HRF spec body → returns SVG");

    for request in server.incoming_requests() {
        if request.url() != "/render" || request.method().as_str() != "POST" {
            let _ = request.respond(Response::from_string("POST /render only").with_status_code(404));
            continue;
        }
        let mut body = String::new();
        let mut r = request;
        let _ = std::io::Read::read_to_string(r.as_reader(), &mut body);

        match crate::specgraph::hrf::parse_hrf(&body) {
            Ok(mut doc) => {
                crate::specgraph::layout::auto_layout(&mut doc);
                let tmp = std::env::temp_dir().join(format!("lf_serve_{}.svg", uuid::Uuid::new_v4()));
                match crate::export::export_svg(&doc, &tmp) {
                    Ok(()) => {
                        let svg = std::fs::read_to_string(&tmp).unwrap_or_default();
                        let _ = std::fs::remove_file(&tmp);
                        let response = Response::from_string(svg).with_header(
                            Header::from_bytes("Content-Type", "image/svg+xml").expect("valid static header"),
                        );
                        let _ = r.respond(response);
                    }
                    Err(e) => {
                        let _ = r.respond(
                            Response::from_string(format!("Export error: {}", e))
                                .with_status_code(500),
                        );
                    }
                }
            }
            Err(e) => {
                let _ = r.respond(
                    Response::from_string(format!("Parse error: {}", e)).with_status_code(400),
                );
            }
        }
    }
}

/// Load an HRF or spec-JSON file into a FlowchartDocument.
fn load_doc(path: &std::path::Path) -> crate::model::FlowchartDocument {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| { eprintln!("Error reading {:?}: {}", path, e); std::process::exit(1); });
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "hrf" => crate::specgraph::hrf::parse_hrf(&src)
            .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); }),
        "spec" => crate::specgraph::hrf::parse_hrf(&src)
            .or_else(|_| serde_json::from_str::<crate::model::FlowchartDocument>(&src).map_err(|e| e.to_string()))
            .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); }),
        "json" => serde_json::from_str(&src)
            .unwrap_or_else(|e| { eprintln!("JSON parse error: {}", e); std::process::exit(1); }),
        _ => crate::specgraph::hrf::parse_hrf(&src)
            .or_else(|_| serde_json::from_str::<crate::model::FlowchartDocument>(&src).map_err(|e| e.to_string()))
            .unwrap_or_else(|_| { eprintln!("Could not parse {:?} as HRF or spec JSON.", path); std::process::exit(1); }),
    }
}

fn cli_stats(input: PathBuf, json: bool) {
    use std::collections::HashMap;
    let doc = load_doc(&input);

    let node_count = doc.nodes.len();
    let edge_count = doc.edges.len();

    // Shape distribution
    let mut shapes: HashMap<&str, usize> = HashMap::new();
    let mut sections: HashMap<&str, usize> = HashMap::new();
    let mut frames = 0usize;
    let mut locked = 0usize;
    let mut with_comment = 0usize;
    let mut with_url = 0usize;
    let mut with_owner = 0usize;

    for node in &doc.nodes {
        if node.is_frame { frames += 1; }
        if node.locked { locked += 1; }
        if !node.comment.is_empty() { with_comment += 1; }
        if !node.url.is_empty() { with_url += 1; }
        if node.owner.is_some() { with_owner += 1; }
        if !node.section_name.is_empty() {
            *sections.entry(node.section_name.as_str()).or_default() += 1;
        }
        let shape_name = match &node.kind {
            crate::model::NodeKind::Shape { shape, .. } => match shape {
                crate::model::NodeShape::Rectangle => "Rectangle",
                crate::model::NodeShape::RoundedRect => "RoundedRect",
                crate::model::NodeShape::Diamond => "Diamond",
                crate::model::NodeShape::Circle => "Circle",
                crate::model::NodeShape::Parallelogram => "Parallelogram",
                crate::model::NodeShape::Connector => "Connector",
                crate::model::NodeShape::Hexagon => "Hexagon",
                crate::model::NodeShape::Triangle => "Triangle",
                crate::model::NodeShape::Callout => "Callout",
                crate::model::NodeShape::Person => "Person",
                crate::model::NodeShape::Screen => "Screen",
                crate::model::NodeShape::Cylinder => "Cylinder",
                crate::model::NodeShape::Cloud => "Cloud",
                crate::model::NodeShape::Document => "Document",
                crate::model::NodeShape::Channel => "Channel",
                crate::model::NodeShape::Segment => "Segment",
            },
            crate::model::NodeKind::StickyNote { .. } => "StickyNote",
            crate::model::NodeKind::Entity { .. } => "Entity",
            crate::model::NodeKind::Text { .. } => "Text",
        };
        *shapes.entry(shape_name).or_default() += 1;
    }

    // Connectivity: count edges per node (degree)
    let mut in_degree: HashMap<crate::model::NodeId, usize> = HashMap::new();
    let mut out_degree: HashMap<crate::model::NodeId, usize> = HashMap::new();
    for edge in &doc.edges {
        *out_degree.entry(edge.source.node_id).or_default() += 1;
        *in_degree.entry(edge.target.node_id).or_default() += 1;
    }
    let max_in = in_degree.values().max().copied().unwrap_or(0);
    let max_out = out_degree.values().max().copied().unwrap_or(0);
    let connected_nodes: std::collections::HashSet<_> = in_degree.keys().chain(out_degree.keys()).collect();
    let disconnected = node_count.saturating_sub(connected_nodes.len());

    // Edge label stats
    let labeled_edges = doc.edges.iter().filter(|e| !e.label.is_empty()).count();

    // Tag distribution
    let mut tags: HashMap<&str, usize> = HashMap::new();
    for node in &doc.nodes {
        if let Some(tag) = &node.tag {
            *tags.entry(tag.label()).or_default() += 1;
        }
    }

    // Connected components (weakly connected, treating edges as undirected)
    let component_count: usize = {
        use crate::model::NodeId;
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for node in &doc.nodes {
            if !node.is_frame {
                adj.entry(node.id).or_default();
            }
        }
        for edge in &doc.edges {
            adj.entry(edge.source.node_id).or_default().push(edge.target.node_id);
            adj.entry(edge.target.node_id).or_default().push(edge.source.node_id);
        }
        let mut visited: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        let mut components = 0usize;
        for &start in adj.keys() {
            if visited.contains(&start) { continue; }
            components += 1;
            let mut stack = vec![start];
            while let Some(cur) = stack.pop() {
                if !visited.insert(cur) { continue; }
                if let Some(nbrs) = adj.get(&cur) {
                    for &n in nbrs { stack.push(n); }
                }
            }
        }
        components
    };

    // Edge density: E / (N*(N-1)/2). 1.0 = complete graph, close to 0 = sparse.
    let non_frame_count = doc.nodes.iter().filter(|n| !n.is_frame).count();
    let max_possible_edges = if non_frame_count >= 2 {
        (non_frame_count * (non_frame_count - 1)) / 2
    } else {
        0
    };
    let edge_density: f32 = if max_possible_edges > 0 {
        edge_count as f32 / max_possible_edges as f32
    } else {
        0.0
    };

    // Layout depth (longest path via BFS layering, same as layout engine).
    // Also reports whether the directed graph has a cycle: Kahn's topo
    // sort leaves `rem[v] > 0` for any node trapped in a cycle since its
    // in-degree never reaches zero. Self-loops are excluded (matching
    // layout) so they don't false-positive the cycle flag.
    let (layout_depth, has_cycle, cycle_node_count) = {
        let node_idx: HashMap<crate::model::NodeId, usize> =
            doc.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let n = doc.nodes.len();
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut in_deg: Vec<i32> = vec![0; n];
        for edge in &doc.edges {
            if let (Some(&from), Some(&to)) =
                (node_idx.get(&edge.source.node_id), node_idx.get(&edge.target.node_id))
            {
                if from != to {
                    adj[from].push(to);
                    in_deg[to] += 1;
                }
            }
        }
        let mut layer: Vec<i32> = vec![0; n];
        let mut rem: Vec<i32> = in_deg.clone();
        let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for (i, &d) in rem.iter().enumerate() {
            if d == 0 { queue.push_back(i); }
        }
        let mut processed = 0usize;
        while let Some(u) = queue.pop_front() {
            processed += 1;
            for &v in &adj[u] {
                let cand = layer[u] + 1;
                if cand > layer[v] { layer[v] = cand; }
                rem[v] -= 1;
                if rem[v] == 0 { queue.push_back(v); }
            }
        }
        // Nodes still holding in-degree > 0 are trapped in a cycle.
        let cycle_nodes = n.saturating_sub(processed);
        let depth = layer.into_iter().max().unwrap_or(0) as usize + 1;
        (depth, cycle_nodes > 0, cycle_nodes)
    };

    // Average degree across non-frame nodes. Each edge contributes +1 to
    // source out-degree and +1 to target in-degree → +2 total degrees.
    let avg_degree: f32 = if non_frame_count > 0 {
        (2.0 * edge_count as f32) / non_frame_count as f32
    } else {
        0.0
    };

    if json {
        // Use serde_json::json! for robust escaping — hand-built JSON broke
        // on tag/shape names containing quotes or backslashes, and had
        // non-deterministic HashMap ordering.
        let tags_obj: serde_json::Map<String, serde_json::Value> = tags
            .iter()
            .map(|(k, v)| ((*k).to_string(), serde_json::json!(*v)))
            .collect();
        let shapes_obj: serde_json::Map<String, serde_json::Value> = shapes
            .iter()
            .map(|(k, v)| ((*k).to_string(), serde_json::json!(*v)))
            .collect();
        let sections_obj: serde_json::Map<String, serde_json::Value> = sections
            .iter()
            .map(|(k, v)| ((*k).to_string(), serde_json::json!(*v)))
            .collect();
        let payload = serde_json::json!({
            "file": input.display().to_string(),
            "nodes": node_count,
            "edges": edge_count,
            "frames": frames,
            "disconnected_nodes": disconnected,
            "locked_nodes": locked,
            "nodes_with_comments": with_comment,
            "nodes_with_urls": with_url,
            "nodes_with_owners": with_owner,
            "labeled_edges": labeled_edges,
            "max_in_degree": max_in,
            "max_out_degree": max_out,
            "layout_depth": layout_depth,
            "has_cycle": has_cycle,
            "cycle_node_count": cycle_node_count,
            "avg_degree": ((avg_degree as f64 * 1000.0).round() / 1000.0),
            "connected_components": component_count,
            "edge_density": ((edge_density as f64 * 1000.0).round() / 1000.0),
            "tags": tags_obj,
            "shapes": shapes_obj,
            "sections": sections_obj,
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else {
        println!("Diagram Statistics: {:?}", input);
        println!("─────────────────────────────────────");
        println!("  Nodes:             {}", node_count);
        println!("  Edges:             {}", edge_count);
        println!("  Frames/Groups:     {}", frames);
        println!("  Disconnected:      {}", disconnected);
        println!("  Locked:            {}", locked);
        println!("  With comments:     {}", with_comment);
        println!("  With URLs:         {}", with_url);
        println!("  With owners:       {}", with_owner);
        println!("  Labeled edges:     {}/{}", labeled_edges, edge_count);
        println!("  Max in-degree:     {}", max_in);
        println!("  Max out-degree:    {}", max_out);
        println!("  Layout depth:      {} layer{}", layout_depth, if layout_depth == 1 { "" } else { "s" });
        println!("  Avg degree:        {:.2}", avg_degree);
        println!(
            "  Components:        {} subgraph{}",
            component_count,
            if component_count == 1 { "" } else { "s" }
        );
        println!("  Edge density:      {:.1}%", edge_density * 100.0);
        if has_cycle {
            println!(
                "  Cycle:             YES ({} node{} trapped)",
                cycle_node_count,
                if cycle_node_count == 1 { "" } else { "s" }
            );
        } else {
            println!("  Cycle:             no (DAG)");
        }
        if !tags.is_empty() {
            println!();
            println!("  Tag distribution:");
            let mut sorted: Vec<_> = tags.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            for (name, count) in sorted {
                println!("    {:16} {}", name, count);
            }
        }
        if !shapes.is_empty() {
            println!();
            println!("  Shape distribution:");
            let mut sorted: Vec<_> = shapes.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            for (name, count) in sorted {
                println!("    {:16} {}", name, count);
            }
        }
        if !sections.is_empty() {
            println!();
            println!("  Sections:");
            let mut sorted: Vec<_> = sections.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            for (name, count) in sorted {
                println!("    {:16} {} nodes", name, count);
            }
        }
    }
}

/// Byte-level Levenshtein distance. Used by lint passes that compare
/// user-typed identifiers against each other (e.g. detecting inline
/// group typo-splits). Not exposed from hrf.rs because the suggest_*
/// helpers there each inline the same algorithm for encapsulation —
/// cli_lint is the only cross-cutting consumer.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let m = a_bytes.len();
    let n = b_bytes.len();
    if m == 0 { return n; }
    if n == 0 { return m; }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

fn cli_lint(input: PathBuf, strict: bool, json: bool) {
    // In JSON mode, handle parse errors by emitting a structured payload so
    // downstream tools always see valid JSON. In text mode, fall through to
    // load_doc's default behavior (eprint + exit 1).
    let doc = if json {
        let src = match std::fs::read_to_string(&input) {
            Ok(s) => s,
            Err(e) => {
                let payload = serde_json::json!({
                    "file": input.display().to_string(),
                    "node_count": 0,
                    "edge_count": 0,
                    "errors": [format!("Could not read file: {}", e)],
                    "warnings": [],
                    "error_count": 1,
                    "warning_count": 0,
                    "clean": false,
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap());
                std::process::exit(1);
            }
        };
        let parsed = crate::specgraph::hrf::parse_hrf(&src)
            .or_else(|e| serde_json::from_str::<crate::model::FlowchartDocument>(&src).map_err(|_| e));
        match parsed {
            Ok(d) => d,
            Err(e) => {
                let payload = serde_json::json!({
                    "file": input.display().to_string(),
                    "node_count": 0,
                    "edge_count": 0,
                    "errors": [format!("Parse error: {}", e)],
                    "warnings": [],
                    "error_count": 1,
                    "warning_count": 0,
                    "clean": false,
                });
                println!("{}", serde_json::to_string_pretty(&payload).unwrap());
                std::process::exit(1);
            }
        }
    } else {
        load_doc(&input)
    };
    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // Shape-tag typo detection: inspect each node's `unknown_tags` (preserved
    // by the HRF parser for tags that no handler claimed) and suggest the
    // closest known shape alias when the distance is small enough. If no
    // shape alias matches but the document defines `## Style` templates,
    // fall back to suggesting the closest style name — a common silent
    // failure is `{primry}` typo'd from a defined `{primary}` style, where
    // `expand_styles` leaves the tag unresolved and it ends up in
    // `unknown_tags` with no signal to the user.
    //
    // Guards on the style-name fallback:
    //   - Only run when at least one style definition exists.
    //   - Require style-name length >= 3 (shorter names are too ambiguous).
    //   - Skip exact matches (defensive — shouldn't reach unknown_tags).
    //   - Distance <= 2 (length-scaled: <= 1 for <= 4 char names).
    for node in &doc.nodes {
        let is_sticky = matches!(node.kind, crate::model::NodeKind::StickyNote { .. });
        for tag in &node.unknown_tags {
            let id_str = if node.hrf_id.is_empty() {
                node.display_label().to_string()
            } else {
                format!("[{}]", node.hrf_id)
            };
            // Sticky-note color typo: `{blu}` / `{grean}` / `{yelow}` on a
            // `## Notes` entry. Dispatch first so sticky colors are not
            // misclassified as shape aliases (shape vocabulary has zero
            // overlap with sticky colors, but we want the message to say
            // "sticky color" for clarity). Only applies to notes — nodes
            // shouldn't get a sticky-color suggestion.
            if is_sticky {
                if let Some(suggestion) = crate::specgraph::hrf::suggest_sticky_color(tag) {
                    warnings.push(format!(
                        "Note {}: unknown sticky color {{{}}} — did you mean {{{}}}?",
                        id_str, tag, suggestion
                    ));
                    continue;
                }
            }
            if let Some(suggestion) = crate::specgraph::hrf::suggest_shape_alias(tag) {
                warnings.push(format!(
                    "Node {}: unknown tag {{{}}} — did you mean {{{}}}?",
                    id_str, tag, suggestion
                ));
                continue;
            }
            // Property-style shape typo: `{shape:diamon}` / `{type:circel}` /
            // `{kind:hexgon}`. The parser used to silently default these to
            // `RoundedRect` via `tag_to_shape`, wiping user intent. Now the
            // parser pushes these to `unknown_tags` verbatim; strip the
            // prefix and re-dispatch through `suggest_shape_alias` to emit
            // a useful hint that preserves the user's chosen prefix.
            let shape_prefix_match = ["shape:", "type:", "kind:"]
                .iter()
                .find_map(|p| tag.strip_prefix(p).map(|rest| (*p, rest)));
            if let Some((prefix, rest)) = shape_prefix_match {
                let rest_trimmed = rest.trim();
                if let Some(suggestion) = crate::specgraph::hrf::suggest_shape_alias(rest_trimmed) {
                    warnings.push(format!(
                        "Node {}: unknown shape {{{}}} — did you mean {{{}{}}}?",
                        id_str, tag, prefix, suggestion
                    ));
                } else {
                    warnings.push(format!(
                        "Node {}: unknown shape {{{}}} — not a recognized shape alias",
                        id_str, tag
                    ));
                }
                continue;
            }
            // Align/valign typo: `{align:rigth}` / `{valign:middel}`. The
            // parser used to collapse these into the default (`Center` /
            // `Middle`) via a catch-all `_` arm — wiping user intent with
            // zero lint signal. Now parse_node_line pushes unrecognized
            // values to unknown_tags, and `suggest_align_value` points the
            // user at the canonical spelling.
            let align_prefix_match = ["align:", "valign:"]
                .iter()
                .find_map(|p| tag.strip_prefix(p).map(|rest| (*p, rest)));
            if let Some((prefix, rest)) = align_prefix_match {
                let horizontal = prefix == "align:";
                let rest_trimmed = rest.trim();
                if let Some(suggestion) = crate::specgraph::hrf::suggest_align_value(
                    rest_trimmed, horizontal) {
                    warnings.push(format!(
                        "Node {}: unknown alignment {{{}}} — did you mean {{{}{}}}?",
                        id_str, tag, prefix, suggestion
                    ));
                } else {
                    let canon = if horizontal { "left, right, center" } else { "top, bottom, middle" };
                    warnings.push(format!(
                        "Node {}: unknown alignment {{{}}} — expected one of: {}",
                        id_str, tag, canon
                    ));
                }
                continue;
            }
            // Numeric tag silent drop: `{opacity:50%}`, `{border:thick}`,
            // `{w:auto}`, `{x:left}`, etc. These arms used to call
            // `.parse::<f32>().ok()` with no else branch, silently swallowing
            // anything the user wrote. The parser now pushes unresolved
            // values to unknown_tags; this arm surfaces them with a message
            // that names the expected type so the user can fix the input.
            //
            // `opacity:` accepts either a 0.0–1.0 float or a 0–100 number
            // (auto-divided by 100 in the parser); the message reflects both.
            // Order matters: longer prefixes must come before shorter ones
            // (`font-size:`/`text-size:` before `fs:`/`fontsize:`/`textsize:`;
            // `radius:` before `r:`; `border:` before `b:` if that existed).
            let numeric_prefix_match = [
                ("opacity:",   "0.0–1.0 float or 0–100 number"),
                ("alpha:",     "0.0–1.0 float or 0–100 number"),
                ("border:",    "number (border width)"),
                ("font-size:", "number (font size in px)"),
                ("text-size:", "number (font size in px)"),
                ("fontsize:",  "number (font size in px)"),
                ("textsize:",  "number (font size in px)"),
                ("fs:",        "number (font size in px)"),
                ("radius:",    "number (corner radius in px)"),
                ("corner:",    "number (corner radius in px)"),
                ("r:",         "number (corner radius in px)"),
                ("w:",         "number (width)"),
                ("h:",         "number (height)"),
                ("x:",         "number (x coordinate)"),
                ("y:",         "number (y coordinate)"),
                // 3D/depth numeric prefixes. `3d-depth:` listed before
                // `depth:` so the longer prefix wins. Note that
                // `depth-scale:` is a separate config key handled elsewhere
                // — the parser's `depth:` arm already excludes it and so
                // does this lint arm because `depth-scale:` never lands in
                // unknown_tags as a raw `depth:` prefix match.
                ("z:",         "number (z offset for 3D layering)"),
                ("3d-depth:",  "number (3D extrusion depth, 0–400)"),
                ("depth:",     "number (3D extrusion depth, 0–400)"),
                // Progress ring numeric prefixes — accept either 0–100
                // (auto-divided) or 0.0–1.0. Optional trailing `%`.
                ("progress:",  "0–100 number, 0.0–1.0 float, or percent (optional trailing %)"),
                ("percent:",   "0–100 number, 0.0–1.0 float, or percent (optional trailing %)"),
                ("pct:",       "0–100 number, 0.0–1.0 float, or percent (optional trailing %)"),
                // Gradient direction — integer 0–255 (u8) angle in degrees.
                ("gradient-angle:", "integer 0–255 (gradient direction in degrees)"),
                ("grad-angle:",     "integer 0–255 (gradient direction in degrees)"),
            ]
                .iter()
                .find_map(|(p, desc)| tag.strip_prefix(p).map(|_| (*p, *desc)));
            if let Some((prefix, desc)) = numeric_prefix_match {
                warnings.push(format!(
                    "Node {}: unresolved {{{}}} — expected {} after `{}`",
                    id_str, tag, desc, prefix
                ));
                continue;
            }
            // Layer/level/tier named typo: `{layer:databse}` → `{layer:database}`.
            // The parser accepts numeric (`{layer:2}`) OR named semantic tier
            // (`db`/`api`/`frontend`/`edge`/`infra` with synonyms). When the
            // named fallback hit `_ => z_offset`, typos silently left z
            // unchanged. Parser now pushes unresolved to unknown_tags and
            // `suggest_layer_name` points at the closest canonical spelling.
            let layer_prefix_match = ["layer:", "level:", "tier:"]
                .iter()
                .find_map(|p| tag.strip_prefix(p).map(|rest| (*p, rest)));
            if let Some((prefix, rest)) = layer_prefix_match {
                let rest_trimmed = rest.trim();
                if let Some(suggestion) =
                    crate::specgraph::hrf::suggest_layer_name(rest_trimmed)
                {
                    warnings.push(format!(
                        "Node {}: unknown layer {{{}}} — did you mean {{{}{}}}?",
                        id_str, tag, prefix, suggestion
                    ));
                } else {
                    warnings.push(format!(
                        "Node {}: unknown layer {{{}}} — not a number or recognized tier name (db/api/frontend/edge/infra)",
                        id_str, tag
                    ));
                }
                continue;
            }
            // Status typo: `{status:doen}` → `{status:done}`. The parser's
            // `status:` arm used to fall through to `tag_to_node_tag(other)`
            // which returned None for typos, silently dropping the tag.
            // Parser now pushes unresolved status values to unknown_tags;
            // `suggest_status_value` points users at the closest canonical
            // spelling from the shared status vocabulary.
            if let Some(rest) = tag.strip_prefix("status:") {
                let rest_trimmed = rest.trim();
                if let Some(suggestion) =
                    crate::specgraph::hrf::suggest_status_value(rest_trimmed)
                {
                    warnings.push(format!(
                        "Node {}: unknown status {{{}}} — did you mean {{status:{}}}?",
                        id_str, tag, suggestion
                    ));
                } else {
                    warnings.push(format!(
                        "Node {}: unknown status {{{}}} — not a recognized status value",
                        id_str, tag
                    ));
                }
                continue;
            }
            // Shorthand numeric tags: `{size:WxH}` and `{pos:X,Y}`. These
            // expect structured formats (not a single number); a dedicated
            // arm emits a message naming the shape so users see the silent
            // drop on `{size:big}`, `{size:200}` (missing `x`), etc.
            if tag.starts_with("size:") {
                warnings.push(format!(
                    "Node {}: unresolved {{{}}} — expected `{{size:WxH}}` where W and H are numbers",
                    id_str, tag
                ));
                continue;
            }
            if tag.starts_with("pos:") {
                warnings.push(format!(
                    "Node {}: unresolved {{{}}} — expected `{{pos:X,Y}}` where X and Y are numbers",
                    id_str, tag
                ));
                continue;
            }
            // Unresolved color reference: the parser's `fill:`/`color:`/
            // `border-color:`/`stroke:` arms now push tags whose rest does
            // not resolve via `tag_to_fill_color` (palette expansion already
            // passed, built-in color lookup failed, hex parse failed).
            // Suggest the closest palette entry first, then fall back to
            // the built-in color vocabulary.
            // Order matters: longer prefixes must come before shorter ones
            // (`text-color:` before `color:`, `border-color:` before `color:`).
            // `frame-color:`, `frame-fill:`, `bg-color:` target the group
            // frame background; include them so typos surface the same way
            // as fill/border/text color typos. Longer prefixes listed first
            // so `frame-color:` doesn't partial-match `color:`.
            let color_prefix_match = [
                "frame-color:", "frame-fill:", "bg-color:",
                "border-color:", "text-color:",
                "fill:", "stroke:", "color:",
            ]
                .iter()
                .find_map(|p| tag.strip_prefix(p).map(|rest| (*p, rest)));
            if let Some((prefix, rest)) = color_prefix_match {
                let rest_trimmed = rest.trim();
                let rest_lower = rest_trimmed.to_ascii_lowercase();
                // First: try palette entries (if any were defined in ## Palette).
                let mut palette_suggestion: Option<String> = None;
                if !doc.import_hints.palette_definition_usage.is_empty() {
                    let mut best: Option<(String, usize)> = None;
                    for (pal_name, _count) in &doc.import_hints.palette_definition_usage {
                        if pal_name.len() < 3 { continue; }
                        let pn_lower = pal_name.to_ascii_lowercase();
                        if pn_lower == rest_lower { continue; } // defensive
                        let max_d: usize = if pal_name.len() <= 4 { 1 } else { 2 };
                        let d = levenshtein_distance(&rest_lower, &pn_lower);
                        if d == 0 || d > max_d { continue; }
                        match &best {
                            None => best = Some((pal_name.clone(), d)),
                            Some((_, bd)) if d < *bd => best = Some((pal_name.clone(), d)),
                            _ => {}
                        }
                    }
                    palette_suggestion = best.map(|(n, _)| n);
                }
                if let Some(pal_name) = palette_suggestion {
                    warnings.push(format!(
                        "Node {}: unresolved color {{{}}} — did you mean {{{}{}}}? (defined in ## Palette)",
                        id_str, tag, prefix, pal_name
                    ));
                    continue;
                }
                // Fallback: suggest a built-in color name.
                if let Some(color_name) = crate::specgraph::hrf::suggest_fill_color_name(rest_trimmed) {
                    warnings.push(format!(
                        "Node {}: unresolved color {{{}}} — did you mean {{{}{}}}?",
                        id_str, tag, prefix, color_name
                    ));
                    continue;
                }
                // No suggestion — still surface as a generic notice so the
                // user sees that the reference dropped. Without this the
                // silent drop is invisible.
                warnings.push(format!(
                    "Node {}: unresolved color {{{}}} — not a built-in color, hex value, or ## Palette entry",
                    id_str, tag
                ));
                continue;
            }
            // Style-template fallback: match against `## Style` definitions.
            if doc.import_hints.style_definition_usage.is_empty() {
                continue;
            }
            let tag_lower = tag.to_ascii_lowercase();
            let mut best: Option<(String, usize)> = None;
            for (style_name, _count) in &doc.import_hints.style_definition_usage {
                if style_name.len() < 3 { continue; }
                let sn_lower = style_name.to_ascii_lowercase();
                if sn_lower == tag_lower { continue; } // defensive
                let max_d: usize = if style_name.len() <= 4 { 1 } else { 2 };
                let d = levenshtein_distance(&tag_lower, &sn_lower);
                if d == 0 || d > max_d { continue; }
                match &best {
                    None => best = Some((style_name.clone(), d)),
                    Some((_, bd)) if d < *bd => best = Some((style_name.clone(), d)),
                    _ => {}
                }
            }
            if let Some((style_name, _)) = best {
                warnings.push(format!(
                    "Node {}: unknown tag {{{}}} — did you mean {{{}}}? (defined in ## Style)",
                    id_str, tag, style_name
                ));
            }
        }
    }

    // Edge-style tag typo detection: mirror the node check on edges, using
    // the edge-style vocabulary (dashed/dotted/thick/ortho/escalate/...). Also
    // runs `suggest_arrow_style` for `arrow:*` sub-tags, which the bare-word
    // suggestor explicitly skips. Also handles `color:X` silent drops by
    // falling back to `suggest_fill_color_name` (edge color vocab is a
    // subset of fill vocab).
    for edge in &doc.edges {
        // Build edge label once, reused across warnings.
        let edge_label = {
            if edge.label.trim().is_empty() {
                let src = doc.nodes.iter().find(|n| n.id == edge.source.node_id)
                    .map(|n| if n.hrf_id.is_empty() { n.display_label().to_string() } else { format!("[{}]", n.hrf_id) })
                    .unwrap_or_else(|| "?".into());
                let tgt = doc.nodes.iter().find(|n| n.id == edge.target.node_id)
                    .map(|n| if n.hrf_id.is_empty() { n.display_label().to_string() } else { format!("[{}]", n.hrf_id) })
                    .unwrap_or_else(|| "?".into());
                format!("{} → {}", src, tgt)
            } else {
                format!("{:?}", edge.label)
            }
        };
        for tag in &edge.unknown_tags {
            // Edge color typo: unresolved `{color:X}` where X is neither a
            // built-in edge color nor a hex. Previously silent drop.
            if let Some(rest) = tag.strip_prefix("color:") {
                let rest_trimmed = rest.trim();
                if rest_trimmed.starts_with('#') {
                    // Malformed hex — still warn so the user sees the drop.
                    warnings.push(format!(
                        "Edge {}: unresolved color {{{}}} — invalid hex value",
                        edge_label, tag
                    ));
                    continue;
                }
                if let Some(color_name) = crate::specgraph::hrf::suggest_fill_color_name(rest_trimmed) {
                    warnings.push(format!(
                        "Edge {}: unresolved color {{{}}} — did you mean {{color:{}}}?",
                        edge_label, tag, color_name
                    ));
                } else {
                    warnings.push(format!(
                        "Edge {}: unresolved color {{{}}} — not a built-in edge color or hex value",
                        edge_label, tag
                    ));
                }
                continue;
            }
            // Edge numeric silent-drop detection: `{bend:X}`, `{weight:X}`,
            // and `{w:X}` each previously parsed with `.ok()` and silently
            // dropped non-numeric input. The parser now preserves the raw
            // tag; surface it with a targeted message so the user knows
            // which field was malformed.
            let edge_numeric_match = [
                ("bend:",   "number in -1.0..=1.0 (curve bend)"),
                ("weight:", "number (edge weight)"),
                ("w:",      "number (edge weight shorthand)"),
            ]
                .iter()
                .find_map(|(p, desc)| tag.strip_prefix(p).map(|_| (*p, *desc)));
            if let Some((prefix, desc)) = edge_numeric_match {
                warnings.push(format!(
                    "Edge {}: unresolved {{{}}} — expected {} after `{}`",
                    edge_label, tag, desc, prefix
                ));
                continue;
            }
            // Cardinality typo detection: `{c-src:1..Z}` and `{c-tgt:foo}`
            // previously parsed via `parse_cardinality` which silently
            // collapsed unknown spellings to `Cardinality::None`. The
            // parser now preserves unresolved values in unknown_tags;
            // surface them with a did-you-mean suggestion from the
            // canonical vocabulary (`1`, `0..1`, `1..N`, `0..N`).
            let cardinality_prefix_match = ["c-src:", "c-tgt:"]
                .iter()
                .find_map(|p| tag.strip_prefix(p).map(|rest| (*p, rest)));
            if let Some((prefix, rest)) = cardinality_prefix_match {
                let trimmed = rest.trim();
                if let Some(suggestion) =
                    crate::specgraph::hrf::suggest_cardinality_value(trimmed)
                {
                    warnings.push(format!(
                        "Edge {}: unknown cardinality {{{}}} — did you mean {{{}{}}}?",
                        edge_label, tag, prefix, suggestion
                    ));
                } else {
                    warnings.push(format!(
                        "Edge {}: unknown cardinality {{{}}} — expected 1, 0..1, 1..N, or 0..N",
                        edge_label, tag
                    ));
                }
                continue;
            }
            let suggestion = crate::specgraph::hrf::suggest_edge_style_alias(tag)
                .or_else(|| crate::specgraph::hrf::suggest_arrow_style(tag));
            if let Some(suggestion) = suggestion {
                warnings.push(format!(
                    "Edge {}: unknown tag {{{}}} — did you mean {{{}}}?",
                    edge_label, tag, suggestion
                ));
            }
        }
    }

    // Config directive typo detection: walk keys the parser couldn't place into
    // any known arm (`tilte = "My Doc"`, `flwo = LR`, ...) and suggest the
    // closest canonical key. Unrecognized keys are dropped silently by the
    // parser — this lint is the user's only signal that a directive no-op'd.
    for key in &doc.import_hints.unknown_config_keys {
        if let Some(suggestion) = crate::specgraph::hrf::suggest_config_key(key) {
            warnings.push(format!(
                "Config: unknown key `{}` — did you mean `{}`?",
                key, suggestion
            ));
        } else {
            warnings.push(format!(
                "Config: unknown key `{}` — directive ignored",
                key
            ));
        }
    }

    // Orphaned group member detection: the `## Groups` section lets you
    // list member IDs that must resolve to nodes in `## Nodes`. When they
    // don't, the parser silently drops the unresolved member and the group
    // frame just skips them — the user gets zero feedback. Surface those
    // as lint warnings, with a "did you mean" suggestion from the real HRF
    // ids. This catches copy-paste typos like `api_service` vs `api-service`
    // and stale references to deleted nodes.
    if !doc.import_hints.unresolved_group_members.is_empty() {
        let known_ids: Vec<&str> = doc
            .nodes
            .iter()
            .filter(|n| !n.hrf_id.is_empty())
            .map(|n| n.hrf_id.as_str())
            .collect();
        for (group_id, member_id) in &doc.import_hints.unresolved_group_members {
            let suggestion =
                crate::specgraph::hrf::suggest_node_id_from_candidates(member_id, &known_ids);
            match suggestion {
                Some(s) => warnings.push(format!(
                    "Group [{}]: member `{}` does not exist — did you mean `{}`?",
                    group_id, member_id, s
                )),
                None => warnings.push(format!(
                    "Group [{}]: member `{}` does not exist in ## Nodes",
                    group_id, member_id
                )),
            }
        }
    }

    // `{dep:X}` unresolved targets: when the dep target doesn't resolve to
    // any existing node by id or slugified label, the parser previously
    // silent-dropped the dependency edge at `Resolve {dep:target}` in
    // hrf.rs with no warning. Typos like `{dep:authservce}` → vanished.
    // The parser now records (source_id, raw_target) here. Emit a lint
    // warning with did-you-mean hints drawn from the same known-id
    // vocabulary used for unresolved group members.
    if !doc.import_hints.unresolved_dep_targets.is_empty() {
        let known_ids: Vec<&str> = doc
            .nodes
            .iter()
            .filter(|n| !n.hrf_id.is_empty())
            .map(|n| n.hrf_id.as_str())
            .collect();
        for (src_id, dep_target) in &doc.import_hints.unresolved_dep_targets {
            let src_display = if src_id.is_empty() { "?" } else { src_id.as_str() };
            let suggestion =
                crate::specgraph::hrf::suggest_node_id_from_candidates(dep_target, &known_ids);
            match suggestion {
                Some(s) => warnings.push(format!(
                    "Node [{}]: {{dep:{}}} does not resolve — did you mean `{{dep:{}}}`?",
                    src_display, dep_target, s
                )),
                None => warnings.push(format!(
                    "Node [{}]: {{dep:{}}} does not resolve to any node in ## Nodes",
                    src_display, dep_target
                )),
            }
        }
    }

    // `{lane:X}` unresolved references: when a node tag like `{lane:Enginering}`
    // doesn't match any declared lane from `## Swimlane:` / `## Lane N:` /
    // `## Kanban:` / `## Swimlanes` list, layout.rs silently auto-creates a
    // phantom empty lane with the typo name — the user sees an extra column/row
    // but no feedback about the mistake. The parser now records
    // (source_id, raw_lane_name) into `unresolved_lane_refs` whenever at least
    // one lane was explicitly declared. Emit did-you-mean hints drawn from the
    // declared lane vocabulary. Candidates are `doc.timeline_lanes` minus the
    // set of typo'd names (so typos don't self-suggest).
    if !doc.import_hints.unresolved_lane_refs.is_empty() {
        let typo_set: std::collections::HashSet<&str> = doc
            .import_hints
            .unresolved_lane_refs
            .iter()
            .map(|(_, n)| n.as_str())
            .collect();
        let declared_lanes: Vec<&str> = doc
            .timeline_lanes
            .iter()
            .map(String::as_str)
            .filter(|s| !typo_set.contains(*s))
            .collect();
        for (src_id, lane_name) in &doc.import_hints.unresolved_lane_refs {
            let src_display = if src_id.is_empty() { "?" } else { src_id.as_str() };
            let suggestion =
                crate::specgraph::hrf::suggest_node_id_from_candidates(lane_name, &declared_lanes);
            match suggestion {
                Some(s) => warnings.push(format!(
                    "Node [{}]: {{lane:{}}} does not match any declared lane — did you mean `{{lane:{}}}`?",
                    src_display, lane_name, s
                )),
                None => warnings.push(format!(
                    "Node [{}]: {{lane:{}}} does not match any declared lane in ## Swimlane/## Lane/## Kanban sections",
                    src_display, lane_name
                )),
            }
        }
    }

    // `{phase:X}` / `{period:X}` unresolved references: when a node tag doesn't
    // match any declared period in the ## Timeline section, layout.rs drops the
    // node into an "unperioded" bucket far below the grid (effectively vanishes
    // off-canvas). Completely silent before this lint. The parser now records
    // (source_id, raw_period_name) into `unresolved_period_refs` whenever the
    // ## Timeline section declared at least one period. `doc.timeline_periods`
    // is only populated from explicit declaration so can be used directly as
    // the candidate vocabulary.
    if !doc.import_hints.unresolved_period_refs.is_empty() {
        let declared_periods: Vec<&str> = doc
            .timeline_periods
            .iter()
            .map(String::as_str)
            .collect();
        for (src_id, period_name) in &doc.import_hints.unresolved_period_refs {
            let src_display = if src_id.is_empty() { "?" } else { src_id.as_str() };
            let suggestion = crate::specgraph::hrf::suggest_node_id_from_candidates(
                period_name,
                &declared_periods,
            );
            match suggestion {
                Some(s) => warnings.push(format!(
                    "Node [{}]: {{phase:{}}} does not match any declared period — did you mean `{{phase:{}}}`?",
                    src_display, period_name, s
                )),
                None => warnings.push(format!(
                    "Node [{}]: {{phase:{}}} does not match any declared period in ## Timeline",
                    src_display, period_name
                )),
            }
        }
    }

    // `## Groups` fill typos: when `{fill:X}` on a group line doesn't resolve
    // (`{fill:blu}`, `{fill:gren}`, bad hex), the frame silently falls back
    // to the default color with no feedback. The parser now records these
    // into `import_hints.unknown_group_fill`. Emit did-you-mean hints via
    // `suggest_fill_color_name` so typos are actionable instead of silent.
    for (group_id, raw_tag) in &doc.import_hints.unknown_group_fill {
        let val = raw_tag.strip_prefix("fill:").unwrap_or(raw_tag).trim();
        if let Some(suggestion) = crate::specgraph::hrf::suggest_fill_color_name(val) {
            warnings.push(format!(
                "Group [{}]: unresolved {{{}}} — did you mean {{fill:{}}}?",
                group_id, raw_tag, suggestion
            ));
        } else {
            warnings.push(format!(
                "Group [{}]: unresolved {{{}}} — not a recognized color name (blue/green/red/...), palette entry, or hex value",
                group_id, raw_tag
            ));
        }
    }

    // Inline group typo-split detection: when `{group:backend}` appears on
    // several nodes and `{group:bakcend}` on one, the parser creates two
    // frames silently. Detect pairs of inline group names that are within
    // Levenshtein distance 2 and flag the minority spelling as a likely typo
    // of the majority. Only reports when at least one of the two has a
    // single member — typical typo split signature. Skips when both names
    // have 2+ members (probably intentionally separate groups).
    {
        let names = &doc.import_hints.inline_group_name_counts;
        if names.len() >= 2 {
            for i in 0..names.len() {
                for j in (i + 1)..names.len() {
                    let (a, ac) = (&names[i].0, names[i].1);
                    let (b, bc) = (&names[j].0, names[j].1);
                    // Both 2+ members = likely intentional separate groups.
                    if ac >= 2 && bc >= 2 { continue; }
                    // Identify minority and majority for a clearer message.
                    let (majority, maj_count, minority, min_count) =
                        if ac > bc { (a, ac, b, bc) } else { (b, bc, a, ac) };
                    // If both are equal counts (both 1), still flag — could
                    // be two 1-node groups that should have been one.
                    let d = levenshtein_distance(&minority.to_ascii_lowercase(),
                                                 &majority.to_ascii_lowercase());
                    // Require names of length ≥ 4 so 3-letter acronyms don't
                    // attract noise (e.g. `api` vs `ipa`).
                    let min_len = minority.len().min(majority.len());
                    if min_len >= 4 && d > 0 && d <= 2 {
                        warnings.push(format!(
                            "Inline group `{}` ({} node{}) looks like a typo of \
                            `{}` ({} node{}) — check for typo-split",
                            minority,
                            min_count,
                            if min_count == 1 { "" } else { "s" },
                            majority,
                            maj_count,
                            if maj_count == 1 { "" } else { "s" },
                        ));
                    }
                }
            }
        }
    }

    // Inline lane / phase / section typo-split detection (mirrors the
    // group-level check above, one kind at a time). The parser stores
    // `{lane:X}` / `{section:X}` / `{col:X}` in `node.timeline_lane`
    // and `{phase:X}` in `node.timeline_period`. A user typing
    // `{lane:Sales}` × 3 and `{lane:Slaes}` × 1 silently creates two
    // lanes with no warning. Emit a typo-split hint when the minority
    // is within Levenshtein 2 of the majority.
    //
    // Kind selection:
    //   - timeline_lane: kanban/swimlane membership
    //   - timeline_period: timeline phase membership
    // We only report when either side has count 1 (same rule as
    // inline groups) and the name is ≥4 chars to suppress 3-letter
    // acronym noise (e.g. "Sales" vs "Sacks").
    {
        for kind in &["lane", "phase"] {
            let mut counts_map: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for node in &doc.nodes {
                if node.is_frame { continue; }
                let val_opt = if *kind == "lane" {
                    node.timeline_lane.clone()
                } else {
                    node.timeline_period.clone()
                };
                if let Some(v) = val_opt {
                    let trimmed = v.trim();
                    if !trimmed.is_empty() {
                        *counts_map.entry(trimmed.to_string()).or_insert(0) += 1;
                    }
                }
            }
            let mut counts: Vec<(String, usize)> = counts_map.into_iter().collect();
            counts.sort_by(|a, b| a.0.cmp(&b.0));
            if counts.len() < 2 { continue; }
            for i in 0..counts.len() {
                for j in (i + 1)..counts.len() {
                    let (a, ac) = (&counts[i].0, counts[i].1);
                    let (b, bc) = (&counts[j].0, counts[j].1);
                    if ac >= 2 && bc >= 2 { continue; }
                    let (majority, maj_count, minority, min_count) =
                        if ac > bc { (a, ac, b, bc) } else { (b, bc, a, ac) };
                    let d = levenshtein_distance(
                        &minority.to_ascii_lowercase(),
                        &majority.to_ascii_lowercase(),
                    );
                    let min_len = minority.len().min(majority.len());
                    if min_len >= 4 && d > 0 && d <= 2 {
                        warnings.push(format!(
                            "Inline {kind} `{}` ({} node{}) looks like a typo of \
                            `{}` ({} node{}) — check for typo-split",
                            minority,
                            min_count,
                            if min_count == 1 { "" } else { "s" },
                            majority,
                            maj_count,
                            if maj_count == 1 { "" } else { "s" },
                        ));
                    }
                }
            }
        }
    }

    // Check for empty labels
    for node in &doc.nodes {
        let label = node.display_label();
        if label.trim().is_empty() && !node.is_frame {
            let id_str = if node.hrf_id.is_empty() { node.id.0.to_string() } else { node.hrf_id.clone() };
            warnings.push(format!("Node [{}] has an empty label", id_str));
        }
    }

    // Check for disconnected nodes (not frames)
    let mut connected: std::collections::HashSet<crate::model::NodeId> = std::collections::HashSet::new();
    for edge in &doc.edges {
        connected.insert(edge.source.node_id);
        connected.insert(edge.target.node_id);
    }
    for node in &doc.nodes {
        if !node.is_frame && !connected.contains(&node.id) && doc.nodes.len() > 1 {
            warnings.push(format!("Node {:?} is disconnected (no edges)", node.display_label()));
        }
    }

    // Check for duplicate HRF IDs
    let mut seen_ids: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for node in &doc.nodes {
        if !node.hrf_id.is_empty() {
            *seen_ids.entry(node.hrf_id.as_str()).or_default() += 1;
        }
    }
    for (id, count) in &seen_ids {
        if *count > 1 {
            errors.push(format!("Duplicate HRF ID [{}] used {} times", id, count));
        }
    }

    // Near-duplicate HRF IDs: two IDs within Levenshtein distance 1 are
    // almost always a typo split where the user meant one ID but spelled
    // it two ways (e.g. `order` vs `oder`, `event` vs `evnet`). Currently
    // the parser silently creates two separate nodes and the `## Flow`
    // edges targeting one of them vanish. Surface the pair as a warning.
    //
    // Aggressive noise filtering (false-positives are worse than misses):
    //   - Only d == 1 (d == 2 flags too many unrelated pairs like kr_g1/kr_o1).
    //   - Min length >= 5 (shorter IDs like `user`/`uses` are too ambiguous).
    //   - Skip numbered-series pairs with identical stems (`svc1`/`svc2`,
    //     `feat1`/`feat3`) — detected by stripping trailing digits/underscores.
    //   - Skip when either ID contains a digit AND the other does too
    //     (enumeration like `kr_g1`/`kr_o1` where the difference is in the
    //     label prefix, not a typo).
    //   - Skip when one is a prefix of the other (`stream` vs `streams`,
    //     `capture` vs `captured`, `authorize` vs `authorized` — these are
    //     inflectional variants that legitimately coexist in state machines).
    {
        fn strip_trailing_digits(s: &str) -> String {
            let mut end = s.len();
            let bytes = s.as_bytes();
            while end > 0 && bytes[end - 1].is_ascii_digit() {
                end -= 1;
            }
            // Also strip a trailing separator like `_` or `-` if present.
            while end > 0 && (bytes[end - 1] == b'_' || bytes[end - 1] == b'-') {
                end -= 1;
            }
            s[..end].to_string()
        }

        let mut unique_ids: Vec<&str> = doc
            .nodes
            .iter()
            .filter(|n| !n.hrf_id.is_empty())
            .map(|n| n.hrf_id.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        unique_ids.sort();
        let mut flagged: Vec<(String, String)> = Vec::new();
        for i in 0..unique_ids.len() {
            for j in (i + 1)..unique_ids.len() {
                let a = unique_ids[i];
                let b = unique_ids[j];
                let min_len = a.len().min(b.len());
                if min_len < 4 { continue; }
                // Both contain a digit → probably numbered enumeration.
                let a_has_digit = a.bytes().any(|c| c.is_ascii_digit());
                let b_has_digit = b.bytes().any(|c| c.is_ascii_digit());
                if a_has_digit && b_has_digit { continue; }
                // Same stem after digit stripping → numbered series.
                let a_stem = strip_trailing_digits(a).to_ascii_lowercase();
                let b_stem = strip_trailing_digits(b).to_ascii_lowercase();
                if a_stem == b_stem { continue; }
                // Prefix relationship on stems → inflectional variant like
                // `stream1`/`streams` (stems `stream`/`streams`) or
                // `capture`/`captured` (stems match as prefix).
                if !a_stem.is_empty() && !b_stem.is_empty()
                    && (a_stem.starts_with(&b_stem) || b_stem.starts_with(&a_stem))
                {
                    continue;
                }
                let a_lower = a.to_ascii_lowercase();
                let b_lower = b.to_ascii_lowercase();
                let d = levenshtein_distance(&a_lower, &b_lower);
                if d == 1 {
                    flagged.push((a.to_string(), b.to_string()));
                }
            }
        }
        for (a, b) in flagged {
            warnings.push(format!(
                "HRF IDs [{}] and [{}] look like typo variants — likely one ID misspelled (check ## Flow references)",
                a, b
            ));
        }
    }

    // Check for self-loops. A single self-loop is often intentional
    // (state retention, event handler loopback). Two or more on the same
    // node is almost always a copy-paste typo — aggregate them into one
    // "multiple" warning to make the count obvious.
    {
        use std::collections::HashMap;
        let mut self_counts: HashMap<crate::model::NodeId, usize> = HashMap::new();
        for edge in &doc.edges {
            if edge.source.node_id == edge.target.node_id {
                *self_counts.entry(edge.source.node_id).or_insert(0) += 1;
            }
        }
        // Sort by node's insertion order for deterministic output.
        let mut ordered: Vec<(crate::model::NodeId, usize)> = Vec::new();
        for node in &doc.nodes {
            if let Some(&count) = self_counts.get(&node.id) {
                ordered.push((node.id, count));
            }
        }
        for (nid, count) in ordered {
            let label = doc.nodes.iter().find(|n| n.id == nid)
                .map(|n| n.display_label().to_string())
                .unwrap_or_else(|| "?".into());
            if count == 1 {
                warnings.push(format!("Self-loop on node {:?}", label));
            } else {
                warnings.push(format!(
                    "Node {:?} has {} self-loops — almost always a copy-paste typo (a single loop covers most retention/loopback semantics)",
                    label, count
                ));
            }
        }
    }

    // Check for very high degree nodes (potential hub bottleneck)
    let mut degrees: std::collections::HashMap<crate::model::NodeId, usize> = std::collections::HashMap::new();
    for edge in &doc.edges {
        *degrees.entry(edge.source.node_id).or_default() += 1;
        *degrees.entry(edge.target.node_id).or_default() += 1;
    }
    for (nid, deg) in &degrees {
        if *deg > 10 {
            let label = doc.nodes.iter().find(|n| n.id == *nid)
                .map(|n| n.display_label().to_string())
                .unwrap_or_else(|| "?".into());
            warnings.push(format!("Node {:?} has {} connections (consider splitting)", label, deg));
        }
    }

    // Check for edges referencing missing nodes
    let node_ids: std::collections::HashSet<crate::model::NodeId> = doc.nodes.iter().map(|n| n.id).collect();
    for edge in &doc.edges {
        if !node_ids.contains(&edge.source.node_id) {
            errors.push(format!("Edge source references missing node {}", edge.source.node_id.0));
        }
        if !node_ids.contains(&edge.target.node_id) {
            errors.push(format!("Edge target references missing node {}", edge.target.node_id.0));
        }
    }

    // Check for overlapping nodes (same position, similar size)
    for i in 0..doc.nodes.len() {
        for j in (i + 1)..doc.nodes.len() {
            let a = &doc.nodes[i];
            let b = &doc.nodes[j];
            if a.is_frame || b.is_frame { continue; }
            let dx = (a.position[0] - b.position[0]).abs();
            let dy = (a.position[1] - b.position[1]).abs();
            if dx < 5.0 && dy < 5.0 {
                warnings.push(format!("Nodes {:?} and {:?} overlap at same position",
                    a.display_label(), b.display_label()));
            }
        }
    }

    // Check for very long labels (readability)
    for node in &doc.nodes {
        if node.is_frame { continue; }
        let label_chars = node.display_label().chars().count();
        if label_chars > 60 {
            warnings.push(format!(
                "Node {:?} has a very long label ({} chars) — consider shortening or splitting",
                node.display_label(), label_chars
            ));
        }
    }

    // Check for unlabeled outgoing edges from decision (diamond) nodes.
    // Decisions branch, so the branches should be labeled (e.g. yes/no).
    {
        use crate::model::{NodeKind, NodeShape};
        let diamond_ids: std::collections::HashSet<crate::model::NodeId> = doc
            .nodes
            .iter()
            .filter(|n| matches!(&n.kind, NodeKind::Shape { shape: NodeShape::Diamond, .. }))
            .map(|n| n.id)
            .collect();
        if !diamond_ids.is_empty() {
            // Group by source node to count outgoing branches
            let mut out_counts: std::collections::HashMap<crate::model::NodeId, (usize, usize)> =
                std::collections::HashMap::new();
            for edge in &doc.edges {
                if diamond_ids.contains(&edge.source.node_id) {
                    let entry = out_counts.entry(edge.source.node_id).or_insert((0, 0));
                    entry.0 += 1;
                    if !edge.label.trim().is_empty() { entry.1 += 1; }
                }
            }
            for (nid, (total, labeled)) in &out_counts {
                if *total >= 2 && *labeled < *total {
                    let label = doc.nodes.iter().find(|n| n.id == *nid)
                        .map(|n| n.display_label().to_string())
                        .unwrap_or_else(|| "?".into());
                    warnings.push(format!(
                        "Decision node {:?} has {} branches but only {} are labeled",
                        label, total, labeled
                    ));
                }
            }
        }
    }

    // Check for diamonds with fewer than 2 branches — a decision with only
    // one outgoing edge isn't really a decision. Likely should be a RoundedRect.
    {
        use crate::model::{NodeKind, NodeShape};
        let mut out_counts: std::collections::HashMap<crate::model::NodeId, usize> =
            std::collections::HashMap::new();
        for edge in &doc.edges {
            *out_counts.entry(edge.source.node_id).or_default() += 1;
        }
        for node in &doc.nodes {
            if matches!(&node.kind, NodeKind::Shape { shape: NodeShape::Diamond, .. }) {
                let out = out_counts.get(&node.id).copied().unwrap_or(0);
                if out < 2 {
                    warnings.push(format!(
                        "Diamond node {:?} has only {} outgoing edge(s) — decisions should have 2+ branches (use rounded rect for simple steps)",
                        node.display_label(), out
                    ));
                }
            }
        }
    }

    // Check for weakly-connected components. If a diagram has multiple
    // disconnected subgraphs, it's usually a sign of missing edges.
    {
        use crate::model::NodeId;
        let mut adj: std::collections::HashMap<NodeId, Vec<NodeId>> =
            std::collections::HashMap::new();
        for node in &doc.nodes {
            if !node.is_frame {
                adj.entry(node.id).or_default();
            }
        }
        for edge in &doc.edges {
            adj.entry(edge.source.node_id).or_default().push(edge.target.node_id);
            adj.entry(edge.target.node_id).or_default().push(edge.source.node_id);
        }
        let mut visited: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        let mut component_count = 0usize;
        for &start in adj.keys() {
            if visited.contains(&start) { continue; }
            component_count += 1;
            let mut stack = vec![start];
            while let Some(cur) = stack.pop() {
                if !visited.insert(cur) { continue; }
                if let Some(nbrs) = adj.get(&cur) {
                    for &n in nbrs { stack.push(n); }
                }
            }
        }
        if component_count > 1 && doc.nodes.iter().filter(|n| !n.is_frame).count() >= 3 {
            warnings.push(format!(
                "Graph has {} disconnected subgraphs — consider linking related components",
                component_count
            ));
        }
    }

    // Unused `## Style` definitions: the parser's pre-scan tracks every
    // style-template definition and counts how many times it's expanded
    // (`{primary}` → body). A definition with 0 expansions is dead code —
    // either a typo in the node-side reference, or a leftover from refactoring.
    // Silent failure mode: the style block still parses, just does nothing.
    for (name, count) in &doc.import_hints.style_definition_usage {
        if *count == 0 {
            warnings.push(format!(
                "Style `{}` is defined in ## Style but never referenced — dead code",
                name
            ));
        }
    }

    // Unused `## Palette` entries: same pattern as unused styles above, but
    // for color names. A palette entry with 0 expansions across `{fill:}`,
    // `{color:}`, `{border-color:}`, and `{stroke:}` is dead code. Common
    // post-refactor leftover — users rename "primary" to "accent", update
    // references, forget to update the palette definition.
    for (name, count) in &doc.import_hints.palette_definition_usage {
        if *count == 0 {
            warnings.push(format!(
                "Palette color `{}` is defined in ## Palette but never referenced — dead code",
                name
            ));
        }
    }

    // Unknown `flow = X` / `layout = X` config values: the parser used to
    // silently default to TB when the direction was unrecognized
    // (`flow = TR` typo → TB, no warning). Now records the raw value so
    // we can emit a did-you-mean hint. Frequent typo mode: users mix up
    // LR/RL and TB/BT constantly.
    for (raw, suggestion) in &doc.import_hints.unknown_layout_direction {
        warnings.push(format!(
            "Unknown layout direction `{}` — did you mean `{}`? (valid: TB, BT, LR, RL)",
            raw, suggestion
        ));
    }

    // Unknown `timeline-dir = X` / `timeline_dir = X` config values: the
    // parser used `_ => "LR"` fallthrough, so typos like `virtical` (→ TB)
    // silently became LR with no feedback. Now recorded as (raw, canonical)
    // pairs; canonical is clamped to the timeline-accepted TB/LR subset.
    for (raw, suggestion) in &doc.import_hints.unknown_timeline_dir {
        warnings.push(format!(
            "Unknown timeline direction `{}` — did you mean `{}`? (valid: TB, LR)",
            raw, suggestion
        ));
    }

    // Invalid `{src-port:X}` / `{tgt-port:X}` values: unknown values used
    // to silently fall back to the default Bottom/Top port, causing edges
    // to connect at the wrong attachment point without any warning. Now
    // records the raw value + kind so we can emit a did-you-mean hint.
    {
        use crate::specgraph::hrf::suggest_port_side;
        for (kind, raw) in &doc.import_hints.invalid_port_side_values {
            let kind_desc = if kind == "src" { "source" } else { "target" };
            let suggestion = suggest_port_side(raw).unwrap_or("top");
            warnings.push(format!(
                "Invalid {} port side `{}` — did you mean `{}`? (valid: top, bottom, left, right)",
                kind_desc, raw, suggestion
            ));
        }
    }

    // Unknown `camera = X` preset values: the parser used to silently
    // leave camera_yaw/pitch unchanged on unrecognized preset names, so
    // typos like `camera = ios` (for `iso`) or `camera = isometrci`
    // vanished without a warning and the view stayed at the default.
    // Now records the raw value so we can emit did-you-mean hints from
    // the canonical vocabulary.
    for raw in &doc.import_hints.unknown_camera_preset {
        if let Some(suggestion) = crate::specgraph::hrf::suggest_camera_preset(raw) {
            warnings.push(format!(
                "Config: unknown camera preset `{}` — did you mean `{}`?",
                raw, suggestion
            ));
        } else {
            warnings.push(format!(
                "Config: unknown camera preset `{}` — expected iso, top, front, or side",
                raw
            ));
        }
    }

    // Unknown boolean / view-mode config values: `timeline = tru`,
    // `auto-z = ye`, `view = threedd` all used to silently fall through
    // `_ => {}` arms leaving the flag off. The parser now records the
    // key + raw value; pick the right suggestor based on the key name
    // (view → 2d/3d vocabulary; everything else → boolean vocabulary).
    for (key, raw) in &doc.import_hints.unknown_bool_config {
        let is_view = matches!(key.as_str(), "view" | "view-mode" | "mode");
        let suggestion = if is_view {
            crate::specgraph::hrf::suggest_view_mode(raw)
        } else {
            crate::specgraph::hrf::suggest_bool_value(raw)
        };
        if let Some(s) = suggestion {
            warnings.push(format!(
                "Config: `{} = {}` not recognized — did you mean `{} = {}`?",
                key, raw, key, s
            ));
        } else if is_view {
            warnings.push(format!(
                "Config: `{} = {}` not recognized — expected 2d or 3d",
                key, raw
            ));
        } else {
            warnings.push(format!(
                "Config: `{} = {}` not recognized — expected true/false, yes/no, on/off, or 1/0",
                key, raw
            ));
        }
    }

    // Unknown canvas background color: `bg-color = primry` or
    // `background = drk` used to silently leave canvas_bg unset. The parser
    // now records (key, raw_value) on unresolved values. Emit a did-you-mean
    // hint when `suggest_fill_color_name` finds a close built-in; otherwise
    // list the accepted vocabulary so the user knows what the field takes.
    for (key, raw) in &doc.import_hints.unknown_canvas_bg {
        if let Some(suggestion) = crate::specgraph::hrf::suggest_fill_color_name(raw) {
            warnings.push(format!(
                "Config: `{} = {}` not a recognized color — did you mean `{} = {}`?",
                key, raw, key, suggestion
            ));
        } else {
            warnings.push(format!(
                "Config: `{} = {}` not a recognized color — expected a hex value (#rrggbb) or built-in name (blue, surface, black, ...)",
                key, raw
            ));
        }
    }

    // Unknown bg-pattern values: `bg = dts` (meant: dots) or
    // `bg-pattern = crosshach` used to be stored verbatim into
    // `import_hints.bg_pattern` and then silently fall back to
    // `BgPattern::None` (or the previous pattern) in the toolbar's
    // application code. The parser now validates against the accepted
    // vocabulary and preserves unresolved values here. Emit a did-you-mean
    // hint from `suggest_bg_pattern` when there's a close canonical name,
    // otherwise list the accepted vocabulary.
    for (key, raw) in &doc.import_hints.unknown_bg_pattern {
        if let Some(suggestion) = crate::specgraph::hrf::suggest_bg_pattern(raw) {
            warnings.push(format!(
                "Config: `{} = {}` not a recognized pattern — did you mean `{} = {}`?",
                key, raw, key, suggestion
            ));
        } else {
            warnings.push(format!(
                "Config: `{} = {}` not a recognized pattern — expected one of: dots, lines, crosshatch, none",
                key, raw
            ));
        }
    }

    // Unknown numeric config values: `grid = small`, `camera_yaw = tilted`,
    // `gap = wide`, `sla-p1 = three` used to silently drop because the parser
    // did `if let Ok(v) = val.parse::<f32>()` with no else branch. The parser
    // now records (key, raw_value) when parse fails and the value is
    // non-empty. Surface a clear warning so the user knows their directive
    // was ignored.
    for (key, raw) in &doc.import_hints.unknown_numeric_config {
        warnings.push(format!(
            "Config: `{} = {}` is not a number — this directive was ignored",
            key, raw
        ));
    }

    // Unresolved `## Layers` value assignments: values that weren't
    // parseable as numbers AND didn't match any canonical tier name
    // (db/app/ui/edge/infra + aliases). Typos like `ui = 24o` (meant:
    // 240) or `api = backned` (meant: backend) used to silently drop
    // from the layer map, leaving `{layer:X}` / `{tier:X}` references
    // unexpanded in the rendered document. Emit a did-you-mean hint
    // from `suggest_layer_tier_name` when close to a canonical tier,
    // otherwise advertise the accepted vocabulary.
    for (name, raw) in &doc.import_hints.unknown_layer_values {
        if let Some(suggestion) = crate::specgraph::hrf::suggest_layer_tier_name(raw) {
            warnings.push(format!(
                "## Layers: `{} = {}` is not a number or known tier — did you mean `{} = {}`?",
                name, raw, name, suggestion
            ));
        } else {
            warnings.push(format!(
                "## Layers: `{} = {}` is not a number or known tier — expected a number (z offset) or one of: db, app, ui, edge, infra",
                name, raw
            ));
        }
    }

    // Unresolved `## Palette` entries: values that were neither a canonical
    // color name nor a valid hex. Typos like `accent = primray` (meant:
    // purple) or `brand = reed` (meant: red) used to silently drop from the
    // palette map, so any later `{fill:accent}` / `{color:brand}` references
    // also silent-fell-through downstream. Emit a did-you-mean hint from
    // `suggest_fill_color_name` when close to a canonical name, otherwise
    // advertise that the value must be a color name or hex.
    for (name, raw) in &doc.import_hints.unknown_palette_values {
        // Hex prefix gets its own message — the user clearly intended a hex
        // value, suggesting a named color would be unhelpful.
        if raw.starts_with('#') {
            warnings.push(format!(
                "## Palette: `{} = {}` is not a valid hex color — expected `#rgb`, `#rrggbb`, or `#rrggbbaa`",
                name, raw
            ));
        } else if let Some(suggestion) = crate::specgraph::hrf::suggest_fill_color_name(raw) {
            warnings.push(format!(
                "## Palette: `{} = {}` is not a recognized color — did you mean `{} = {}`?",
                name, raw, name, suggestion
            ));
        } else {
            warnings.push(format!(
                "## Palette: `{} = {}` is not a recognized color — expected a color name (blue, green, red, ...) or a hex value (#rrggbb)",
                name, raw
            ));
        }
    }

    // Unresolved `## Grid` / `## Matrix` / `## Table` `cols=` header values.
    // The parser falls back to `cols=3` on any parse failure — a user who
    // typed `## Grid cols=fve` (meant: 5) silently gets a 3-column grid
    // with no indication their value was dropped. Emit one warning per
    // unresolved header so the typo surfaces in lint output.
    for (header_alias, raw) in &doc.import_hints.unknown_grid_cols {
        warnings.push(format!(
            "## {}: `{}` is not a positive integer column count — expected a number like `cols=5`, `cols=3`, or a bare integer (defaulted to 3)",
            header_alias, raw
        ));
    }

    // Unresolved `## Layer X` / `## Layer z=X` header values. The parser
    // falls back to z=0.0 on any parse failure, which collides with the
    // default layer and silently drops the user's intent — a user who
    // typed `## Layer z=abc: Frontend` (meant: `z=120`) or `## Layer foo`
    // (meant: `Layer 1`) gets a layer at z=0 with no feedback. Emit one
    // warning per unresolved header so the typo surfaces in lint output.
    for raw in &doc.import_hints.unknown_layer_z {
        warnings.push(format!(
            "## Layer: `{}` is not a valid layer index or z value — expected a bare integer like `Layer 1` (0-10), a raw z like `Layer 120`, or an explicit form like `Layer z=120` (defaulted to z=0)",
            raw
        ));
    }

    // Unresolved `## Period N` / `## Period N: Label` index values. The
    // parser falls back to idx=0 on any parse failure, so a user who
    // typed `## Period two: Q2 2026` silently placed "Q2 2026" at
    // position 0 instead of position 2. Emit one warning per unresolved
    // header so the typo surfaces in lint output.
    for (raw, label_opt) in &doc.import_hints.unknown_period_idx {
        match label_opt {
            Some(label) => warnings.push(format!(
                "## Period: `{}` in `## Period {}: {}` is not a valid index — expected a positive integer like `## Period 2: {}` (defaulted to position 0)",
                raw, raw, label, label
            )),
            None => warnings.push(format!(
                "## Period: `{}` is not a valid index — expected a positive integer like `## Period 2` or `## Period 2: Label` (defaulted to position 0)",
                raw
            )),
        }
    }

    // Unresolved `## Lane N` / `## Lane N: Label` index values. Same
    // silent-drop pattern as Period — `## Lane three: Engineering`
    // placed "Engineering" at lane 0 and `## Lane foo` created a phantom
    // "Lane 0" label with no feedback. Emit one warning per unresolved
    // header so the typo surfaces in lint output.
    for (raw, label_opt) in &doc.import_hints.unknown_lane_idx {
        match label_opt {
            Some(label) => warnings.push(format!(
                "## Lane: `{}` in `## Lane {}: {}` is not a valid index — expected a positive integer like `## Lane 2: {}` (defaulted to position 0)",
                raw, raw, label, label
            )),
            None => warnings.push(format!(
                "## Lane: `{}` is not a valid index — expected a positive integer like `## Lane 2` or `## Lane 2: Label` (defaulted to position 0)",
                raw
            )),
        }
    }

    // `## Config` `layerN = Name` keys where N was missing or non-numeric.
    // The parser handles these via `if let Ok(idx) = ... { doc.layer_names.insert(...) }`
    // with no else, AND the generic unknown-config-keys fallthrough below
    // specifically excludes `layer*`-prefixed keys, so typos like
    // `layer = Frontend` or `layerfoo = Backend` receive zero feedback.
    // Emit a warning listing the valid formats. Lists examples with
    // existing layer_names (if any) so the user can see the working
    // pattern at a glance.
    for (bad_key, bad_val) in &doc.import_hints.unknown_layer_config_keys {
        warnings.push(format!(
            "## Config: `{} = {}` — `{}` is not a valid layer index key; expected `layer0`, `layer1`, `layer2`, ... (the digit selects the layer, e.g. `layer0 = Base`, `layer1 = {}`)",
            bad_key,
            bad_val,
            bad_key,
            bad_val,
        ));
    }

    // Duplicate parallel edges: same (source, target, label) tuple appearing
    // more than once. Almost always a copy-paste typo in `## Flow` — the two
    // edges get drawn on top of each other and look like one, so the user has
    // no visual signal. Emit one warning per duplicated tuple with the count.
    {
        use std::collections::HashMap;
        let mut edge_counts: HashMap<(crate::model::NodeId, crate::model::NodeId, String), usize> =
            HashMap::new();
        for edge in &doc.edges {
            // Trim whitespace on labels so "foo " and "foo" collapse.
            let key = (
                edge.source.node_id,
                edge.target.node_id,
                edge.label.trim().to_string(),
            );
            *edge_counts.entry(key).or_insert(0) += 1;
        }
        // Sort so output is deterministic.
        let mut dupes: Vec<_> = edge_counts
            .into_iter()
            .filter(|(_, c)| *c > 1)
            .collect();
        dupes.sort_by(|a, b| a.0.0.0.cmp(&b.0.0.0).then(a.0.1.0.cmp(&b.0.1.0)));
        for ((src_id, tgt_id, label), count) in dupes {
            let src_label = doc
                .nodes
                .iter()
                .find(|n| n.id == src_id)
                .map(|n| n.display_label().to_string())
                .unwrap_or_else(|| "?".into());
            let tgt_label = doc
                .nodes
                .iter()
                .find(|n| n.id == tgt_id)
                .map(|n| n.display_label().to_string())
                .unwrap_or_else(|| "?".into());
            let label_suffix = if label.is_empty() {
                String::new()
            } else {
                format!(" labeled {:?}", label)
            };
            warnings.push(format!(
                "Duplicate edge {:?} → {:?}{} appears {} times — likely a copy-paste typo",
                src_label, tgt_label, label_suffix, count
            ));
        }
    }

    // Empty frame detection: a frame with no non-frame nodes inside its
    // bounding box is almost always a leftover — either the contents were
    // deleted or the frame was created empty as a placeholder. Both cases are
    // silent failures: the frame still draws but looks like a random empty
    // rectangle. Check spatial containment since frames don't track member
    // IDs in the model.
    {
        for frame in doc.nodes.iter().filter(|n| n.is_frame) {
            let fx0 = frame.position[0];
            let fy0 = frame.position[1];
            let fx1 = fx0 + frame.size[0];
            let fy1 = fy0 + frame.size[1];
            let mut inside = 0usize;
            for node in &doc.nodes {
                if node.is_frame { continue; }
                if node.id == frame.id { continue; }
                // Node center inside the frame bbox counts as containment.
                let cx = node.position[0] + node.size[0] * 0.5;
                let cy = node.position[1] + node.size[1] * 0.5;
                if cx >= fx0 && cx <= fx1 && cy >= fy0 && cy <= fy1 {
                    inside += 1;
                }
            }
            if inside == 0 {
                warnings.push(format!(
                    "Frame {:?} is empty (contains 0 nodes) — likely a leftover placeholder",
                    frame.display_label()
                ));
            }
        }
    }

    // Check for inconsistent edge labeling — when SOME outgoing edges of a node
    // are labeled but others aren't, that's a sign of forgotten labels rather than
    // a pure hierarchy. (A fully unlabeled fan-out is usually OrgTree-style.)
    {
        let mut out_edges: std::collections::HashMap<crate::model::NodeId, Vec<usize>> =
            std::collections::HashMap::new();
        for (i, edge) in doc.edges.iter().enumerate() {
            out_edges.entry(edge.source.node_id).or_default().push(i);
        }
        for (nid, edge_idxs) in &out_edges {
            if edge_idxs.len() < 2 { continue; }
            let labeled = edge_idxs.iter().filter(|&&i| !doc.edges[i].label.trim().is_empty()).count();
            let unlabeled = edge_idxs.len() - labeled;
            // Only flag if mixed (at least one of each)
            if labeled > 0 && unlabeled > 0 {
                let label = doc.nodes.iter().find(|n| n.id == *nid)
                    .map(|n| n.display_label().to_string())
                    .unwrap_or_else(|| "?".into());
                warnings.push(format!(
                    "Node {:?} has inconsistent edge labeling ({} labeled, {} unlabeled)",
                    label, labeled, unlabeled
                ));
            }
        }
    }

    // Duplicate display labels: when two different HRF IDs carry the same
    // visible label, readers can't tell which node an edge points at.
    // Common after a copy-paste — users duplicate a node row and forget to
    // rename one side. Frames and empty labels are excluded since they have
    // legitimate reasons to repeat. Comparison is case-insensitive so
    // "Start" and "START" are treated as the same; the first-seen original
    // casing is what gets reported.
    {
        use std::collections::HashMap;
        let mut label_to_ids: HashMap<String, (String, Vec<String>)> = HashMap::new();
        for node in &doc.nodes {
            if node.is_frame { continue; }
            let label = node.display_label().trim().to_string();
            if label.is_empty() { continue; }
            let key = label.to_lowercase();
            // Prefer hrf_id for reporting; fall back to the label itself
            // for nodes that came in without an explicit ID.
            let ident = if node.hrf_id.is_empty() {
                label.clone()
            } else {
                node.hrf_id.clone()
            };
            label_to_ids
                .entry(key)
                .or_insert_with(|| (label.clone(), Vec::new()))
                .1
                .push(ident);
        }
        let mut dupes: Vec<_> = label_to_ids
            .into_iter()
            .filter(|(_, (_, ids))| ids.len() >= 2)
            .collect();
        dupes.sort_by(|a, b| a.0.cmp(&b.0));
        for (_key, (display_label, mut ids)) in dupes {
            ids.sort();
            ids.dedup();
            // After dedup we still need >=2 distinct identifiers to warrant
            // a warning — otherwise the duplication is really the same node
            // referenced twice under the same label which is harmless.
            if ids.len() < 2 { continue; }
            warnings.push(format!(
                "Duplicate node label {:?} used by {} different nodes ({}) — readers may not know which is referenced",
                display_label,
                ids.len(),
                ids.join(", ")
            ));
        }
    }

    // Output
    if json {
        // Structured output for CI/IDE integration. Shape is intentionally
        // flat so `jq`/downstream tools can filter easily:
        // `jq '.errors[]' findings.json` or
        // `jq -r '.warnings[] | "\(.)"' findings.json`.
        let payload = serde_json::json!({
            "file": input.display().to_string(),
            "node_count": doc.nodes.len(),
            "edge_count": doc.edges.len(),
            "errors": errors,
            "warnings": warnings,
            "error_count": errors.len(),
            "warning_count": warnings.len(),
            "clean": errors.is_empty() && warnings.is_empty(),
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
        if !errors.is_empty() || (strict && !warnings.is_empty()) {
            std::process::exit(1);
        }
        return;
    }

    let total = warnings.len() + errors.len();
    if total == 0 {
        println!("✓ No issues found in {:?} ({} nodes, {} edges)", input, doc.nodes.len(), doc.edges.len());
    } else {
        for e in &errors {
            eprintln!("✗ ERROR: {}", e);
        }
        for w in &warnings {
            eprintln!("⚠ WARNING: {}", w);
        }
        println!();
        println!("{} error(s), {} warning(s) in {:?}", errors.len(), warnings.len(), input);
        if !errors.is_empty() || (strict && !warnings.is_empty()) {
            std::process::exit(1);
        }
    }
}

fn cli_merge(base_path: PathBuf, overlay_path: PathBuf, out: Option<&std::path::Path>) {
    let mut base = load_doc(&base_path);
    let overlay = load_doc(&overlay_path);

    // Build a mapping from overlay node IDs to new IDs to avoid conflicts
    let mut id_map: std::collections::HashMap<crate::model::NodeId, crate::model::NodeId> = std::collections::HashMap::new();
    let base_ids: std::collections::HashSet<crate::model::NodeId> = base.nodes.iter().map(|n| n.id).collect();

    // Offset overlay nodes so they don't overlap with base nodes
    let base_max_x = base.nodes.iter().map(|n| n.position[0] + n.size[0]).fold(0.0f32, f32::max);
    let x_offset = if base_max_x > 0.0 { base_max_x + 100.0 } else { 0.0 };

    for mut node in overlay.nodes {
        let old_id = node.id;
        if base_ids.contains(&old_id) {
            // Assign a new ID to avoid collision
            let new_id = crate::model::NodeId::new();
            node.id = new_id;
            id_map.insert(old_id, new_id);
        } else {
            id_map.insert(old_id, old_id);
        }
        node.position[0] += x_offset;
        base.nodes.push(node);
    }

    for mut edge in overlay.edges {
        edge.id = crate::model::EdgeId::new();
        if let Some(&new_src) = id_map.get(&edge.source.node_id) {
            edge.source.node_id = new_src;
        }
        if let Some(&new_tgt) = id_map.get(&edge.target.node_id) {
            edge.target.node_id = new_tgt;
        }
        base.edges.push(edge);
    }

    // Merge layer names
    for (k, v) in overlay.layer_names {
        base.layer_names.entry(k).or_insert(v);
    }

    let output_text = crate::specgraph::hrf::export_hrf(&base, "");
    match out {
        Some(path) => {
            std::fs::write(path, &output_text)
                .unwrap_or_else(|e| { eprintln!("Write error: {}", e); std::process::exit(1); });
            println!("Merged {:?} + {:?} → {:?} ({} nodes, {} edges)",
                base_path, overlay_path, path, base.nodes.len(), base.edges.len());
        }
        None => print!("{}", output_text),
    }
}

#[cfg(test)]
mod cli_tests {
    #[test]
    fn test_cli_render_produces_svg() {
        let spec = "## Nodes\n- [alpha] Alpha {done}\n- [beta] Beta {todo}\n## Flow\nalpha --> beta\n";
        let mut doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        crate::specgraph::layout::auto_layout(&mut doc);
        let tmp = std::env::temp_dir().join(format!("test_render_cli_{}.svg", uuid::Uuid::new_v4()));
        crate::export::export_svg(&doc, &tmp).unwrap();
        let content = std::fs::read_to_string(&tmp).unwrap();
        let _ = std::fs::remove_file(&tmp);
        assert!(content.contains("<svg"));
        assert!(content.contains("Alpha"));
    }

    #[test]
    fn test_svg_export_all_templates_wellformed() {
        // Regression guard: every bundled template must export to SVG without
        // panicking and the resulting output must contain basic SVG structure.
        for template in crate::templates::TEMPLATES {
            let mut doc = crate::specgraph::hrf::parse_hrf(template.content)
                .unwrap_or_else(|e| panic!("template '{}' failed to parse: {}", template.name, e));
            crate::specgraph::layout::auto_layout(&mut doc);
            let safe_name: String = template
                .name
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect();
            let tmp = std::env::temp_dir().join(format!(
                "test_svg_export_{}_{}.svg",
                safe_name,
                uuid::Uuid::new_v4()
            ));
            crate::export::export_svg(&doc, &tmp)
                .unwrap_or_else(|e| panic!("template '{}' svg export failed: {}", template.name, e));
            let content = std::fs::read_to_string(&tmp)
                .unwrap_or_else(|e| panic!("template '{}' svg file unreadable: {}", template.name, e));
            let _ = std::fs::remove_file(&tmp);

            // Every SVG must declare an svg element and close it.
            assert!(
                content.contains("<svg"),
                "template '{}' svg missing <svg tag",
                template.name
            );
            assert!(
                content.contains("</svg>"),
                "template '{}' svg missing </svg> closing tag",
                template.name
            );
            // xmlns is essential for the output to render in a browser.
            assert!(
                content.contains("xmlns"),
                "template '{}' svg missing xmlns — won't render in browsers",
                template.name
            );
            // The SVG should be non-trivial in size (>500 bytes means it
            // contains at least shapes + text, not just an empty root).
            assert!(
                content.len() > 500,
                "template '{}' svg suspiciously small ({} bytes)",
                template.name,
                content.len()
            );
        }
    }

    #[test]
    fn test_stats_json_is_valid() {
        // Run the binary and parse its stats --json output. Catches any
        // regression where a tag/shape name contains quotes and breaks
        // hand-built JSON.
        use std::process::Command;
        let spec = "## Nodes\n- [a] Alpha\n- [b] Beta {diamond}\n- [c] Gamma\n\n## Flow\na --> b\nb --> c\n";
        let tmp = std::env::temp_dir().join(format!("stats_json_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, spec).unwrap();

        let exe = std::env::current_exe().unwrap();
        let target_dir = exe.parent().unwrap().parent().unwrap();
        let bin = target_dir.join("open-draftly");
        if !bin.exists() {
            eprintln!("skipping test_stats_json_is_valid: release binary not found at {:?}", bin);
            return;
        }
        let out = Command::new(&bin)
            .arg("stats")
            .arg(&tmp)
            .arg("--json")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|e| panic!("stats --json did not emit valid JSON: {}\n---\n{}", e, stdout));
        assert_eq!(parsed.get("nodes").and_then(|v| v.as_u64()), Some(3));
        assert_eq!(parsed.get("edges").and_then(|v| v.as_u64()), Some(2));
        assert!(parsed.get("shapes").is_some());
        assert!(parsed.get("tags").is_some());
        assert!(parsed.get("layout_depth").is_some());
        assert!(parsed.get("connected_components").is_some());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_stats_counts() {
        let spec = "## Nodes\n- [a] Alpha\n- [b] Beta\n- [c] Gamma\n## Flow\na --> b\nb --> c\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert_eq!(doc.nodes.len(), 3);
        assert_eq!(doc.edges.len(), 2);
    }

    #[test]
    fn test_lint_detects_disconnected() {
        let spec = "## Nodes\n- [a] Alpha\n- [b] Beta\n- [c] Lonely\n## Flow\na --> b\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        // Lonely node 'c' has no edges
        let connected: std::collections::HashSet<_> = doc.edges.iter()
            .flat_map(|e| [e.source.node_id, e.target.node_id])
            .collect();
        let disconnected: Vec<_> = doc.nodes.iter()
            .filter(|n| !n.is_frame && !connected.contains(&n.id))
            .collect();
        assert_eq!(disconnected.len(), 1);
        assert_eq!(disconnected[0].display_label(), "Lonely");
    }

    #[test]
    fn test_merge_combines_nodes() {
        let spec_a = "## Nodes\n- [a] Alpha\n- [b] Beta\n## Flow\na --> b\n";
        let spec_b = "## Nodes\n- [c] Gamma\n- [d] Delta\n## Flow\nc --> d\n";
        let mut base = crate::specgraph::hrf::parse_hrf(spec_a).unwrap();
        let overlay = crate::specgraph::hrf::parse_hrf(spec_b).unwrap();
        let overlay_node_count = overlay.nodes.len();
        let overlay_edge_count = overlay.edges.len();
        for node in overlay.nodes { base.nodes.push(node); }
        for edge in overlay.edges { base.edges.push(edge); }
        assert_eq!(base.nodes.len(), 2 + overlay_node_count);
        assert_eq!(base.edges.len(), 1 + overlay_edge_count);
    }

    #[test]
    fn test_lint_detects_long_label() {
        let long = "This label is intentionally very long to exceed the sixty character limit for testing";
        let spec = format!("## Nodes\n- [a] {}\n- [b] Short\n## Flow\na --> b\n", long);
        let doc = crate::specgraph::hrf::parse_hrf(&spec).unwrap();
        let long_count = doc.nodes.iter()
            .filter(|n| n.display_label().chars().count() > 60)
            .count();
        assert_eq!(long_count, 1);
    }

    #[test]
    fn test_diamond_unlabeled_branches_detected() {
        // Diamond with 2 unlabeled outgoing should be flagged
        let spec = "## Nodes\n- [s] Start {rounded}\n- [d] Decide {diamond}\n- [y] Yes {rounded}\n- [n] No {rounded}\n## Flow\ns --> d\nd --> y\nd --> n\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        use crate::model::{NodeKind, NodeShape};
        let diamond_ids: std::collections::HashSet<_> = doc.nodes.iter()
            .filter(|n| matches!(&n.kind, NodeKind::Shape { shape: NodeShape::Diamond, .. }))
            .map(|n| n.id)
            .collect();
        assert_eq!(diamond_ids.len(), 1);
        let mut out_counts: std::collections::HashMap<_, (usize, usize)> =
            std::collections::HashMap::new();
        for edge in &doc.edges {
            if diamond_ids.contains(&edge.source.node_id) {
                let e = out_counts.entry(edge.source.node_id).or_insert((0, 0));
                e.0 += 1;
                if !edge.label.trim().is_empty() { e.1 += 1; }
            }
        }
        // 2 branches, 0 labeled → should be flagged
        let flagged: Vec<_> = out_counts.iter()
            .filter(|(_, (total, labeled))| *total >= 2 && *labeled < *total)
            .collect();
        assert_eq!(flagged.len(), 1);
    }

    #[test]
    fn test_stats_layout_depth_multi_layer() {
        // a → b → c → d creates 4 layers (depth 4)
        let spec = "## Nodes\n- [a] A\n- [b] B\n- [c] C\n- [d] D\n## Flow\na --> b\nb --> c\nc --> d\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let node_idx: std::collections::HashMap<_, _> =
            doc.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let n = doc.nodes.len();
        let mut adj = vec![Vec::new(); n];
        let mut in_deg = vec![0i32; n];
        for edge in &doc.edges {
            if let (Some(&from), Some(&to)) =
                (node_idx.get(&edge.source.node_id), node_idx.get(&edge.target.node_id))
            {
                adj[from].push(to);
                in_deg[to] += 1;
            }
        }
        let mut layer = vec![0i32; n];
        let mut rem = in_deg.clone();
        let mut queue: std::collections::VecDeque<_> = std::collections::VecDeque::new();
        for (i, &d) in rem.iter().enumerate() {
            if d == 0 { queue.push_back(i); }
        }
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                let cand = layer[u] + 1;
                if cand > layer[v] { layer[v] = cand; }
                rem[v] -= 1;
                if rem[v] == 0 { queue.push_back(v); }
            }
        }
        let depth = *layer.iter().max().unwrap() as usize + 1;
        assert_eq!(depth, 4);
    }

    #[test]
    fn test_diff_detects_modified_label() {
        let spec_a = "## Nodes\n- [a] Original\n- [b] Beta\n## Flow\na --> b\n";
        let spec_b = "## Nodes\n- [a] Renamed\n- [b] Beta\n## Flow\na --> b\n";
        let doc_a = crate::specgraph::hrf::parse_hrf(spec_a).unwrap();
        let doc_b = crate::specgraph::hrf::parse_hrf(spec_b).unwrap();
        let node_a = doc_a.nodes.iter().find(|n| n.hrf_id == "a").unwrap();
        let node_b = doc_b.nodes.iter().find(|n| n.hrf_id == "a").unwrap();
        assert_ne!(node_a.display_label(), node_b.display_label());
    }

    #[test]
    fn test_lint_diamond_with_one_branch_flagged() {
        // A diamond with only 1 outgoing edge isn't really a decision.
        let spec = "## Nodes\n- [a] Start {rounded}\n- [d] Decision {diamond}\n- [e] End {rounded}\n## Flow\na --> d: start\nd --> e: only path\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        use crate::model::{NodeKind, NodeShape};
        let mut out_counts: std::collections::HashMap<crate::model::NodeId, usize> =
            std::collections::HashMap::new();
        for edge in &doc.edges {
            *out_counts.entry(edge.source.node_id).or_default() += 1;
        }
        let flagged: Vec<_> = doc.nodes.iter()
            .filter(|n| matches!(&n.kind, NodeKind::Shape { shape: NodeShape::Diamond, .. }))
            .filter(|n| out_counts.get(&n.id).copied().unwrap_or(0) < 2)
            .collect();
        assert_eq!(flagged.len(), 1, "diamond with 1 outgoing edge should be flagged");
    }

    #[test]
    fn test_lint_decision_branch_labels_e2e_flags_partial() {
        // End-to-end: a diamond with 2 branches where only 1 is labeled
        // should surface the "X branches but only Y are labeled" warning
        // through the CLI's lint --json pipeline.
        let spec = "## Nodes\n- [a] Start {rounded}\n- [d] Decide {diamond}\n\
                    - [y] Yes {rounded}\n- [n] No {rounded}\n\
                    ## Flow\na --> d\nd --> y: yes\nd --> n\n";
        let tmp = std::env::temp_dir().join(format!("decision_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_decision = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Decide") && s.contains("branches") && s.contains("labeled")
        });
        assert!(has_decision, "expected decision-branch warning, got: {}", stdout);
    }

    #[test]
    fn test_lint_decision_branch_labels_e2e_allows_fully_labeled() {
        // A diamond whose outgoing branches are ALL labeled must NOT trip the
        // decision-branch warning.
        let spec = "## Nodes\n- [a] Start {rounded}\n- [d] Decide {diamond}\n\
                    - [y] Yes {rounded}\n- [n] No {rounded}\n\
                    ## Flow\na --> d\nd --> y: yes\nd --> n: no\n";
        let tmp = std::env::temp_dir().join(format!("decision_ok_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let any_decision = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Decide") && s.contains("branches") && s.contains("labeled")
        });
        assert!(
            !any_decision,
            "fully-labeled decision branches must not be flagged, got: {}",
            stdout
        );
    }

    #[test]
    fn test_lint_detects_disconnected_subgraphs() {
        // Two disconnected subgraphs: {a→b} and {c→d}
        let spec = "## Nodes\n- [a] A\n- [b] B\n- [c] C\n- [d] D\n## Flow\na --> b\nc --> d\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        use crate::model::NodeId;
        let mut adj: std::collections::HashMap<NodeId, Vec<NodeId>> =
            std::collections::HashMap::new();
        for node in &doc.nodes {
            if !node.is_frame {
                adj.entry(node.id).or_default();
            }
        }
        for edge in &doc.edges {
            adj.entry(edge.source.node_id).or_default().push(edge.target.node_id);
            adj.entry(edge.target.node_id).or_default().push(edge.source.node_id);
        }
        let mut visited: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        let mut components = 0usize;
        for &start in adj.keys() {
            if visited.contains(&start) { continue; }
            components += 1;
            let mut stack = vec![start];
            while let Some(cur) = stack.pop() {
                if !visited.insert(cur) { continue; }
                if let Some(nbrs) = adj.get(&cur) {
                    for &n in nbrs { stack.push(n); }
                }
            }
        }
        assert_eq!(components, 2, "should find 2 disconnected components");
    }

    #[test]
    fn test_lint_duplicate_display_labels_flagged_e2e() {
        // Two different HRF IDs carrying the same display label is a
        // readability smell. lint --json should emit a warning that names
        // both IDs and preserves the original casing.
        let spec = "## Nodes\n- [first] Start\n- [middle] Middle\n\
                    - [second] Start\n\n## Flow\nfirst --> middle\nmiddle --> second\n";
        let tmp = std::env::temp_dir().join(format!("dup_label_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let hit = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Duplicate node label") && s.contains("\"Start\"")
                && s.contains("first") && s.contains("second")
        });
        assert!(
            hit,
            "expected duplicate-label warning naming 'Start', 'first', 'second', got: {}",
            stdout
        );
    }

    #[test]
    fn test_lint_duplicate_label_case_insensitive() {
        // "Review" and "REVIEW" should still collapse since readers
        // can't tell them apart at a glance.
        let spec = "## Nodes\n- [r1] Review\n- [next] Next\n- [r2] REVIEW\n\n\
                    ## Flow\nr1 --> next\nnext --> r2\n";
        let tmp = std::env::temp_dir().join(format!("dup_label_case_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let hit = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Duplicate node label") && (s.contains("Review") || s.contains("REVIEW"))
        });
        assert!(hit, "expected duplicate-label warning, got: {}", stdout);
    }

    #[test]
    fn test_lint_distinct_labels_not_flagged() {
        // Two different labels must never trip the duplicate-label lint.
        let spec = "## Nodes\n- [a] Alpha\n- [b] Beta\n- [c] Gamma\n\n\
                    ## Flow\na --> b\nb --> c\n";
        let tmp = std::env::temp_dir().join(format!("distinct_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let any_dup = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("Duplicate node label")
        });
        assert!(!any_dup, "distinct labels must not trip the lint, got: {}", stdout);
    }

    #[test]
    fn test_description_parsed_as_sublabel() {
        // Indented continuation after a node should populate the Shape.description field,
        // which export.rs renders as a sublabel.
        let spec = "## Nodes\n- [a] Hello\n  This is a description.\n- [b] World\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let node_a = doc.nodes.iter().find(|n| n.hrf_id == "a").unwrap();
        if let crate::model::NodeKind::Shape { description, .. } = &node_a.kind {
            assert_eq!(description, "This is a description.");
        } else {
            panic!("expected shape");
        }
    }

    #[test]
    fn test_description_triggers_post_parse_autosize() {
        // A long description should expand the node's width beyond the label's needs.
        let spec = "## Nodes\n- [a] Short\n  This is a much longer description that should force auto-sizing.\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let node = doc.nodes.iter().find(|n| n.hrf_id == "a").unwrap();
        // Default rounded rect is 140px wide; post-parse pass should expand this.
        assert!(node.size[0] > 140.0, "width should expand for long description, got {}", node.size[0]);
    }

    #[test]
    fn test_shape_typo_suggests_diamond() {
        use crate::specgraph::hrf::suggest_shape_alias;
        assert_eq!(suggest_shape_alias("daimond"), Some("diamond"));
        assert_eq!(suggest_shape_alias("diomond"), Some("diamond"));
    }

    #[test]
    fn test_shape_typo_suggests_cylinder() {
        use crate::specgraph::hrf::suggest_shape_alias;
        assert_eq!(suggest_shape_alias("cylindar"), Some("cylinder"));
    }

    #[test]
    fn test_shape_typo_ignores_exact_matches() {
        use crate::specgraph::hrf::suggest_shape_alias;
        for known in ["diamond", "rectangle", "cylinder", "person", "hexagon"] {
            assert_eq!(
                suggest_shape_alias(known),
                None,
                "exact-match shape '{known}' should not get a suggestion"
            );
        }
    }

    #[test]
    fn test_shape_typo_ignores_unrelated_words() {
        use crate::specgraph::hrf::suggest_shape_alias;
        assert_eq!(suggest_shape_alias("totallyrandom"), None);
        assert_eq!(suggest_shape_alias("xyzzy"), None);
    }

    #[test]
    fn test_shape_typo_unknown_tags_stored_on_node() {
        // End-to-end: a typo'd shape tag lands in node.unknown_tags and
        // suggest_shape_alias identifies it.
        let spec = "## Nodes\n- [a] Decision {daimond}\n- [b] DB {cylinder}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let a = doc.nodes.iter().find(|n| n.hrf_id == "a").unwrap();
        assert_eq!(a.unknown_tags, vec!["daimond".to_string()]);
        let b = doc.nodes.iter().find(|n| n.hrf_id == "b").unwrap();
        assert!(b.unknown_tags.is_empty(), "known tag should not leak into unknown_tags");
    }

    #[test]
    fn test_shape_prefix_typo_stored_in_unknown_tags() {
        // `{shape:diamon}` / `{type:circel}` / `{kind:hexgon}` — the parser
        // must preserve the full prefixed tag so lint can strip the prefix
        // and suggest the closest shape alias. Previously `tag_to_shape`
        // silently defaulted to RoundedRect.
        let spec = "## Nodes\n\
                    - [a] Alpha {shape:diamon}\n\
                    - [b] Beta {type:circel}\n\
                    - [c] Gamma {kind:hexgon}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let a = doc.nodes.iter().find(|n| n.hrf_id == "a").unwrap();
        assert!(
            a.unknown_tags.iter().any(|t| t == "shape:diamon"),
            "expected `shape:diamon` in unknown_tags, got {:?}", a.unknown_tags
        );
        let b = doc.nodes.iter().find(|n| n.hrf_id == "b").unwrap();
        assert!(
            b.unknown_tags.iter().any(|t| t == "type:circel"),
            "expected `type:circel` in unknown_tags, got {:?}", b.unknown_tags
        );
        let c = doc.nodes.iter().find(|n| n.hrf_id == "c").unwrap();
        assert!(
            c.unknown_tags.iter().any(|t| t == "kind:hexgon"),
            "expected `kind:hexgon` in unknown_tags, got {:?}", c.unknown_tags
        );
    }

    #[test]
    fn test_edge_typo_suggests_dashed() {
        use crate::specgraph::hrf::suggest_edge_style_alias;
        assert_eq!(suggest_edge_style_alias("dahsed"), Some("dashed"));
        assert_eq!(suggest_edge_style_alias("dased"), Some("dashed"));
    }

    #[test]
    fn test_edge_typo_suggests_thick_ortho() {
        use crate::specgraph::hrf::suggest_edge_style_alias;
        assert_eq!(suggest_edge_style_alias("thikc"), Some("thick"));
        assert_eq!(suggest_edge_style_alias("othro"), Some("ortho"));
    }

    #[test]
    fn test_edge_typo_ignores_known_tags() {
        use crate::specgraph::hrf::suggest_edge_style_alias;
        for t in ["dashed", "glow", "thick", "ortho", "escalate", "resolves", "blocks"] {
            assert_eq!(
                suggest_edge_style_alias(t),
                None,
                "known edge tag '{t}' should not be flagged"
            );
        }
    }

    #[test]
    fn test_edge_typo_ignores_prefixed_tags() {
        use crate::specgraph::hrf::suggest_edge_style_alias;
        // color:/note:/weight: are value-bearing — not bare style tags.
        assert_eq!(suggest_edge_style_alias("color:#abc"), None);
        assert_eq!(suggest_edge_style_alias("note:hi"), None);
        assert_eq!(suggest_edge_style_alias("weight:3"), None);
    }

    #[test]
    fn test_edge_typo_unknown_tag_preserved_on_flow_edge() {
        // End-to-end: typo on a `## Flow` edge lands in edge.unknown_tags.
        let spec = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b {dahsed}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert_eq!(doc.edges.len(), 1);
        assert_eq!(doc.edges[0].unknown_tags, vec!["dahsed".to_string()]);
    }

    #[test]
    fn test_edge_typo_unknown_tag_preserved_on_inline_edge() {
        // End-to-end: typo on an inline-edge (`- [a] A --> b {tag}`) also
        // lands in edge.unknown_tags — previously this path silently dropped
        // unrecognized tags.
        let spec = "## Nodes\n- [a] A --> b {dahsed}\n- [b] B\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert_eq!(doc.edges.len(), 1);
        assert_eq!(doc.edges[0].unknown_tags, vec!["dahsed".to_string()]);
    }

    #[test]
    fn test_arrow_style_typo_suggests_circle() {
        use crate::specgraph::hrf::suggest_arrow_style;
        assert_eq!(suggest_arrow_style("arrow:cirlce"), Some("arrow:circle"));
        assert_eq!(suggest_arrow_style("arrow:cirle"),  Some("arrow:circle"));
    }

    #[test]
    fn test_arrow_style_typo_suggests_open_and_none() {
        use crate::specgraph::hrf::suggest_arrow_style;
        assert_eq!(suggest_arrow_style("arrow:opn"),  Some("arrow:open"));
        assert_eq!(suggest_arrow_style("arrow:non"),  Some("arrow:none"));
    }

    #[test]
    fn test_arrow_style_ignores_exact_matches() {
        use crate::specgraph::hrf::suggest_arrow_style;
        for known in ["arrow:open", "arrow:circle", "arrow:none"] {
            assert_eq!(
                suggest_arrow_style(known),
                None,
                "exact-match arrow style '{known}' should not be flagged"
            );
        }
    }

    #[test]
    fn test_arrow_style_ignores_non_arrow_prefixes() {
        use crate::specgraph::hrf::suggest_arrow_style;
        // color:/bend:/weight: are not arrow sub-tags — must never be
        // suggested as `arrow:*`.
        assert_eq!(suggest_arrow_style("color:#abc"), None);
        assert_eq!(suggest_arrow_style("bend:0.5"),   None);
        assert_eq!(suggest_arrow_style("weight:3"),   None);
        assert_eq!(suggest_arrow_style("dashed"),     None);
    }

    #[test]
    fn test_arrow_style_ignores_empty_and_wild_suffix() {
        use crate::specgraph::hrf::suggest_arrow_style;
        // Empty suffix: no suggestion
        assert_eq!(suggest_arrow_style("arrow:"), None);
        // Far-off suffix: no suggestion (keeps noise out of lint)
        assert_eq!(suggest_arrow_style("arrow:totallyrandom"), None);
    }

    #[test]
    fn test_arrow_style_typo_preserved_and_linted_e2e() {
        // End-to-end: an `arrow:cirlce` typo on a flow edge lands in
        // edge.unknown_tags, and the suggestor resolves it. This guards both
        // the parser preservation path and the suggestion helper together.
        let spec = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b {arrow:cirlce}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert_eq!(doc.edges.len(), 1);
        assert_eq!(doc.edges[0].unknown_tags, vec!["arrow:cirlce".to_string()]);
        assert_eq!(
            crate::specgraph::hrf::suggest_arrow_style(&doc.edges[0].unknown_tags[0]),
            Some("arrow:circle")
        );
    }

    #[test]
    fn test_lint_duplicate_edges_parse_creates_two_edges() {
        // Parser sanity: two identical `a --> b: ping` lines should yield
        // two distinct edges in doc.edges. If the parser silently merged
        // them the duplicate lint would have nothing to flag.
        let spec = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b: ping\na --> b: ping\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert_eq!(doc.edges.len(), 2, "expected two distinct edges, got {}", doc.edges.len());
        assert_eq!(doc.edges[0].source.node_id, doc.edges[1].source.node_id);
        assert_eq!(doc.edges[0].target.node_id, doc.edges[1].target.node_id);
        assert_eq!(doc.edges[0].label.trim(), "ping");
        assert_eq!(doc.edges[1].label.trim(), "ping");
    }

    #[test]
    fn test_lint_duplicate_edges_flagged() {
        // Same (source, target, label) appearing twice is a copy-paste typo.
        // End-to-end via lint --json so we exercise the full warning pipeline.
        use std::process::Command;
        let spec = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b: ping\na --> b: ping\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("dup_edge_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();

        let exe = std::env::current_exe().unwrap();
        let bin = exe.parent().unwrap().parent().unwrap().join("open-draftly");
        if !bin.exists() {
            eprintln!("skipping test_lint_duplicate_edges_flagged: release binary not found");
            let _ = std::fs::remove_file(&tmp);
            return;
        }
        let out = Command::new(&bin).arg("lint").arg(&tmp).arg("--json").output().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .expect("lint --json should be valid JSON");
        let warnings = parsed.get("warnings").unwrap().as_array().unwrap();
        assert!(
            warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("Duplicate edge") && s.contains("appears 2 times")
            }),
            "expected duplicate-edge warning, got: {warnings:?}"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_lint_duplicate_edges_different_labels_not_flagged() {
        // Same endpoints but different labels = two distinct semantic edges.
        // Must NOT trigger the duplicate warning.
        use std::process::Command;
        let spec = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b: ping\na --> b: pong\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("dup_edge_distinct_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();

        let exe = std::env::current_exe().unwrap();
        let bin = exe.parent().unwrap().parent().unwrap().join("open-draftly");
        if !bin.exists() {
            eprintln!("skipping test_lint_duplicate_edges_different_labels_not_flagged: release binary not found");
            let _ = std::fs::remove_file(&tmp);
            return;
        }
        let out = Command::new(&bin).arg("lint").arg(&tmp).arg("--json").output().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .expect("lint --json should be valid JSON");
        let warnings = parsed.get("warnings").unwrap().as_array().unwrap();
        assert!(
            !warnings.iter().any(|w| {
                w.as_str().unwrap_or("").contains("Duplicate edge")
            }),
            "should NOT warn about duplicate edges when labels differ, got: {warnings:?}"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_lint_duplicate_display_label_unit() {
        // Two distinct hrf_ids with the same display label should be
        // grouped and flagged by the duplicate-label lint. This is a
        // pure parse test that mirrors the cli_lint algorithm without
        // needing the CLI binary.
        let spec = "## Nodes\n- [a] Deploy\n- [b] Deploy\n- [c] Review\n## Flow\na --> c\nb --> c\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        use std::collections::HashMap;
        let mut label_to_ids: HashMap<String, Vec<String>> = HashMap::new();
        for node in &doc.nodes {
            if node.is_frame { continue; }
            let label = node.display_label().trim().to_string();
            if label.is_empty() { continue; }
            let key = label.to_lowercase();
            let ident = if node.hrf_id.is_empty() {
                label.clone()
            } else {
                node.hrf_id.clone()
            };
            label_to_ids.entry(key).or_default().push(ident);
        }
        let dupes: Vec<_> = label_to_ids
            .iter()
            .filter(|(_, ids)| {
                let mut sorted = (*ids).clone();
                sorted.sort();
                sorted.dedup();
                sorted.len() >= 2
            })
            .collect();
        assert_eq!(dupes.len(), 1, "expected exactly one duplicated label group");
        let (label, ids) = dupes[0];
        assert_eq!(label, "deploy");
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_lint_duplicate_display_label_case_insensitive_via_cli() {
        // "Deploy" and "DEPLOY" should collapse via lowercasing and
        // fire the duplicate-label warning end-to-end through lint --json.
        let spec = "## Nodes\n- [first] Deploy\n- [second] DEPLOY\n- [third] Review\n\
                    ## Flow\nfirst --> third\nsecond --> third\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("dup_label_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_dupe = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Duplicate node label") && s.to_lowercase().contains("deploy")
        });
        assert!(
            has_dupe,
            "expected duplicate-label warning, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_duplicate_display_label_distinct_labels_not_flagged() {
        // Three distinct labels: must NOT fire the duplicate-label warning.
        let spec = "## Nodes\n- [a] Alpha\n- [b] Beta\n- [c] Gamma\n## Flow\na --> b\nb --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("dup_label_none_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_dupe = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("Duplicate node label")
        });
        assert!(
            !has_dupe,
            "distinct labels must not be flagged, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_duplicate_display_label_frames_excluded() {
        // Frames with duplicate labels must NOT fire the warning —
        // frames often legitimately share names across groups. Build the
        // doc programmatically so we can inject two is_frame=true nodes
        // with identical labels without depending on group syntax.
        let mut doc = crate::model::FlowchartDocument::default();
        let mut f1 = crate::model::Node::new_frame(egui::Pos2::new(0.0, 0.0));
        f1.size = [200.0, 200.0];
        f1.hrf_id = "g1".into();
        if let crate::model::NodeKind::Shape { label, .. } = &mut f1.kind {
            *label = "Cluster".into();
        }
        let mut f2 = crate::model::Node::new_frame(egui::Pos2::new(500.0, 0.0));
        f2.size = [200.0, 200.0];
        f2.hrf_id = "g2".into();
        if let crate::model::NodeKind::Shape { label, .. } = &mut f2.kind {
            *label = "Cluster".into();
        }
        doc.nodes.push(f1);
        doc.nodes.push(f2);
        assert_eq!(doc.nodes.iter().filter(|n| n.is_frame).count(), 2);

        // Mirror the lint's exclusion rule: frames are skipped entirely.
        use std::collections::HashMap;
        let mut label_to_ids: HashMap<String, Vec<String>> = HashMap::new();
        for node in &doc.nodes {
            if node.is_frame { continue; } // <- exclusion under test
            let label = node.display_label().trim().to_lowercase();
            if label.is_empty() { continue; }
            label_to_ids.entry(label).or_default().push(node.hrf_id.clone());
        }
        // "cluster" must be absent because both carriers are frames.
        assert!(
            !label_to_ids.contains_key("cluster"),
            "frame labels must not be counted toward duplicates"
        );
    }

    #[test]
    fn test_suggest_port_side_exact_matches_return_none() {
        use crate::specgraph::hrf::suggest_port_side;
        for known in ["top", "bottom", "left", "right", "TOP", "Left"] {
            assert_eq!(suggest_port_side(known), None, "known '{known}'");
        }
    }

    #[test]
    fn test_suggest_port_side_compass_aliases() {
        use crate::specgraph::hrf::suggest_port_side;
        assert_eq!(suggest_port_side("north"), Some("top"));
        assert_eq!(suggest_port_side("south"), Some("bottom"));
        assert_eq!(suggest_port_side("west"), Some("left"));
        assert_eq!(suggest_port_side("east"), Some("right"));
        assert_eq!(suggest_port_side("n"), Some("top"));
        assert_eq!(suggest_port_side("sw"), Some("bottom"));
    }

    #[test]
    fn test_suggest_port_side_diagonal_fallbacks() {
        use crate::specgraph::hrf::suggest_port_side;
        // Diagonals fall back to the nearest cardinal; we only need them
        // to route to SOMETHING valid so the user still gets a hint.
        assert_eq!(suggest_port_side("topleft"), Some("top"));
        assert_eq!(suggest_port_side("top-left"), Some("top"));
        assert_eq!(suggest_port_side("bottomright"), Some("bottom"));
        assert_eq!(suggest_port_side("center"), Some("top"));
    }

    #[test]
    fn test_suggest_port_side_typos() {
        use crate::specgraph::hrf::suggest_port_side;
        assert_eq!(suggest_port_side("bottm"), Some("bottom"));
        assert_eq!(suggest_port_side("rigt"), Some("right"));
        assert_eq!(suggest_port_side("lef"), Some("left"));
        assert_eq!(suggest_port_side("topp"), Some("top"));
    }

    #[test]
    fn test_suggest_port_side_rejects_junk() {
        use crate::specgraph::hrf::suggest_port_side;
        assert_eq!(suggest_port_side(""), None);
        assert_eq!(suggest_port_side("xy"), None);
        assert_eq!(suggest_port_side("xyzzypuzzle"), None);
    }

    #[test]
    fn test_parse_captures_invalid_port_side() {
        // `{sport:cent}` used to fall back to Bottom silently. Now it
        // must show up in import_hints.invalid_port_side_values.
        let spec = "## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b {sport:cent} {tport:nope}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let invalid = &doc.import_hints.invalid_port_side_values;
        assert_eq!(invalid.len(), 2, "expected two invalid port sides");
        assert!(invalid.iter().any(|(k, v)| k == "src" && v == "cent"));
        assert!(invalid.iter().any(|(k, v)| k == "tgt" && v == "nope"));
    }

    #[test]
    fn test_parse_known_port_side_does_not_populate_invalid() {
        // Canonical port-side values must NOT populate the invalid list.
        let spec = "## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b {sport:right} {tport:left}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert!(
            doc.import_hints.invalid_port_side_values.is_empty(),
            "known port sides should not populate invalid list, got: {:?}",
            doc.import_hints.invalid_port_side_values
        );
    }

    #[test]
    fn test_lint_invalid_port_side_via_cli() {
        // End-to-end: invalid port side surfaces as a warning with the
        // "did you mean" phrasing.
        let spec = "## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b {sport:bottm}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("port_invalid_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_port = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Invalid source port side")
                && s.contains("bottm")
                && s.contains("did you mean")
                && s.contains("bottom")
        });
        assert!(has_port, "expected invalid-port warning, got: {stdout}");
    }

    #[test]
    fn test_lint_valid_port_side_not_flagged_via_cli() {
        // Canonical port side must NOT fire the warning.
        let spec = "## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b {sport:right}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("port_valid_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_port = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("Invalid") && w.as_str().unwrap_or("").contains("port side")
        });
        assert!(!has_port, "valid port side must not fire, got: {stdout}");
    }

    #[test]
    fn test_lint_single_self_loop_still_warns_once_via_cli() {
        // Regression guard: a single self-loop is often intentional
        // (state retention, event handler loopback) but still deserves a
        // single notice-level warning. The aggregation refactor must not
        // suppress it. Exactly one "Self-loop on node" warning should
        // appear, and NO "multiple self-loops" warning.
        let spec = "## Nodes\n- [a] Retry\n- [b] Done\n## Flow\na --> a\na --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("single_self_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let single_count = warnings
            .iter()
            .filter(|w| {
                let s = w.as_str().unwrap_or("");
                s.starts_with("Self-loop on node") && s.contains("\"Retry\"")
            })
            .count();
        assert_eq!(single_count, 1, "expected exactly one single-loop warning, got: {stdout}");
        let has_multi = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("self-loops") && s.contains("copy-paste")
        });
        assert!(!has_multi, "one self-loop must not fire multi-loop warning, got: {stdout}");
    }

    #[test]
    fn test_lint_self_loops_distinct_nodes_warn_individually_via_cli() {
        // Three separate nodes each with ONE self-loop should produce
        // three independent single-loop warnings — the aggregation is
        // per-node, not per-document.
        let spec = "## Nodes\n\
                    - [a] A\n\
                    - [b] B\n\
                    - [c] C\n\
                    ## Flow\n\
                    a --> a\n\
                    b --> b\n\
                    c --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("distinct_self_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for label in ["\"A\"", "\"B\"", "\"C\""] {
            let has = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.starts_with("Self-loop on node") && s.contains(label)
            });
            assert!(has, "expected single-loop warning for {label}, got: {stdout}");
        }
        let has_multi = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("self-loops") && s.contains("copy-paste")
        });
        assert!(!has_multi, "distinct single loops must not fire multi warning, got: {stdout}");
    }

    #[test]
    fn test_lint_three_self_loops_reports_count_via_cli() {
        // Three self-loops on ONE node must be aggregated into a single
        // "has 3 self-loops" warning with the exact count — not three
        // separate single-loop warnings, and not a generic "multiple"
        // without a number.
        let spec = "## Nodes\n- [node] Processor\n## Flow\nnode --> node\nnode --> node\nnode --> node\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("triple_self_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_three = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("has 3 self-loops") && s.contains("\"Processor\"")
        });
        assert!(has_three, "expected 'has 3 self-loops' warning, got: {stdout}");
        // Must not also emit the single-loop variant.
        let single_count = warnings
            .iter()
            .filter(|w| {
                let s = w.as_str().unwrap_or("");
                s.starts_with("Self-loop on node") && s.contains("\"Processor\"")
            })
            .count();
        assert_eq!(single_count, 0, "triple loop must subsume single variant, got: {stdout}");
    }

    #[test]
    fn test_stats_detects_simple_cycle() {
        // 3-node ring: a → b → c → a. Kahn's topo sort can never strip
        // any of the three (each has in-degree 1 from a cycle participant),
        // so `has_cycle` must be true and `cycle_node_count` == 3.
        let spec = "## Nodes\n- [a] A\n- [b] B\n- [c] C\n## Flow\na --> b\nb --> c\nc --> a\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("cycle_stats_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["stats", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("stats --json not valid JSON: {stdout}"));
        assert_eq!(v["has_cycle"].as_bool(), Some(true), "expected has_cycle=true, got: {stdout}");
        assert_eq!(v["cycle_node_count"].as_u64(), Some(3), "expected 3 cycle nodes, got: {stdout}");
    }

    #[test]
    fn test_stats_reports_dag_without_cycle() {
        // Linear chain: a → b → c. No cycle, `has_cycle` must be false.
        let spec = "## Nodes\n- [a] A\n- [b] B\n- [c] C\n## Flow\na --> b\nb --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("dag_stats_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["stats", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("stats --json not valid JSON: {stdout}"));
        assert_eq!(v["has_cycle"].as_bool(), Some(false), "DAG must report has_cycle=false, got: {stdout}");
        assert_eq!(v["cycle_node_count"].as_u64(), Some(0), "DAG must report 0 cycle nodes, got: {stdout}");
    }

    #[test]
    fn test_stats_avg_degree_computed() {
        // Linear chain: a → b → c. Two edges, three non-frame nodes.
        // avg_degree = 2*2/3 = 1.333...
        let spec = "## Nodes\n- [a] A\n- [b] B\n- [c] C\n## Flow\na --> b\nb --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("avg_deg_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["stats", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("stats --json not valid JSON: {stdout}"));
        let avg = v["avg_degree"].as_f64().expect("avg_degree number");
        assert!(
            (avg - 1.333).abs() < 0.01,
            "expected avg_degree ≈ 1.333, got {}: {stdout}", avg
        );
    }

    #[test]
    fn test_lint_multiple_self_loops_aggregated() {
        // Two self-loops on the same node collapse into a single
        // "multiple self-loops" warning with the count — instead of two
        // identical "Self-loop on node X" warnings the single-loop case
        // would emit.
        let spec = "## Nodes\n- [a] A\n- [b] B\n## Flow\na --> a\na --> a\nb --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("multi_self_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        // "A" has 2 self-loops → "multiple" variant
        let has_multi = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("has 2 self-loops") && s.contains("\"A\"")
        });
        assert!(has_multi, "expected multiple self-loop warning for A, got: {stdout}");
        // "B" has 1 self-loop → single variant
        let has_single = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Self-loop on node") && s.contains("\"B\"")
        });
        assert!(has_single, "expected single self-loop warning for B, got: {stdout}");
        // The single-loop warning for A must NOT also appear (it's subsumed).
        let has_single_a = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.starts_with("Self-loop on node") && s.contains("\"A\"")
        });
        assert!(!has_single_a, "A's loops should not produce a single-loop warning, got: {stdout}");
    }

    #[test]
    fn test_lint_style_template_typo_flagged_via_cli() {
        // A defined `## Style primary` and a node tag `{primry}` (one char
        // off) should fire the style-template suggestion fallback — the
        // shape-alias walk returns None for `primry`, so without this
        // fallback the user gets zero signal that the style expansion
        // silently no-op'd.
        let spec = "## Style\n\
                    primary = {fill:#0000ff}\n\
                    danger = {fill:#ff0000}\n\
                    \n\
                    ## Nodes\n\
                    - [a] Alpha {primry}\n\
                    - [b] Beta {primary}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("style_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("primry")
                && s.contains("did you mean")
                && s.contains("primary")
                && s.contains("## Style")
        });
        assert!(has, "expected style-template typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_no_style_section_no_fallback_via_cli() {
        // When no `## Style` section exists, the style-template fallback
        // must stay silent — an unknown tag with no shape suggestion is
        // simply dropped (there's nothing to suggest from).
        let spec = "## Nodes\n- [a] Alpha {weirdstyle}\n- [b] Beta\n## Flow\na --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("no_style_fallback_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("weirdstyle") && s.contains("## Style")
        });
        assert!(!has, "unknown tag without style section must not trigger fallback, got: {stdout}");
    }

    #[test]
    fn test_lint_style_exact_match_no_fallback_via_cli() {
        // When a node uses the correct style name, `expand_styles` consumes
        // the tag before it reaches `unknown_tags` — so the fallback walks
        // an empty list and emits nothing. This guards that exact matches
        // never produce a noisy "did you mean X" against themselves.
        let spec = "## Style\n\
                    primary = {fill:#0000ff}\n\
                    \n\
                    ## Nodes\n\
                    - [a] Alpha {primary}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("style_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("## Style") && s.contains("did you mean")
        });
        assert!(!has, "exact style match must not fire fallback, got: {stdout}");
    }

    #[test]
    fn test_lint_very_short_style_name_not_flagged_via_cli() {
        // Style names shorter than 3 chars are skipped by the fallback.
        // Here `fx` is a 2-char style and `fy` on a node is d=1 away, but
        // the length guard prevents a noisy match (too many single-char
        // pairs would fire false positives).
        let spec = "## Style\n\
                    fx = {fill:#0000ff}\n\
                    \n\
                    ## Nodes\n\
                    - [a] Alpha {fy}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("short_style_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("fy") && s.contains("## Style") && s.contains("did you mean")
        });
        assert!(!has, "short style name must not trigger fallback, got: {stdout}");
    }

    #[test]
    fn test_lint_unresolved_fill_palette_typo_via_cli() {
        // Palette typo case: `{fill:primry}` where `## Palette` defines
        // `primary = #hex`. Should suggest the palette entry.
        let spec = "## Palette\n\
                    primary = #ff0000\n\
                    accent = #00ff00\n\
                    \n\
                    ## Nodes\n\
                    - [a] Alpha {fill:primry}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("fill_pal_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("primry")
                && s.contains("did you mean")
                && s.contains("primary")
                && s.contains("## Palette")
        });
        assert!(has, "expected palette-typo fill warning, got: {stdout}");
    }

    #[test]
    fn test_lint_unresolved_fill_builtin_color_typo_via_cli() {
        // No-palette case: `{fill:blu}` should suggest `blue` (built-in).
        let spec = "## Nodes\n\
                    - [a] Alpha {fill:blu}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("fill_builtin_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("fill:blu")
                && s.contains("did you mean")
                && s.contains("fill:blue")
        });
        assert!(has, "expected built-in color typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_unresolved_fill_exact_palette_no_warning_via_cli() {
        // Exact palette match: `{fill:primary}` should NOT emit an unresolved
        // color warning because the palette expansion succeeds.
        let spec = "## Palette\n\
                    primary = #ff0000\n\
                    \n\
                    ## Nodes\n\
                    - [a] Alpha {fill:primary}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("fill_pal_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unresolved color")
        });
        assert!(!has, "exact palette match must not emit unresolved color warning, got: {stdout}");
    }

    #[test]
    fn test_lint_unresolved_fill_exact_builtin_color_no_warning_via_cli() {
        // Exact built-in color: `{fill:blue}` should NOT emit any unresolved
        // color warning since `tag_to_fill_color` resolves it.
        let spec = "## Nodes\n\
                    - [a] Alpha {fill:blue}\n\
                    - [b] Beta {fill:green}\n\
                    - [c] Gamma {fill:red}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("fill_builtin_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unresolved color")
        });
        assert!(!has, "exact built-in color must not emit unresolved color warning, got: {stdout}");
    }

    #[test]
    fn test_lint_unresolved_edge_color_typo_via_cli() {
        // Edge `{color:blu}` should suggest `{color:blue}`. Previously the
        // parser silently dropped the tag via `if let Some(c) = ...` with no
        // else branch, so the user had no signal the color was lost.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b {color:blu}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("edge_color_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("color:blu")
                && s.contains("did you mean")
                && s.contains("color:blue")
        });
        assert!(has, "expected edge color typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_unresolved_edge_color_no_suggestion_via_cli() {
        // Edge `{color:xyzzy}` has no close match — the lint should still
        // emit a fallback warning so the silent drop is visible to the user.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b {color:xyzzy}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("edge_color_nosug_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("color:xyzzy") && s.contains("unresolved color")
        });
        assert!(has, "expected fallback unresolved color warning, got: {stdout}");
    }

    #[test]
    fn test_lint_edge_color_exact_match_no_warning_via_cli() {
        // Edges with exact built-in colors (red, blue, green, ...) must NOT
        // emit unresolved color warnings — the parser consumes them and they
        // never reach unknown_tags.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    - [c] Gamma\n\
                    ## Flow\n\
                    a --> b {color:red}\n\
                    b --> c {color:green}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("edge_color_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unresolved color")
        });
        assert!(!has, "exact edge color must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_shape_prefix_typo_via_cli() {
        // `{shape:diamon}` should warn and suggest `{shape:diamond}`.
        // Previously `tag_to_shape` silently defaulted unknown shape values
        // to RoundedRect, wiping user intent with no lint signal.
        let spec = "## Nodes\n\
                    - [a] Alpha {shape:diamon}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("shape_prefix_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("shape:diamon")
                && s.contains("did you mean")
                && s.contains("shape:diamond")
        });
        assert!(has, "expected shape prefix typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_type_prefix_typo_via_cli() {
        // `{type:circel}` should warn and suggest `{type:circle}`. The prefix
        // should be preserved so the user can copy/paste the suggestion back.
        let spec = "## Nodes\n\
                    - [a] Alpha {type:circel}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("type_prefix_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("type:circel")
                && s.contains("did you mean")
                && s.contains("type:circle")
        });
        assert!(has, "expected type prefix typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_kind_prefix_typo_via_cli() {
        // `{kind:hexgon}` should warn and suggest `{kind:hexagon}`.
        let spec = "## Nodes\n\
                    - [a] Alpha {kind:hexgon}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("kind_prefix_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("kind:hexgon")
                && s.contains("did you mean")
                && s.contains("kind:hexagon")
        });
        assert!(has, "expected kind prefix typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_shape_prefix_no_suggestion_via_cli() {
        // `{shape:qwerty}` has no close shape alias — fallback warning must
        // still fire so the silent drop is visible to the user.
        let spec = "## Nodes\n\
                    - [a] Alpha {shape:qwerty}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("shape_prefix_nosug_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("shape:qwerty") && s.contains("not a recognized shape alias")
        });
        assert!(has, "expected fallback shape warning, got: {stdout}");
    }

    #[test]
    fn test_lint_shape_prefix_exact_no_warning_via_cli() {
        // Exact `{shape:diamond}` / `{type:circle}` / `{kind:hexagon}` must
        // NOT emit unknown-shape warnings — the parser should resolve them
        // cleanly through `tag_to_shape_opt`.
        let spec = "## Nodes\n\
                    - [a] Alpha {shape:diamond}\n\
                    - [b] Beta {type:circle}\n\
                    - [c] Gamma {kind:hexagon}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("shape_prefix_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unknown shape")
        });
        assert!(!has, "exact shape prefix match must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_align_typo_via_cli() {
        // `{align:rigth}` should warn and suggest `{align:right}`. Previously
        // the parser's catch-all `_ => Center` arm silently collapsed typos
        // into the horizontal default, wiping user intent.
        let spec = "## Nodes\n\
                    - [a] Alpha {align:rigth}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("align_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("align:rigth")
                && s.contains("did you mean")
                && s.contains("align:right")
        });
        assert!(has, "expected align typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_valign_typo_via_cli() {
        // `{valign:middel}` should warn and suggest `{valign:middle}`.
        let spec = "## Nodes\n\
                    - [a] Alpha {valign:middel}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("valign_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("valign:middel")
                && s.contains("did you mean")
                && s.contains("valign:middle")
        });
        assert!(has, "expected valign typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_align_no_suggestion_via_cli() {
        // `{align:qwerty}` has no close match — fallback warning listing the
        // canonical vocabulary must still fire so the silent drop is visible.
        let spec = "## Nodes\n\
                    - [a] Alpha {align:qwerty}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("align_nosug_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("align:qwerty")
                && s.contains("expected one of: left, right, center")
        });
        assert!(has, "expected fallback align warning, got: {stdout}");
    }

    #[test]
    fn test_lint_align_exact_values_no_warning_via_cli() {
        // Canonical + accepted-synonym values must NOT warn. Includes
        // `center` (canonical), `centre` (synonym), `left`, `right`, and
        // valign counterparts `middle`, `top`, `bottom`. Guards against
        // regressions where a synonym accidentally gets classified as a typo.
        let spec = "## Nodes\n\
                    - [a] Alpha {align:left}\n\
                    - [b] Beta {align:right}\n\
                    - [c] Gamma {align:center}\n\
                    - [d] Delta {align:centre}\n\
                    - [e] Epsilon {valign:top}\n\
                    - [f] Foxtrot {valign:bottom}\n\
                    - [g] Golf {valign:middle}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    c --> d\n\
                    d --> e\n\
                    e --> f\n\
                    f --> g\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("align_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unknown alignment")
        });
        assert!(!has, "canonical alignment values must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_numeric_opacity_non_numeric_via_cli() {
        // `{opacity:50%}` and `{opacity:half}` are common user mistakes the
        // parser used to silently swallow via `.parse::<f32>().ok()`. Both
        // should now surface as unresolved numeric-tag warnings naming the
        // expected value type.
        let spec = "## Nodes\n\
                    - [a] Alpha {opacity:50%}\n\
                    - [b] Beta {opacity:half}\n\
                    - [c] Gamma {opacity:0.5}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("opacity_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        // Both typo cases must be flagged.
        let has_pct = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("opacity:50%") && s.contains("expected") && s.contains("opacity:")
        });
        let has_word = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("opacity:half") && s.contains("expected") && s.contains("opacity:")
        });
        assert!(has_pct, "expected opacity:50% typo warning, got: {stdout}");
        assert!(has_word, "expected opacity:half typo warning, got: {stdout}");
        // The valid `opacity:0.5` must NOT be flagged.
        let has_valid = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("opacity:0.5")
        });
        assert!(!has_valid, "valid opacity:0.5 should not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_numeric_border_non_numeric_via_cli() {
        // `{border:thick}` is the common mistake — border takes a numeric
        // width, not a descriptive keyword.
        let spec = "## Nodes\n\
                    - [a] Alpha {border:thick}\n\
                    - [b] Beta {border:2.5}\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("border_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("border:thick")
                && s.contains("expected")
                && s.contains("border width")
        });
        assert!(has, "expected border:thick typo warning, got: {stdout}");
        // Valid `border:2.5` must NOT be flagged.
        let has_valid = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("border:2.5")
        });
        assert!(!has_valid, "valid border:2.5 should not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_numeric_size_non_numeric_via_cli() {
        // `{w:auto}` and `{h:abc}` and `{x:left}` / `{y:top}` are common
        // silent drops that should be surfaced as numeric-tag warnings.
        let spec = "## Nodes\n\
                    - [a] Alpha {w:auto}\n\
                    - [b] Beta {h:abc}\n\
                    - [c] Gamma {x:left}\n\
                    - [d] Delta {y:top}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    c --> d\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("size_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for (needle, expected_kind) in [
            ("w:auto", "width"),
            ("h:abc", "height"),
            ("x:left", "x coordinate"),
            ("y:top", "y coordinate"),
        ] {
            let has = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains(needle) && s.contains("expected") && s.contains(expected_kind)
            });
            assert!(has, "expected `{needle}` warning mentioning `{expected_kind}`, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_numeric_exact_values_no_warning_via_cli() {
        // Exact numeric values (opacity 0.5, border 2.5, w/h/x/y as numbers)
        // must NOT be flagged. Guards against regression where the new
        // unknown_tag path accidentally fires for valid input.
        let spec = "## Nodes\n\
                    - [a] Alpha {opacity:0.5}\n\
                    - [b] Beta {opacity:50}\n\
                    - [c] Gamma {alpha:0.2}\n\
                    - [d] Delta {border:2.5}\n\
                    - [e] Epsilon {w:200} {h:120}\n\
                    - [f] Foxtrot {x:50} {y:100}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    c --> d\n\
                    d --> e\n\
                    e --> f\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("numeric_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        // None of these numeric tag values should appear as unresolved warnings.
        for needle in ["opacity:0.5", "opacity:50", "alpha:0.2", "border:2.5",
                       "w:200", "h:120", "x:50", "y:100"] {
            let has = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("unresolved") && s.contains(needle)
            });
            assert!(!has, "valid {needle} should not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_z_non_numeric_via_cli() {
        // `{z:top}` previously parsed with `if let Ok` and silently dropped.
        // Parser now pushes to unknown_tags; cli_lint's numeric_prefix_match
        // (extended with `z:`, `3d-depth:`, `depth:`) emits the warning.
        let spec = "## Nodes\n\
                    - [a] Alpha {z:top}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("z_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("z:top") && s.contains("z offset")
        });
        assert!(has, "expected z:top numeric warning, got: {stdout}");
    }

    #[test]
    fn test_lint_3d_depth_non_numeric_via_cli() {
        // Both `{3d-depth:big}` and `{depth:thick}` silently dropped
        // pre-fix. Warnings should fire for each.
        let spec = "## Nodes\n\
                    - [a] Alpha {3d-depth:big}\n\
                    - [b] Beta {depth:thick}\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("depth_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_3d = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("3d-depth:big") && s.contains("extrusion depth")
        });
        let has_depth = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("depth:thick") && s.contains("extrusion depth")
        });
        assert!(has_3d, "expected 3d-depth:big warning, got: {stdout}");
        assert!(has_depth, "expected depth:thick warning, got: {stdout}");
    }

    #[test]
    fn test_lint_z_depth_valid_values_no_warning_via_cli() {
        // Valid numeric z/depth values must NOT trigger the unresolved
        // warning. Regression guard for the new numeric_prefix_match rows.
        let spec = "## Nodes\n\
                    - [a] Alpha {z:120}\n\
                    - [b] Beta {z:-50}\n\
                    - [c] Gamma {3d-depth:50}\n\
                    - [d] Delta {depth:100}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    c --> d\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("z_depth_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for needle in ["z:120", "z:-50", "3d-depth:50", "depth:100"] {
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("unresolved") && s.contains(needle)
            });
            assert!(!bad, "valid {needle} should not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_layer_name_typo_via_cli() {
        // `{layer:databse}` used to silently fall through `_ => z_offset`.
        // Parser now pushes to unknown_tags and suggest_layer_name points
        // at the closest canonical spelling across all 5 tier buckets.
        let spec = "## Nodes\n\
                    - [a] Alpha {layer:databse}\n\
                    - [b] Beta {tier:kubernets}\n\
                    - [c] Gamma {level:servr}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for (needle, expected) in [
            ("layer:databse", "layer:database"),
            ("tier:kubernets", "tier:kubernetes"),
            ("level:servr", "level:server"),
        ] {
            let has = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains(needle) && s.contains("did you mean") && s.contains(expected)
            });
            assert!(has, "expected `{needle}` → `{expected}`, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_layer_name_no_suggestion_via_cli() {
        // Unknown layer with no close match falls back to the explanatory
        // warning listing the 5 canonical tier groups.
        let spec = "## Nodes\n\
                    - [a] Alpha {layer:qwerty}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_nosug_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("layer:qwerty") && s.contains("not a number or recognized tier name")
        });
        assert!(has, "expected fallback layer warning, got: {stdout}");
    }

    #[test]
    fn test_lint_layer_exact_values_no_warning_via_cli() {
        // Canonical layer names across all 5 tier buckets + numeric
        // value + alternate prefixes must NOT warn. Regression guard.
        let spec = "## Nodes\n\
                    - [a] Alpha {layer:db}\n\
                    - [b] Beta {layer:api}\n\
                    - [c] Gamma {tier:frontend}\n\
                    - [d] Delta {level:edge}\n\
                    - [e] Epsilon {layer:infra}\n\
                    - [f] Foxtrot {layer:2}\n\
                    - [g] Golf {layer:database}\n\
                    - [h] Hotel {tier:kubernetes}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    c --> d\n\
                    d --> e\n\
                    e --> f\n\
                    f --> g\n\
                    g --> h\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unknown layer")
        });
        assert!(!bad, "canonical layer values must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_progress_non_numeric_via_cli() {
        // `{progress:half}`, `{pct:done}`, `{percent:almost}` — all three
        // prefixes previously used `if let Ok` without else and silently
        // dropped non-numeric values. Parser now pushes to unknown_tags
        // and cli_lint surfaces the numeric-expected warning.
        let spec = "## Nodes\n\
                    - [a] Alpha {progress:half}\n\
                    - [b] Beta {pct:done}\n\
                    - [c] Gamma {percent:almost}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("progress_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for needle in ["progress:half", "pct:done", "percent:almost"] {
            let has = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains(needle) && s.contains("unresolved")
            });
            assert!(has, "expected `{needle}` warning, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_progress_valid_values_no_warning_via_cli() {
        // Valid numeric progress values (0–100, 0.0–1.0, with/without %)
        // must NOT warn. Regression guard for the new unknown_tags path
        // and the numeric_prefix_match table row.
        let spec = "## Nodes\n\
                    - [a] Alpha {progress:50}\n\
                    - [b] Beta {progress:0.75}\n\
                    - [c] Gamma {pct:80%}\n\
                    - [d] Delta {percent:0}\n\
                    - [e] Epsilon {progress:100}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    c --> d\n\
                    d --> e\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("progress_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for needle in ["progress:50", "progress:0.75", "pct:80%", "percent:0", "progress:100"] {
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("unresolved") && s.contains(needle)
            });
            assert!(!bad, "valid {needle} should not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_gradient_angle_non_numeric_via_cli() {
        // `{gradient-angle:half}` and `{grad-angle:90deg}` should warn because
        // the gradient angle parser requires an integer 0-255. Previously the
        // arm was `if let Ok(...)` and silently dropped bad values.
        let spec = "## Nodes\n\
                    - [a] Alpha {gradient-angle:half}\n\
                    - [b] Beta {grad-angle:90deg}\n\
                    - [c] Gamma {gradient-angle:45}\n\
                    - [d] Delta {grad-angle:180}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    c --> d\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("gradient_angle_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for needle in ["gradient-angle:half", "grad-angle:90deg"] {
            let has = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("unresolved")
                    && s.contains(needle)
                    && s.contains("gradient direction")
            });
            assert!(has, "expected {needle} warning, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_gradient_angle_valid_values_no_warning_via_cli() {
        // Regression guard: valid gradient-angle values should not produce
        // `unresolved` warnings.
        let spec = "## Nodes\n\
                    - [a] Alpha {gradient-angle:0}\n\
                    - [b] Beta {gradient-angle:90}\n\
                    - [c] Gamma {gradient-angle:180}\n\
                    - [d] Delta {grad-angle:45}\n\
                    - [e] Epsilon {grad-angle:255}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    c --> d\n\
                    d --> e\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("gradient_angle_valid_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for needle in [
            "gradient-angle:0",
            "gradient-angle:90",
            "gradient-angle:180",
            "grad-angle:45",
            "grad-angle:255",
        ] {
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("unresolved") && s.contains(needle)
            });
            assert!(!bad, "valid {needle} should not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_snap_config_typo_via_cli() {
        // `snap = tru` used to silently leave snap disabled via a
        // `_ => None` fallthrough. Verify it now surfaces via the
        // existing unknown_bool_config walk with a did-you-mean hint.
        let spec = "## Config\n\
                    snap = tru\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("snap_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("snap = tru")
                && s.contains("did you mean")
                && s.contains("snap = true")
        });
        assert!(has, "expected snap=tru typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_bg_color_typo_via_cli() {
        // `bg-color = blu` / `canvas-bg = gren` / `background = yelow` all
        // used to silently leave canvas_bg unset. Verify cli_lint now emits
        // did-you-mean hints via suggest_fill_color_name.
        let spec = "## Config\n\
                    bg-color = blu\n\
                    canvas-bg = gren\n\
                    background = yelow\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("bg_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for (key, typo, expected) in [
            ("bg-color",   "blu",   "blue"),
            ("canvas-bg",  "gren",  "green"),
            ("background", "yelow", "yellow"),
        ] {
            let has = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains(&format!("{key} = {typo}"))
                    && s.contains("did you mean")
                    && s.contains(&format!("{key} = {expected}"))
            });
            assert!(has, "expected {key}={typo} → {expected} warning, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_snap_bg_valid_values_no_warning_via_cli() {
        // Regression guard: canonical snap + bg-color values must not warn.
        let spec = "## Config\n\
                    snap = true\n\
                    bg-color = #1e1e2e\n\
                    canvas-bg = surface\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("snap_bg_valid_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for needle in ["snap = true", "bg-color = #1e1e2e", "canvas-bg = surface"] {
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                (s.contains("not recognized") || s.contains("not a recognized color"))
                    && s.contains(needle)
            });
            assert!(!bad, "valid {needle} should not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_bg_pattern_typo_via_cli() {
        // `bg = dts` (typo for `dots`) and `bg-pattern = crosshach` (typo for
        // `crosshatch`) used to silently fall back to `BgPattern::None` or
        // keep the previous pattern after import, with no user feedback.
        // Verify they now surface as lint warnings with did-you-mean hints.
        let spec = "## Config\n\
                    bg = dts\n\
                    bg-pattern = crosshach\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("bg_pattern_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("bg = dts") && joined.contains("did you mean `bg = dots`"),
            "expected bg=dts did-you-mean hint, got: {stdout}"
        );
        assert!(
            joined.contains("bg-pattern = crosshach")
                && joined.contains("did you mean `bg-pattern = crosshatch`"),
            "expected bg-pattern=crosshach did-you-mean hint, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_bg_pattern_nonsense_expected_vocabulary_via_cli() {
        // Unresolvable values like `bg = zzzqqq` should still warn, falling
        // back to listing the accepted vocabulary rather than a did-you-mean.
        let spec = "## Config\n\
                    bg = zzzqqq\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("bg_pattern_nonsense_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("bg = zzzqqq")
                && joined.contains("expected one of")
                && joined.contains("dots")
                && joined.contains("lines")
                && joined.contains("crosshatch"),
            "expected accepted-vocabulary fallback, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_bg_pattern_valid_values_no_warning_via_cli() {
        // Regression guard: canonical bg / bg-pattern values must not warn.
        let spec = "## Config\n\
                    bg = dots\n\
                    bg-pattern = lines\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("bg_pattern_valid_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for needle in ["bg = dots", "bg-pattern = lines"] {
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("not a recognized pattern") && s.contains(needle)
            });
            assert!(!bad, "valid {needle} should not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_dep_target_typo_via_cli() {
        // `{dep:auth_servce}` (typo for `auth_service`) used to silently
        // drop the dependency edge with no warning. Verify cli_lint now
        // emits a did-you-mean hint against the known node id vocabulary.
        let spec = "## Nodes\n\
                    - [auth_service] Auth Service\n\
                    - [db] Database\n\
                    - [api] API Gateway {dep:auth_servce} {dep:db}\n\
                    ## Flow\n\
                    api --> db\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("dep_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("{dep:auth_servce}")
                && joined.contains("did you mean `{dep:auth_service}`"),
            "expected dep:auth_servce did-you-mean hint, got: {stdout}"
        );
        // Valid `{dep:db}` on the same node must NOT warn.
        let bad_valid = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("{dep:db}") && s.contains("does not resolve")
        });
        assert!(!bad_valid, "valid {{dep:db}} must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_dep_target_nonsense_falls_back_via_cli() {
        // Unresolvable dep targets with no close match (e.g. `xyznonsense`)
        // should still warn, falling back to a generic "does not resolve
        // to any node" message instead of a did-you-mean suggestion.
        let spec = "## Nodes\n\
                    - [a] Alpha {dep:xyznonsense}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("dep_nonsense_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("{dep:xyznonsense}")
                && joined.contains("does not resolve to any node in ## Nodes"),
            "expected generic fallback for unresolvable dep, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_dep_target_valid_no_warning_via_cli() {
        // Regression guard: a valid `{dep:X}` pointing at an existing node
        // must not warn. Tests both HRF-id lookup and label-slug fallback.
        let spec = "## Nodes\n\
                    - [auth_service] Auth Service\n\
                    - [api] API Gateway {dep:auth_service}\n\
                    - [db] Database\n\
                    ## Flow\n\
                    api --> db\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("dep_valid_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("{dep:auth_service}") && s.contains("does not resolve")
        });
        assert!(!bad, "valid {{dep:auth_service}} must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_palette_value_nonsense_falls_back_via_cli() {
        // Unresolvable palette values with no close match (e.g. `zzzqqq`)
        // should still warn, falling back to a generic "expected a color
        // name or hex" message instead of a did-you-mean suggestion. This
        // complements the `test_lint_palette_value_typo_via_cli` test that
        // covers the did-you-mean path — together they exercise both
        // branches of the `suggest_fill_color_name` fallback logic.
        let spec = "## Palette\n\
                    accent = zzzqqq\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("palette_nonsense_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("accent = zzzqqq")
                && joined.contains("expected a color name"),
            "expected generic fallback for unresolvable palette value, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_layer_value_tier_typo_via_cli() {
        // `## Layers` with a tier name typo like `api = backned` used to
        // silently drop from the layer map — no warning, and any
        // `{layer:api}` references in Flow stayed as literal tags.
        // Verify the typo now surfaces with a did-you-mean hint.
        let spec = "## Layers\n\
                    api = backned\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_tier_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("api = backned") && joined.contains("did you mean"),
            "expected layer tier typo did-you-mean hint, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_layer_value_nonsense_expected_vocabulary_via_cli() {
        // Unresolvable layer values like `svc = zzzqqq` should warn with
        // the accepted-vocabulary fallback.
        let spec = "## Layers\n\
                    svc = zzzqqq\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_nonsense_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("svc = zzzqqq") && joined.contains("expected a number"),
            "expected accepted-vocabulary fallback, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_layer_value_valid_not_flagged_via_cli() {
        // Regression guard: valid layer values (numbers and canonical tier
        // names) must not fire the warning.
        for (name, val) in [
            ("data", "0"),
            ("app", "120"),
            ("ui", "240"),
            ("db", "database"),
            ("svc", "backend"),
            ("web", "frontend"),
            ("edge", "gateway"),
            ("infra", "platform"),
        ] {
            let spec = format!(
                "## Layers\n\
                 {name} = {val}\n\
                 ## Nodes\n\
                 - [a] Alpha\n\
                 - [b] Beta\n\
                 ## Flow\n\
                 a --> b\n"
            );
            let uid = uuid::Uuid::new_v4();
            let tmp = std::env::temp_dir().join(format!("layer_ok_{}_{}.spec", name, uid));
            std::fs::write(&tmp, &spec).unwrap();
            let bin = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .map(|p| p.join("open-draftly"));
            let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
            if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
            let out = std::process::Command::new(&bin)
                .args(["lint", "--json", tmp.to_str().unwrap()])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let _ = std::fs::remove_file(&tmp);
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
            let warnings = v["warnings"].as_array().expect("warnings array");
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("## Layers") && s.contains("not a number or known tier")
            });
            assert!(!bad, "`{name} = {val}` should not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_palette_value_typo_via_cli() {
        // `## Palette` with a color-name typo like `brand = reed` used to
        // silently drop from the palette map — no warning, and any
        // `{fill:brand}` references also silent-fell-through. Verify the
        // typo now surfaces with a did-you-mean hint.
        let spec = "## Palette\n\
                    brand = reed\n\
                    ## Nodes\n\
                    - [a] Alpha {fill:brand}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("palette_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("brand = reed")
                && joined.contains("## Palette")
                && joined.contains("did you mean")
                && joined.contains("red"),
            "expected palette color typo did-you-mean hint, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_palette_invalid_hex_via_cli() {
        // `## Palette` with a bad hex value should emit a dedicated
        // "not a valid hex color" warning (not a color-name suggestion).
        let spec = "## Palette\n\
                    accent = #zzghij\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("palette_hex_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("accent = #zzghij")
                && joined.contains("not a valid hex"),
            "expected palette hex warning, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_palette_valid_values_not_flagged_via_cli() {
        // Regression guard: canonical palette values (named colors and valid
        // hex) must not fire the warning.
        let spec = "## Palette\n\
                    primary = blue\n\
                    danger = red\n\
                    accent = #f38ba8\n\
                    soft = #fce\n\
                    ## Nodes\n\
                    - [a] Alpha {fill:primary}\n\
                    - [b] Beta {fill:danger}\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("palette_ok_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("## Palette")
                && (s.contains("not a recognized color") || s.contains("not a valid hex"))
        });
        assert!(!bad, "canonical palette values should not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_grid_cols_typo_via_cli() {
        // `## Grid cols=fve` used to silently fall back to a 3-column
        // grid via `.unwrap_or(3)`, dropping the user's intended value
        // with no warning. Verify the typo now surfaces in lint output.
        let spec = "## Grid cols=fve\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    - [c] Gamma\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("grid_cols_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("## Grid")
                && joined.contains("cols=fve")
                && joined.contains("positive integer"),
            "expected grid cols typo warning, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_matrix_and_table_cols_typo_via_cli() {
        // `## Matrix cols=X` and `## Table cols=X` share the same code
        // path as Grid and must surface typos under their own alias so
        // users see the section name they actually typed.
        for (header, expected_alias) in [
            ("## Matrix cols=fore\n", "## Matrix"),
            ("## Table cols=tree\n",  "## Table"),
        ] {
            let spec = format!("{header}- [a] Alpha\n- [b] Beta\n");
            let uid = uuid::Uuid::new_v4();
            let tmp = std::env::temp_dir()
                .join(format!("grid_alias_typo_{}_{}.spec", expected_alias.replace("## ", ""), uid));
            std::fs::write(&tmp, &spec).unwrap();
            let bin = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .map(|p| p.join("open-draftly"));
            let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
            if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
            let out = std::process::Command::new(&bin)
                .args(["lint", "--json", tmp.to_str().unwrap()])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let _ = std::fs::remove_file(&tmp);
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
            let warnings = v["warnings"].as_array().expect("warnings array");
            let joined: String = warnings.iter()
                .filter_map(|w| w.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                joined.contains(expected_alias) && joined.contains("positive integer"),
                "expected `{expected_alias}` cols typo warning, got: {stdout}"
            );
        }
    }

    #[test]
    fn test_lint_grid_cols_valid_not_flagged_via_cli() {
        // Regression guard: valid cols values and bare-number forms and
        // missing cols (empty rest) must NOT emit the warning.
        for spec in [
            "## Grid cols=4\n- [a] Alpha\n- [b] Beta\n",
            "## Grid 5\n- [a] Alpha\n- [b] Beta\n",
            "## Grid\n- [a] Alpha\n- [b] Beta\n",
            "## Matrix cols=2\n- [a] Alpha\n- [b] Beta\n",
            "## Table columns=3\n- [a] Alpha\n- [b] Beta\n",
        ] {
            let uid = uuid::Uuid::new_v4();
            let tmp = std::env::temp_dir().join(format!("grid_ok_{}.spec", uid));
            std::fs::write(&tmp, spec).unwrap();
            let bin = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .map(|p| p.join("open-draftly"));
            let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
            if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
            let out = std::process::Command::new(&bin)
                .args(["lint", "--json", tmp.to_str().unwrap()])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let _ = std::fs::remove_file(&tmp);
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
            let warnings = v["warnings"].as_array().expect("warnings array");
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("positive integer column count")
            });
            assert!(!bad, "valid grid cols should not warn: spec={spec:?} out={stdout}");
        }
    }

    #[test]
    fn test_lint_layer_z_typo_via_cli() {
        // `## Layer z=abc: Frontend` used to silently fall back to z=0.0
        // (colliding with the default layer) with no user feedback. Verify
        // the typo now surfaces in lint output.
        let spec = "## Layer z=abc: Frontend\n\
                    - [a] Alpha\n\
                    - [b] Beta\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_z_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings
            .iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("## Layer")
                && joined.contains("z=abc")
                && joined.contains("not a valid layer index"),
            "expected layer z typo warning, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_layer_bare_typo_via_cli() {
        // `## Layer foo` (bare non-number) used to silently fall back to
        // z=0.0 just like the explicit z=X form. Verify the bare typo
        // surfaces via lint under the same warning code path.
        let spec = "## Layer foo\n\
                    - [a] Alpha\n\
                    - [b] Beta\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_bare_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings
            .iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("## Layer")
                && joined.contains("`foo`")
                && joined.contains("not a valid layer index"),
            "expected bare layer typo warning, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_layer_valid_not_flagged_via_cli() {
        // Regression guard: all legal layer forms must NOT emit a warning.
        // Includes `z:N` which was a pre-existing parser bug fixed alongside
        // the lint — `## Layer z:240` now parses correctly to z=240.
        for spec in [
            "## Layer 1\n- [a] Alpha\n",
            "## Layer z=120\n- [a] Alpha\n",
            "## Layer z:240\n- [a] Alpha\n",
            "## Layer 120\n- [a] Alpha\n",
            "## Layer\n- [a] Alpha\n",
            "## Layer 2: Backend\n- [a] Alpha\n",
            "## Layer z=240: Frontend\n- [a] Alpha\n",
        ] {
            let uid = uuid::Uuid::new_v4();
            let tmp = std::env::temp_dir().join(format!("layer_ok_{}.spec", uid));
            std::fs::write(&tmp, spec).unwrap();
            let bin = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .map(|p| p.join("open-draftly"));
            let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
            if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
            let out = std::process::Command::new(&bin)
                .args(["lint", "--json", tmp.to_str().unwrap()])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let _ = std::fs::remove_file(&tmp);
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
            let warnings = v["warnings"].as_array().expect("warnings array");
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("not a valid layer index")
            });
            assert!(!bad, "valid layer form should not warn: spec={spec:?} out={stdout}");
        }
    }

    #[test]
    fn test_lint_period_idx_typo_via_cli() {
        // `## Period two: Q2 2026` silently placed "Q2 2026" at position
        // 0 via `.unwrap_or(0)`. Verify the typo now surfaces in lint.
        let spec = "## Period two: Q2 2026\n\
                    - [a] Alpha\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("period_idx_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings
            .iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("## Period")
                && joined.contains("`two`")
                && joined.contains("not a valid index"),
            "expected period idx typo warning, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_lane_idx_typo_via_cli() {
        // `## Lane three: Engineering` silently placed "Engineering" at
        // lane 0, and `## Lane foo` created a phantom "Lane 0" label.
        // Verify both variants surface in lint.
        for (spec, expected_raw) in [
            ("## Lane three: Engineering\n- [a] Alpha\n", "`three`"),
            ("## Lane foo\n- [a] Alpha\n",                "`foo`"),
        ] {
            let uid = uuid::Uuid::new_v4();
            let tmp = std::env::temp_dir().join(format!("lane_idx_typo_{}.spec", uid));
            std::fs::write(&tmp, spec).unwrap();
            let bin = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .map(|p| p.join("open-draftly"));
            let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
            if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
            let out = std::process::Command::new(&bin)
                .args(["lint", "--json", tmp.to_str().unwrap()])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let _ = std::fs::remove_file(&tmp);
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
            let warnings = v["warnings"].as_array().expect("warnings array");
            let joined: String = warnings
                .iter()
                .filter_map(|w| w.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                joined.contains("## Lane")
                    && joined.contains(expected_raw)
                    && joined.contains("not a valid index"),
                "expected lane idx typo warning for {expected_raw}, got: {stdout}"
            );
        }
    }

    #[test]
    fn test_lint_period_lane_valid_not_flagged_via_cli() {
        // Regression guard: legal Period/Lane index forms must NOT warn.
        for spec in [
            "## Period 1: Q1 2026\n- [a] Alpha\n",
            "## Period 2\n- [a] Alpha\n",
            "## Lane 1: Engineering\n- [a] Alpha\n",
            "## Lane 3\n- [a] Alpha\n",
        ] {
            let uid = uuid::Uuid::new_v4();
            let tmp = std::env::temp_dir().join(format!("period_lane_ok_{}.spec", uid));
            std::fs::write(&tmp, spec).unwrap();
            let bin = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .map(|p| p.join("open-draftly"));
            let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
            if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
            let out = std::process::Command::new(&bin)
                .args(["lint", "--json", tmp.to_str().unwrap()])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let _ = std::fs::remove_file(&tmp);
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
            let warnings = v["warnings"].as_array().expect("warnings array");
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                (s.contains("## Period:") || s.contains("## Lane:"))
                    && s.contains("not a valid index")
            });
            assert!(!bad, "valid period/lane form should not warn: spec={spec:?} out={stdout}");
        }
    }

    #[test]
    fn test_lint_group_fill_typo_via_cli() {
        // `## Groups` {fill:X} typos (`{fill:blu}`, `{fill:gren}`) used to
        // silently drop to the default frame color. Verify they now surface
        // as lint warnings with did-you-mean hints.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    - [c] Gamma\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    ## Groups\n\
                    - [g1] Backend {fill:blu}\n\
                    \x20\x20a, b\n\
                    - [g2] Frontend {fill:gren}\n\
                    \x20\x20c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("group_fill_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_blu = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("g1")
                && s.contains("fill:blu")
                && s.contains("did you mean")
                && s.contains("fill:blue")
        });
        let has_gren = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("g2")
                && s.contains("fill:gren")
                && s.contains("did you mean")
                && s.contains("fill:green")
        });
        assert!(has_blu, "expected g1 fill:blu typo warning, got: {stdout}");
        assert!(has_gren, "expected g2 fill:gren typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_group_fill_valid_no_warning_via_cli() {
        // Regression guard: valid group fill colors should not produce any
        // `unresolved` warnings.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n\
                    ## Groups\n\
                    - [g1] Backend {fill:blue}\n\
                    \x20\x20a\n\
                    - [g2] Frontend {fill:green}\n\
                    \x20\x20b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("group_fill_valid_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for needle in ["fill:blue", "fill:green"] {
            let bad = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("unresolved") && s.contains(needle)
            });
            assert!(!bad, "valid {needle} should not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_status_typo_via_cli() {
        // `{status:doen}` should warn and suggest `{status:done}`. Previously
        // the parser's `status:` arm fell through to `tag_to_node_tag` which
        // returned None for unrecognized values, silently dropping the tag.
        let spec = "## Nodes\n\
                    - [a] Alpha {status:doen}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("status_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("status:doen")
                && s.contains("did you mean")
                && s.contains("status:done")
        });
        assert!(has, "expected status:doen typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_status_typo_multi_char_via_cli() {
        // `{status:blokced}` has d=2 from `blocked` — should still suggest.
        // Also tests `in-progres` (missing final s) and `revew` (missing i).
        let spec = "## Nodes\n\
                    - [a] Alpha {status:blokced}\n\
                    - [b] Beta {status:in-progres}\n\
                    - [c] Gamma {status:revew}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("status_multi_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for (needle, expected) in [
            ("status:blokced", "status:blocked"),
            ("status:in-progres", "status:in-progress"),
            ("status:revew", "status:review"),
        ] {
            let has = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains(needle) && s.contains("did you mean") && s.contains(expected)
            });
            assert!(has, "expected `{needle}` → `{expected}` warning, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_status_no_suggestion_via_cli() {
        // `{status:qwerty}` has no close match — fallback warning must fire.
        let spec = "## Nodes\n\
                    - [a] Alpha {status:qwerty}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("status_nosug_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("status:qwerty") && s.contains("not a recognized status value")
        });
        assert!(has, "expected fallback status warning, got: {stdout}");
    }

    #[test]
    fn test_lint_status_exact_values_no_warning_via_cli() {
        // Canonical status values and accepted synonyms must NOT trigger
        // the unknown-status warning. Covers the main vocabulary buckets:
        // done, wip, review, blocked, todo, plus info (via tag_to_node_tag).
        let spec = "## Nodes\n\
                    - [a] Alpha {status:done}\n\
                    - [b] Beta {status:wip}\n\
                    - [c] Gamma {status:review}\n\
                    - [d] Delta {status:blocked}\n\
                    - [e] Epsilon {status:todo}\n\
                    - [f] Foxtrot {status:in-progress}\n\
                    - [g] Golf {status:info}\n\
                    ## Flow\n\
                    a --> b\n\
                    b --> c\n\
                    c --> d\n\
                    d --> e\n\
                    e --> f\n\
                    f --> g\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("status_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unknown status")
        });
        assert!(!has, "canonical status values must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_edge_bend_non_numeric_via_cli() {
        // Edge `{bend:medium}` previously parsed with `.ok()` and was
        // silently dropped. Parser now pushes the raw tag to
        // edge.unknown_tags; cli_lint emits a numeric-expected warning.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b {bend:medium}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("edge_bend_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("bend:medium") && s.contains("curve bend")
        });
        assert!(has, "expected bend numeric warning, got: {stdout}");
    }

    #[test]
    fn test_lint_edge_weight_non_numeric_via_cli() {
        // Edge `{weight:heavy}` and `{w:thick}` both silently dropped
        // pre-fix. Each should now trigger a dedicated warning.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    - [c] Gamma\n\
                    ## Flow\n\
                    a --> b {weight:heavy}\n\
                    b --> c {w:thick}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("edge_weight_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_weight = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("weight:heavy") && s.contains("edge weight")
        });
        let has_w = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("w:thick") && s.contains("edge weight")
        });
        assert!(has_weight, "expected weight:heavy warning, got: {stdout}");
        assert!(has_w, "expected w:thick warning, got: {stdout}");
    }

    #[test]
    fn test_lint_edge_numeric_valid_values_no_warning_via_cli() {
        // Valid numeric values for bend/weight/w must NOT trigger the
        // unknown-edge warning. Regression guard for legit diagrams.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    - [c] Gamma\n\
                    - [d] Delta\n\
                    ## Flow\n\
                    a --> b {bend:0.5}\n\
                    b --> c {weight:2}\n\
                    c --> d {w:3}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("edge_numeric_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            (s.contains("bend:") || s.contains("weight:") || s.contains("w:"))
                && s.contains("unresolved")
        });
        assert!(!bad, "valid numeric edge values must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_edge_cardinality_typo_via_cli() {
        // `{c-src:1..Z}` and `{c-tgt:zero}` should both fire the
        // unknown-cardinality warning. `1..Z` is within Levenshtein-2
        // of canonical `1..N`, so the first should suggest it; `zero`
        // is too far from any canonical form, so the second should
        // fall through to the "expected 1, 0..1, ..." message.
        // Pre-fix, parse_cardinality silently collapsed both to
        // Cardinality::None, dropping the user's intent.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    - [c] Gamma\n\
                    ## Flow\n\
                    a --> b {c-src:1..Z}\n\
                    b --> c {c-tgt:zero}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("cardinality_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        // Parser lowercases tags before storing them, so the raw tag
        // in the warning is `c-src:1..z`. The suggestion preserves
        // canonical casing (`1..N`).
        let has_suggest = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("c-src:1..z")
                && s.contains("did you mean")
                && s.contains("c-src:1..N")
        });
        let has_unknown = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("c-tgt:zero") && s.contains("unknown cardinality")
        });
        assert!(has_suggest, "expected c-src:1..z suggestion, got: {stdout}");
        assert!(has_unknown, "expected c-tgt:zero unknown warning, got: {stdout}");
    }

    #[test]
    fn test_lint_edge_cardinality_valid_values_no_warning_via_cli() {
        // Canonical cardinality values must NOT trigger the unknown
        // warning. Regression guard for `1`, `0..1`, `1..N`, `0..N`,
        // `1..*`, and `0..*` across both c-src: and c-tgt:.
        let spec = "## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    - [c] Gamma\n\
                    - [d] Delta\n\
                    - [e] Epsilon\n\
                    - [f] Phi\n\
                    - [g] Gamma2\n\
                    ## Flow\n\
                    a --> b {c-src:1} {c-tgt:0..1}\n\
                    b --> c {c-src:1..N} {c-tgt:0..N}\n\
                    c --> d {c-src:1..*} {c-tgt:0..*}\n\
                    d --> e\n\
                    e --> f\n\
                    f --> g\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("cardinality_exact_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unknown cardinality")
        });
        assert!(!bad, "valid cardinality values must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_unresolved_text_color_typo_via_cli() {
        // `{text-color:blu}` should suggest `{text-color:blue}`. Previously
        // the color lint walk only checked `fill:`/`color:`/`border-color:`/
        // `stroke:` — `text-color:` fell through unmatched and silently
        // dropped at parse time. The prefix list now includes `text-color:`
        // (with `color:` listed last so the longer prefix wins).
        let spec = "## Nodes\n\
                    - [a] Alpha {text-color:blu}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("text_color_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("text-color:blu")
                && s.contains("did you mean")
                && s.contains("text-color:blue")
        });
        assert!(has, "expected text-color typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_unresolved_frame_color_typo_via_cli() {
        // `{frame-color:blu}` should suggest `{frame-color:blue}`. The
        // parser's frame-color/frame-fill/bg-color arm used to silently
        // drop unresolved values; now it preserves them in unknown_tags
        // and cli_lint surfaces the typo via the shared color walk.
        // frame-color applies at the node level via `{frame}`; test a
        // frame-tagged container node directly.
        let spec = "## Nodes\n\
                    - [team] Team {frame} {frame-color:blu}\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("frame_color_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("frame-color:blu")
                && s.contains("did you mean")
                && s.contains("frame-color:blue")
        });
        assert!(has, "expected frame-color typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_numeric_size_shorthand_silent_drop_via_cli() {
        // `{size:big}` used to silently drop (no `x` separator or bad parts).
        let spec = "## Nodes\n\
                    - [a] Alpha {size:big}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("size_big_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("size:big") && s.contains("WxH")
        });
        assert!(has, "expected size shorthand warning, got: {stdout}");
    }

    #[test]
    fn test_lint_numeric_font_size_silent_drop_via_cli() {
        // `{font-size:large}` used to silently drop.
        let spec = "## Nodes\n\
                    - [a] Alpha {font-size:large}\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("font_size_large_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("font-size:large") && s.contains("font size")
        });
        assert!(has, "expected font-size numeric warning, got: {stdout}");
    }

    #[test]
    fn test_lint_near_duplicate_hrf_ids_flagged() {
        // Two HRF IDs that differ by a single character (and aren't
        // numbered siblings or inflectional variants) should surface as
        // a typo warning.
        let spec = "## Nodes\n- [order] Order\n- [oder] Oder\n## Flow\norder --> oder\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("near_dup_id_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("typo variants")
                && s.contains("order")
                && s.contains("oder")
        });
        assert!(has, "expected near-duplicate HRF ID warning, got: {stdout}");
    }

    #[test]
    fn test_lint_near_duplicate_hrf_ids_numbered_series_skipped() {
        // `svc1`/`svc2`/`svc3` is a legitimate numbered series; the lint
        // must NOT flag it as typo variants.
        let spec = "## Nodes\n- [svc1] Svc 1\n- [svc2] Svc 2\n- [svc3] Svc 3\n## Flow\nsvc1 --> svc2\nsvc2 --> svc3\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("numbered_svc_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("typo variants")
        });
        assert!(!has, "numbered series must not fire, got: {stdout}");
    }

    #[test]
    fn test_lint_near_duplicate_hrf_ids_inflection_skipped() {
        // `capture`/`captured` and `authorize`/`authorized` are prefix-
        // related inflectional variants that legitimately coexist in
        // state machines. The lint must NOT flag them.
        let spec = "## Nodes\n- [capture] Capture\n- [captured] Captured\n- [authorize] Authorize\n- [authorized] Authorized\n## Flow\ncapture --> captured\nauthorize --> authorized\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("inflection_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("typo variants")
        });
        assert!(!has, "inflectional variants must not fire, got: {stdout}");
    }

    #[test]
    fn test_lint_near_duplicate_hrf_ids_short_skipped() {
        // Three-char IDs are too ambiguous to safely flag — `api` vs
        // `ipa` could be an ordering flip OR two legitimate acronyms.
        let spec = "## Nodes\n- [api] API\n- [ipa] IPA\n## Flow\napi --> ipa\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("short_ids_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("typo variants")
        });
        assert!(!has, "short (<4 char) IDs must not fire, got: {stdout}");
    }

    #[test]
    fn test_suggest_layout_direction_short_typos() {
        use crate::specgraph::hrf::suggest_layout_direction;
        // Single-character typos on 2-letter codes resolve confidently.
        assert_eq!(suggest_layout_direction("TR"), Some("TB"));
        assert_eq!(suggest_layout_direction("BR"), Some("BT"));
        assert_eq!(suggest_layout_direction("LF"), Some("LR"));
        assert_eq!(suggest_layout_direction("RR"), Some("RL"));
    }

    #[test]
    fn test_suggest_layout_direction_long_form_typos() {
        use crate::specgraph::hrf::suggest_layout_direction;
        // Long forms with a typo collapse to the canonical short code.
        assert_eq!(suggest_layout_direction("TOP-BOTOM"), Some("TB"));
        assert_eq!(suggest_layout_direction("LEFT-RIGTH"), Some("LR"));
        assert_eq!(suggest_layout_direction("HORIZNTAL"), Some("LR"));
    }

    #[test]
    fn test_suggest_layout_direction_exact_matches_return_none() {
        use crate::specgraph::hrf::suggest_layout_direction;
        for known in ["TB", "BT", "LR", "RL", "tb", "Lr"] {
            assert_eq!(
                suggest_layout_direction(known),
                None,
                "exact-match direction '{known}' must not get a suggestion"
            );
        }
    }

    #[test]
    fn test_suggest_layout_direction_rejects_unrelated_junk() {
        use crate::specgraph::hrf::suggest_layout_direction;
        assert_eq!(suggest_layout_direction(""), None);
        assert_eq!(suggest_layout_direction("XYZZY"), None);
        assert_eq!(suggest_layout_direction("DIAGONAL"), None);
    }

    #[test]
    fn test_parse_captures_unknown_layout_direction() {
        // `flow = TR` must populate import_hints.unknown_layout_direction
        // with the raw value and a TB suggestion, and still default to TB.
        let spec = "## Config\nflow = TR\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert_eq!(doc.layout_dir, "TB", "unknown dir should fall back to TB");
        let unknown = &doc.import_hints.unknown_layout_direction;
        assert_eq!(unknown.len(), 1, "expected one unknown-direction entry");
        assert_eq!(unknown[0].0, "TR");
        assert_eq!(unknown[0].1, "TB");
    }

    #[test]
    fn test_parse_known_direction_does_not_populate_unknown() {
        // Canonical `flow = LR` must NOT populate the unknown list.
        let spec = "## Config\nflow = LR\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert_eq!(doc.layout_dir, "LR");
        assert!(
            doc.import_hints.unknown_layout_direction.is_empty(),
            "known direction should not populate unknown list, got: {:?}",
            doc.import_hints.unknown_layout_direction
        );
    }

    #[test]
    fn test_lint_unknown_layout_direction_via_cli() {
        // End-to-end: `flow = TR` should surface as a warning via lint --json,
        // with the "did you mean TB" phrasing.
        let spec = "## Config\nflow = TR\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("flow_dir_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_dir = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Unknown layout direction")
                && s.contains("TR")
                && s.contains("did you mean")
                && s.contains("TB")
        });
        assert!(has_dir, "expected unknown-direction warning, got: {stdout}");
    }

    #[test]
    fn test_lint_known_layout_direction_not_flagged() {
        // `flow = TB` must not fire the warning.
        let spec = "## Config\nflow = TB\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("flow_dir_ok_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_dir = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("Unknown layout direction")
        });
        assert!(!has_dir, "known direction must not fire, got: {stdout}");
    }

    #[test]
    fn test_lint_unknown_camera_preset_typo_via_cli() {
        // `camera = ios` (typo for `iso`) and `camera = isometrci` (typo
        // for `isometric`) used to silently leave camera_yaw/pitch
        // unchanged. Parser now pushes to unknown_camera_preset and
        // cli_lint surfaces a did-you-mean suggestion from the canonical
        // vocabulary (iso/top/front/side + synonyms).
        let spec = "## Config\ncamera = isometrci\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("cam_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unknown camera preset")
                && s.contains("isometrci")
                && s.contains("did you mean")
                && s.contains("isometric")
        });
        assert!(has, "expected camera preset typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_unknown_camera_preset_no_suggestion_via_cli() {
        // A far-off preset name should fall back to the explanatory
        // warning listing the 4 canonical buckets.
        let spec = "## Config\ncamera = xyzzy\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("cam_nosug_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("unknown camera preset")
                && s.contains("xyzzy")
                && s.contains("expected iso, top, front, or side")
        });
        assert!(has, "expected camera preset fallback warning, got: {stdout}");
    }

    #[test]
    fn test_lint_known_camera_presets_not_flagged_via_cli() {
        // Canonical preset names + synonyms must NOT fire the unknown
        // camera warning. Regression guard across all 4 buckets.
        let specs = [
            "## Config\ncamera = iso\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n",
            "## Config\ncamera = isometric\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n",
            "## Config\ncamera = top\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n",
            "## Config\ncamera = front\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n",
            "## Config\ncamera = side\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n",
            "## Config\ncam = default\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n",
        ];
        for (i, spec) in specs.iter().enumerate() {
            let uid = uuid::Uuid::new_v4();
            let tmp = std::env::temp_dir().join(format!("cam_ok_{}_{}.spec", i, uid));
            std::fs::write(&tmp, spec).unwrap();
            let bin = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .map(|p| p.join("open-draftly"));
            let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
            if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
            let out = std::process::Command::new(&bin)
                .args(["lint", "--json", tmp.to_str().unwrap()])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let _ = std::fs::remove_file(&tmp);
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
            let warnings = v["warnings"].as_array().expect("warnings array");
            let bad = warnings.iter().any(|w| {
                w.as_str().unwrap_or("").contains("unknown camera preset")
            });
            assert!(!bad, "canonical preset spec #{i} must not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_bool_config_typo_via_cli() {
        // `timeline = tru`, `auto-z = ye`, `auto-tier-color = onn` all
        // used to silently fall through the `_ => {}` arm leaving the
        // flag off. Parser now records the raw value and cli_lint
        // surfaces did-you-mean suggestions from the canonical boolean
        // vocabulary (true/false/yes/no/on/off).
        let spec = "## Config\n\
                    timeline = tru\n\
                    auto-z = ye\n\
                    auto-tier-color = onn\n\
                    ## Nodes\n\
                    - [a] A\n\
                    - [b] B\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("bool_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        for (key, bad, good) in [
            ("timeline", "tru", "true"),
            ("auto-z", "ye", "yes"),
            ("auto-tier-color", "onn", "on"),
        ] {
            let has = warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains(&format!("{} = {}", key, bad))
                    && s.contains("did you mean")
                    && s.contains(&format!("{} = {}", key, good))
            });
            assert!(has, "expected `{key} = {bad}` → `{good}`, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_view_config_typo_via_cli() {
        // `view = threedd` is within Levenshtein distance 2 of `threed`
        // and should suggest that canonical spelling. `view = 3-d` is
        // similarly within 2 of `3d`.
        let spec = "## Config\nview = threedd\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("view_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("view = threedd")
                && s.contains("did you mean")
                && s.contains("view = threed")
        });
        assert!(has, "expected view typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_bool_config_no_suggestion_via_cli() {
        // Far-off boolean value should fall back to the explanatory
        // warning listing the canonical tokens.
        let spec = "## Config\ntimeline = xyzzy\n## Nodes\n- [a] A\n- [b] B\n## Flow\na --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("bool_nosug_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("timeline = xyzzy")
                && s.contains("expected true/false, yes/no, on/off, or 1/0")
        });
        assert!(has, "expected bool fallback warning, got: {stdout}");
    }

    #[test]
    fn test_lint_bool_config_valid_values_not_flagged_via_cli() {
        // Canonical boolean spellings across all 4 keys must NOT fire.
        let spec = "## Config\n\
                    timeline = true\n\
                    auto-z = yes\n\
                    auto-tier-color = on\n\
                    view = 3d\n\
                    ## Nodes\n\
                    - [a] A\n\
                    - [b] B\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("bool_ok_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("not recognized")
        });
        assert!(!bad, "canonical bool/view values must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_numeric_config_non_numeric_via_cli() {
        // Typos like `grid = small`, `camera_yaw = tilted`, `gap = wide`,
        // `sla-p1 = three` used to silently drop because the parser did
        // `if let Ok(v) = val.parse::<f32>()` with no else branch. They
        // should now emit "is not a number" warnings so the author knows
        // their directive was ignored.
        let spec = "## Config\n\
                    grid = small\n\
                    camera_yaw = tilted\n\
                    gap = wide\n\
                    sla-p1 = three\n\
                    ## Nodes\n\
                    - [a] A\n\
                    - [b] B\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("num_bad_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_grid = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("grid") && s.contains("small") && s.contains("not a number")
        });
        let has_yaw = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("camera_yaw") && s.contains("tilted") && s.contains("not a number")
        });
        let has_gap = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("gap") && s.contains("wide") && s.contains("not a number")
        });
        let has_sla = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("sla-p1") && s.contains("three") && s.contains("not a number")
        });
        assert!(has_grid, "expected grid numeric warning, got: {stdout}");
        assert!(has_yaw, "expected camera_yaw numeric warning, got: {stdout}");
        assert!(has_gap, "expected gap numeric warning, got: {stdout}");
        assert!(has_sla, "expected sla-p1 numeric warning, got: {stdout}");
    }

    #[test]
    fn test_lint_numeric_config_valid_values_not_flagged_via_cli() {
        // Regression guard: valid numeric values across the numeric-config
        // key family must not fire the "not a number" warning.
        let spec = "## Config\n\
                    grid-size = 32\n\
                    camera_yaw = 45\n\
                    camera_pitch = -30\n\
                    gap = 80\n\
                    sla-p1 = 5\n\
                    sla-p2 = 10\n\
                    sla-p3 = 20\n\
                    sla-p4 = 40\n\
                    ## Nodes\n\
                    - [a] A\n\
                    - [b] B\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("num_ok_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("not a number")
        });
        assert!(!bad, "canonical numeric values must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_sticky_note_color_typo_via_cli() {
        // `## Notes` sticky-note tags like `{pnk}`, `{grean}`, `{yelow}`
        // used to silently default to yellow because parse_note_line had
        // a `_ => {}` catch-all. `{z:top}` (non-numeric z:) had the same
        // problem with `if let Ok(v) = ...`. Lint should now surface
        // did-you-mean hints via suggest_sticky_color for color typos and
        // the numeric_prefix_match walk for the bad `z:` value.
        let spec = "## Notes\n\
                    - Bug found here {pnk}\n\
                    - All good {grean}\n\
                    - Watch out {z:top}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("sticky_bad_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_pink = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("sticky color") && s.contains("pnk") && s.contains("pink")
        });
        let has_green = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("sticky color") && s.contains("grean") && s.contains("green")
        });
        let has_z = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("z:top") && s.contains("number")
        });
        assert!(has_pink, "expected sticky color hint for pnk→pink, got: {stdout}");
        assert!(has_green, "expected sticky color hint for grean→green, got: {stdout}");
        assert!(has_z, "expected numeric hint for {{z:top}}, got: {stdout}");
    }

    #[test]
    fn test_lint_sticky_note_valid_tags_not_flagged_via_cli() {
        // Regression guard: canonical sticky colors + valid numeric z:
        // must NOT fire any warning.
        let spec = "## Notes\n\
                    - Pink is fine {pink}\n\
                    - Green is fine {green}\n\
                    - Blue is fine {blue}\n\
                    - Purple is fine {purple}\n\
                    - Yellow is fine {yellow}\n\
                    - Layered note {z:120}\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("sticky_ok_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("sticky color") || (s.contains("Note") && s.contains("unknown"))
        });
        assert!(!bad, "canonical sticky tags must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_timeline_dir_typo_via_cli() {
        // `timeline-dir = virtical` (meant: vertical/TB) used to silently
        // default to LR through a `_ => "LR"` fallthrough. Lint should now
        // surface "Unknown timeline direction" with a did-you-mean hint
        // clamped to TB/LR (timeline doesn't support RL/BT).
        let spec = "## Config\n\
                    timeline = true\n\
                    timeline-dir = virtical\n\
                    ## Nodes\n\
                    - [a] A\n\
                    - [b] B\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("tdir_bad_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_tdir = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("timeline direction") && s.contains("virtical")
        });
        assert!(has_tdir, "expected timeline direction typo warning, got: {stdout}");
    }

    #[test]
    fn test_lint_timeline_dir_valid_values_not_flagged_via_cli() {
        // Regression guard: canonical timeline-dir values must not fire.
        for val in ["TB", "tb", "LR", "lr", "horizontal", "vertical", "top-bottom"] {
            let spec = format!(
                "## Config\n\
                 timeline = true\n\
                 timeline-dir = {}\n\
                 ## Nodes\n\
                 - [a] A\n\
                 - [b] B\n\
                 ## Flow\n\
                 a --> b\n",
                val
            );
            let uid = uuid::Uuid::new_v4();
            let tmp = std::env::temp_dir().join(format!("tdir_ok_{}.spec", uid));
            std::fs::write(&tmp, &spec).unwrap();
            let bin = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .and_then(|p| p.parent().map(|q| q.to_path_buf()))
                .map(|p| p.join("open-draftly"));
            let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
            if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
            let out = std::process::Command::new(&bin)
                .args(["lint", "--json", tmp.to_str().unwrap()])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let _ = std::fs::remove_file(&tmp);
            let v: serde_json::Value = serde_json::from_str(&stdout)
                .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
            let warnings = v["warnings"].as_array().expect("warnings array");
            let bad = warnings.iter().any(|w| {
                w.as_str().unwrap_or("").contains("timeline direction")
            });
            assert!(!bad, "`timeline-dir = {val}` should not warn, got: {stdout}");
        }
    }

    #[test]
    fn test_lint_near_duplicate_hrf_ids_flagged_via_cli() {
        // End-to-end: two HRF IDs at Levenshtein distance 1 with min length
        // >= 5 should surface a typo-variant warning. `customer` vs `custumer`
        // is the canonical case: the parser silently creates two separate
        // nodes so any `## Flow` edge referencing only one of them looks
        // valid but the other hangs off unnoticed.
        let spec = "## Nodes\n\
                    - [customer] Customer\n\
                    - [custumer] Custumer\n\
                    ## Flow\n\
                    customer --> custumer\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("near_dup_ids_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_typo = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("HRF IDs")
                && s.contains("customer")
                && s.contains("custumer")
                && s.contains("typo")
        });
        assert!(has_typo, "expected near-duplicate HRF ID warning, got: {stdout}");
    }

    #[test]
    fn test_lint_short_hrf_ids_not_flagged_via_cli() {
        // IDs shorter than 5 chars are too ambiguous (too many legitimate
        // near-match pairs like `api`/`ipa`), so the lint must ignore them.
        // `user`/`uesr` differ by a swap but sit below the min-length guard.
        let spec = "## Nodes\n\
                    - [user] User\n\
                    - [uesr] Uesr\n\
                    ## Flow\n\
                    user --> uesr\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("short_dup_ids_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_typo = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("HRF IDs") && s.contains("typo")
        });
        assert!(!has_typo, "short IDs must not flag near-duplicate, got: {stdout}");
    }

    #[test]
    fn test_lint_distant_hrf_ids_not_flagged_via_cli() {
        // IDs that are far apart (`customer` vs `product`, d >= 5) are
        // legitimately distinct domain concepts, not typo variants — the
        // lint must stay silent.
        let spec = "## Nodes\n\
                    - [customer] Customer\n\
                    - [product] Product\n\
                    ## Flow\n\
                    customer --> product\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("distant_ids_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_typo = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("HRF IDs") && s.contains("typo")
        });
        assert!(!has_typo, "distant IDs must not flag near-duplicate, got: {stdout}");
    }

    #[test]
    fn test_lint_exact_duplicate_hrf_ids_still_errors_via_cli() {
        // Exact duplicate IDs are rejected by the parser before lint runs.
        // In JSON mode, cli_lint now catches the parse failure and emits
        // a structured JSON payload with the error in `errors[]` rather
        // than leaving stdout empty. This guards the contract that
        // `lint --json` ALWAYS produces valid JSON for CI/IDE consumption.
        let spec = "## Nodes\n\
                    - [orders] Orders v1\n\
                    - [orders] Orders v2\n\
                    ## Flow\n\
                    orders --> orders\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("exact_dup_ids_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        assert!(
            !out.status.success(),
            "exact duplicate must cause non-zero exit. stdout={stdout}"
        );
        // stdout must parse as JSON and carry the duplicate-ID error.
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let errors = v["errors"].as_array().expect("errors array");
        let has_dup_err = errors.iter().any(|e| {
            let s = e.as_str().unwrap_or("").to_ascii_lowercase();
            s.contains("orders") && s.contains("duplicate")
        });
        assert!(has_dup_err, "expected duplicate-ID error in JSON payload, got: {stdout}");
        // Near-duplicate warning MUST NOT fire — parser rejected input first.
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_typo = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("HRF IDs") && s.contains("typo")
        });
        assert!(!has_typo, "exact dup must not fire near-duplicate warning, got: {stdout}");
    }

    #[test]
    fn test_lint_empty_frame_flagged() {
        // A frame with no spatially-contained nodes is a leftover. Build the
        // doc programmatically so we have full control over positions: one
        // shape node at the origin and one frame 500 units away with no
        // members. Then mirror the lint's bbox-containment walk directly.
        let mut doc = crate::model::FlowchartDocument::default();
        doc.nodes.push(crate::model::Node::new(
            crate::model::NodeShape::Rectangle,
            egui::Pos2::new(0.0, 0.0),
        ));
        let mut frame = crate::model::Node::new_frame(egui::Pos2::new(500.0, 500.0));
        frame.size = [100.0, 100.0];
        doc.nodes.push(frame);

        assert_eq!(doc.nodes.iter().filter(|n| n.is_frame).count(), 1);
        assert_eq!(doc.nodes.iter().filter(|n| !n.is_frame).count(), 1);

        // Walk the same spatial-containment check the lint uses.
        let mut empty_frames = 0;
        for frame in doc.nodes.iter().filter(|n| n.is_frame) {
            let fx0 = frame.position[0];
            let fy0 = frame.position[1];
            let fx1 = fx0 + frame.size[0];
            let fy1 = fy0 + frame.size[1];
            let inside = doc.nodes.iter().filter(|n| {
                if n.is_frame || n.id == frame.id { return false; }
                let cx = n.position[0] + n.size[0] * 0.5;
                let cy = n.position[1] + n.size[1] * 0.5;
                cx >= fx0 && cx <= fx1 && cy >= fy0 && cy <= fy1
            }).count();
            if inside == 0 { empty_frames += 1; }
        }
        assert_eq!(empty_frames, 1, "expected exactly one empty frame");
    }

    #[test]
    fn test_group_orphan_member_captured() {
        // Parser preserves unresolved `## Groups` member IDs in
        // `import_hints.unresolved_group_members` so lint can flag them.
        // Without this, the silently-skipped member would ship in users'
        // specs forever.
        let spec = "## Nodes\n- [api] API\n- [db] Database\n\n## Groups\n- [backend] Backend\n  api, ghost_node\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert_eq!(
            doc.import_hints.unresolved_group_members,
            vec![("backend".to_string(), "ghost_node".to_string())],
            "expected exactly one unresolved group member"
        );
    }

    #[test]
    fn test_group_orphan_member_suggests_closest_id() {
        // Typo case: `api_svc` vs real id `api-svc` — suggestor must find
        // the 1-edit alternative across the punctuation swap.
        use crate::specgraph::hrf::suggest_node_id_from_candidates;
        let candidates = ["api-svc", "db", "cache"];
        assert_eq!(
            suggest_node_id_from_candidates("api_svc", &candidates),
            Some("api-svc".to_string())
        );
    }

    #[test]
    fn test_group_orphan_member_no_suggestion_when_far() {
        // Safety: if nothing is close, we must NOT invent a bogus match —
        // false-positive "did you mean" hints are worse than silence.
        use crate::specgraph::hrf::suggest_node_id_from_candidates;
        let candidates = ["alpha", "beta", "gamma"];
        assert_eq!(
            suggest_node_id_from_candidates("totally_unrelated", &candidates),
            None
        );
    }

    #[test]
    fn test_group_orphan_member_ignores_exact_match() {
        // An id that literally exists must return None so the lint check
        // never emits "did you mean X?" for a match that's already correct.
        use crate::specgraph::hrf::suggest_node_id_from_candidates;
        let candidates = ["api", "db"];
        assert_eq!(
            suggest_node_id_from_candidates("api", &candidates),
            None
        );
    }

    #[test]
    fn test_group_orphan_lint_json_output_contains_warning() {
        // End-to-end: `lint --json` must surface orphaned group members as
        // a warning so downstream tooling (CI, IDE) can act on them.
        use std::process::Command;
        let spec = "## Nodes\n- [api] API\n- [db] Database\n\n## Groups\n- [backend] Backend\n  api, ghost_node\n\n## Flow\napi --> db\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("group_orphan_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();

        let exe = std::env::current_exe().unwrap();
        let bin = exe.parent().unwrap().parent().unwrap().join("open-draftly");
        if !bin.exists() {
            eprintln!("skipping test_group_orphan_lint_json_output_contains_warning: release binary not found");
            let _ = std::fs::remove_file(&tmp);
            return;
        }
        let out = Command::new(&bin).arg("lint").arg(&tmp).arg("--json").output().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .expect("lint --json should be valid JSON");
        let warnings = parsed.get("warnings").unwrap().as_array().unwrap();
        assert!(
            warnings.iter().any(|w| {
                let s = w.as_str().unwrap_or("");
                s.contains("ghost_node") && s.contains("backend")
            }),
            "expected orphaned-group-member warning, got: {warnings:?}"
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_config_typo_suggests_title() {
        use crate::specgraph::hrf::suggest_config_key;
        assert_eq!(suggest_config_key("tilte"), Some("title"));
        assert_eq!(suggest_config_key("titel"), Some("title"));
    }

    #[test]
    fn test_config_typo_suggests_flow_and_zoom() {
        use crate::specgraph::hrf::suggest_config_key;
        assert_eq!(suggest_config_key("flwo"), Some("flow"));
        assert_eq!(suggest_config_key("zooom"), Some("zoom"));
    }

    #[test]
    fn test_config_typo_suggests_timeline() {
        use crate::specgraph::hrf::suggest_config_key;
        assert_eq!(suggest_config_key("timline"), Some("timeline"));
    }

    #[test]
    fn test_config_typo_ignores_exact_matches() {
        use crate::specgraph::hrf::suggest_config_key;
        for known in ["title", "flow", "zoom", "bg", "timeline", "camera", "sla-p1", "gap-main"] {
            assert_eq!(
                suggest_config_key(known),
                None,
                "exact-match config key '{known}' should not get a suggestion"
            );
        }
    }

    #[test]
    fn test_config_typo_ignores_layer_keys() {
        use crate::specgraph::hrf::suggest_config_key;
        // layerN keys are handled by a dedicated arm before reaching the
        // fallthrough — never flag them as typos.
        assert_eq!(suggest_config_key("layer0"), None);
        assert_eq!(suggest_config_key("layer12"), None);
    }

    #[test]
    fn test_config_typo_ignores_unrelated() {
        use crate::specgraph::hrf::suggest_config_key;
        assert_eq!(suggest_config_key("totally-unrelated-thing"), None);
        assert_eq!(suggest_config_key("xyzzy"), None);
    }

    #[test]
    fn test_config_unknown_key_captured_on_doc() {
        // End-to-end: an unknown config key lands in unknown_config_keys so
        // cli_lint can surface it as a warning.
        let spec = "## Config\ntilte = My Flowchart\nflwo = LR\ntitle = Real Title\n\n## Nodes\n- [a] A\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        // Known keys applied normally
        assert_eq!(doc.title, "Real Title");
        // flwo=LR is a typo — layout_dir must NOT have been set to "LR".
        // (The real `flow` key was not present, so layout_dir stays at its
        // struct default empty string.)
        assert_ne!(doc.layout_dir, "LR", "flwo typo must not affect layout_dir");
        // Both typos captured
        assert!(doc.import_hints.unknown_config_keys.contains(&"tilte".to_string()));
        assert!(doc.import_hints.unknown_config_keys.contains(&"flwo".to_string()));
        assert!(!doc.import_hints.unknown_config_keys.contains(&"title".to_string()));
    }

    #[test]
    fn test_orphan_group_member_captured_with_suggestion() {
        // End-to-end: a `## Groups` section that references a non-existent
        // member id must surface in unresolved_group_members, and
        // suggest_node_id_from_candidates must resolve a typo back to the
        // real id. Previously the parser silently dropped bad members.
        let spec = "\
## Nodes
- [api] API
- [db] Database
- [ui] Frontend

## Groups
- [backend] Backend
  api, database, ui
";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        // `api` and `ui` are valid. `database` is NOT — the real id is `db`.
        let orphans = &doc.import_hints.unresolved_group_members;
        assert_eq!(orphans.len(), 1,
            "exactly one orphan expected, got {:?}", orphans);
        assert_eq!(orphans[0].0, "backend", "group_id should be the containing group");
        assert_eq!(orphans[0].1, "database", "member_id should be the bad reference");
        // Suggestion check: `database` should resolve to `db` or return None if
        // the edit distance is too wide. With the length-scaled tolerance
        // (len 8 → max_d 3), the match is still far — so None is acceptable.
        // What MUST hold: `api` is a closer candidate than a random string.
        let known_ids: Vec<&str> = doc.nodes.iter()
            .filter(|n| !n.hrf_id.is_empty())
            .map(|n| n.hrf_id.as_str())
            .collect();
        // Exact-match ids are never flagged
        assert_eq!(
            crate::specgraph::hrf::suggest_node_id_from_candidates("api", &known_ids),
            None
        );
    }

    #[test]
    fn test_orphan_group_member_suggests_near_typo() {
        // A 1-edit typo in a member id must resolve to the real id.
        let spec = "\
## Nodes
- [api] API
- [db] Database
- [ui] Frontend

## Groups
- [backend] Backend
  apii, db
";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let orphans = &doc.import_hints.unresolved_group_members;
        // `apii` (len 4) with 1 edit from `api` — must be in orphans.
        assert!(orphans.iter().any(|(gid, mid)| gid == "backend" && mid == "apii"),
            "expected orphan (backend, apii), got {:?}", orphans);
        let known_ids: Vec<&str> = doc.nodes.iter()
            .filter(|n| !n.hrf_id.is_empty())
            .map(|n| n.hrf_id.as_str())
            .collect();
        // `apii` → `api` (distance 1, within max_d=2 for len 4).
        assert_eq!(
            crate::specgraph::hrf::suggest_node_id_from_candidates("apii", &known_ids),
            Some("api".to_string())
        );
    }

    #[test]
    fn test_orphan_group_member_clean_spec_has_none() {
        // Regression: a fully-resolved Groups section must leave
        // unresolved_group_members empty — no false positives.
        let spec = "\
## Nodes
- [api] API
- [db] Database
- [ui] Frontend

## Groups
- [backend] Backend
  api, db
- [frontend] Frontend
  ui
";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert!(doc.import_hints.unresolved_group_members.is_empty(),
            "clean spec should have no unresolved members, got {:?}",
            doc.import_hints.unresolved_group_members);
    }

    #[test]
    fn test_suggest_node_id_from_candidates_basic() {
        use crate::specgraph::hrf::suggest_node_id_from_candidates;
        let candidates = ["api", "db", "ui", "backend", "frontend"];
        // 1-edit typo
        assert_eq!(
            suggest_node_id_from_candidates("apii", &candidates),
            Some("api".to_string())
        );
        // 2-edit typo in a longer id
        assert_eq!(
            suggest_node_id_from_candidates("bakend", &candidates),
            Some("backend".to_string())
        );
        // Exact match returns None (not a typo)
        assert_eq!(suggest_node_id_from_candidates("api", &candidates), None);
        // Far off — no suggestion
        assert_eq!(
            suggest_node_id_from_candidates("totallyrandom", &candidates),
            None
        );
    }

    #[test]
    fn test_suggest_node_id_from_candidates_empty_cases() {
        use crate::specgraph::hrf::suggest_node_id_from_candidates;
        // Empty candidate list
        assert_eq!(suggest_node_id_from_candidates("anything", &[]), None);
        // Empty bad id
        let candidates = ["api", "db"];
        assert_eq!(suggest_node_id_from_candidates("", &candidates), None);
    }

    #[test]
    fn test_suggest_node_id_from_candidates_case_insensitive() {
        use crate::specgraph::hrf::suggest_node_id_from_candidates;
        let candidates = ["API", "DB", "UI"];
        // Exact match ignoring case returns None
        assert_eq!(suggest_node_id_from_candidates("api", &candidates), None);
        assert_eq!(suggest_node_id_from_candidates("Api", &candidates), None);
    }

    #[test]
    fn test_lint_lane_typo_split_flagged_via_cli() {
        // `{lane:Sales}` × 3 and `{lane:Slaes}` × 1 should trip the
        // typo-split warning with the new lane-aware lint.
        let spec = "## Nodes\n\
                    - [a] Alpha {lane:Sales}\n\
                    - [b] Beta {lane:Sales}\n\
                    - [c] Gamma {lane:Sales}\n\
                    - [d] Delta {lane:Slaes}\n\
                    ## Flow\na --> b\nb --> c\nc --> d\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("lane_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_lane = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Inline lane")
                && s.contains("Slaes")
                && s.contains("Sales")
                && s.contains("typo")
        });
        assert!(has_lane, "expected lane typo-split warning, got: {stdout}");
    }

    #[test]
    fn test_lint_phase_typo_split_flagged_via_cli() {
        // `{phase:Quarter1}` × 3 and `{phase:Qaurter1}` × 1 should trip.
        // "Q1" / "Q01" are < 4 chars so they're suppressed — need a
        // longer name to exercise the path.
        let spec = "## Nodes\n\
                    - [a] Alpha {phase:Quarter1}\n\
                    - [b] Beta {phase:Quarter1}\n\
                    - [c] Gamma {phase:Quarter1}\n\
                    - [d] Delta {phase:Qaurter1}\n\
                    ## Flow\na --> b\nb --> c\nc --> d\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("phase_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_phase = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("Inline phase")
                && s.contains("Qaurter1")
                && s.contains("Quarter1")
                && s.contains("typo")
        });
        assert!(has_phase, "expected phase typo-split warning, got: {stdout}");
    }

    #[test]
    fn test_lint_distinct_lanes_not_flagged_via_cli() {
        // `Backend` vs `Frontend` are far apart — must NOT fire the
        // typo-split warning.
        let spec = "## Nodes\n\
                    - [a] API {lane:Backend}\n\
                    - [b] DB {lane:Backend}\n\
                    - [c] UI {lane:Frontend}\n\
                    - [d] CDN {lane:Frontend}\n\
                    ## Flow\na --> b\nc --> d\na --> c\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("lane_distinct_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_lane = warnings.iter().any(|w| {
            w.as_str().unwrap_or("").contains("Inline lane")
        });
        assert!(!has_lane, "distinct lanes must not fire, got: {stdout}");
    }

    #[test]
    fn test_lint_lane_both_majorities_not_flagged() {
        // Rule: if both candidates have count ≥ 2, don't flag — they're
        // probably intentional separate lanes. `Sales` × 2 and `Slaes` × 2
        // should NOT trip (neither is a minority).
        let spec = "## Nodes\n\
                    - [a] A {lane:Sales}\n\
                    - [b] B {lane:Sales}\n\
                    - [c] C {lane:Slaes}\n\
                    - [d] D {lane:Slaes}\n\
                    ## Flow\na --> b\nc --> d\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        // Build the same aggregation the lint does and assert that no
        // (a,b) pair where ac>=2 and bc>=2 ever fires.
        let mut counts_map: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for node in &doc.nodes {
            if node.is_frame { continue; }
            if let Some(l) = &node.timeline_lane {
                *counts_map.entry(l.clone()).or_insert(0) += 1;
            }
        }
        let pairs_both_majority: usize = counts_map
            .values()
            .filter(|&&c| c >= 2)
            .count();
        assert!(
            pairs_both_majority >= 2,
            "expected both lanes to have count ≥ 2, got {counts_map:?}"
        );
    }

    #[test]
    fn test_inline_group_names_captured_with_counts() {
        // Parser must record every inline `{group:X}` name along with how
        // many nodes carried it, so the lint pass can compare them pairwise.
        let spec = "\
## Nodes
- [db] Database {group:backend}
- [api] API {group:backend}
- [cache] Cache {group:backend}
- [ui] Frontend {group:frontend}
- [worker] Worker {group:bakcend}
";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let counts = &doc.import_hints.inline_group_name_counts;
        // Three distinct names: backend (×3), frontend (×1), bakcend (×1)
        assert_eq!(counts.len(), 3, "expected 3 group names, got {:?}", counts);
        let backend = counts.iter().find(|(n, _)| n == "backend");
        assert_eq!(backend.map(|(_, c)| *c), Some(3), "backend should have 3 members");
        let bakcend = counts.iter().find(|(n, _)| n == "bakcend");
        assert_eq!(bakcend.map(|(_, c)| *c), Some(1), "bakcend should have 1 member");
    }

    #[test]
    fn test_inline_group_different_via_cluster_and_in_aliases() {
        // `cluster:` and `in:` are aliases for `group:` and must all flow
        // into the same count map.
        let spec = "\
## Nodes
- [db] Database {cluster:data}
- [cache] Cache {in:data}
- [ui] Frontend {group:web}
";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let counts = &doc.import_hints.inline_group_name_counts;
        let data = counts.iter().find(|(n, _)| n == "data");
        assert_eq!(data.map(|(_, c)| *c), Some(2),
            "cluster: and in: should merge into the same `data` group, got {:?}", counts);
    }

    #[test]
    fn test_inline_group_distinct_names_not_captured_as_typo_split() {
        // Regression: `backend` vs `frontend` are far apart and must NEVER
        // both appear as one name looking like a typo of the other. The
        // cli_lint walk does the comparison, but we can sanity-check by
        // computing distance directly.
        let spec = "\
## Nodes
- [db] Database {group:backend}
- [api] API {group:backend}
- [ui] Frontend {group:frontend}
- [cdn] CDN {group:frontend}
";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let counts = &doc.import_hints.inline_group_name_counts;
        assert_eq!(counts.len(), 2);
        // Both groups have 2 members — the lint walk skips these entirely
        // (both_count >= 2 == intentional distinct groups).
        let backend_count = counts.iter().find(|(n, _)| n == "backend").unwrap().1;
        let frontend_count = counts.iter().find(|(n, _)| n == "frontend").unwrap().1;
        assert_eq!(backend_count, 2);
        assert_eq!(frontend_count, 2);
        // Distance is large — even if one side had 1 member, this wouldn't
        // trigger (distance > 2).
        assert!(super::levenshtein_distance("backend", "frontend") > 2);
    }

    #[test]
    fn test_levenshtein_distance_basic() {
        use super::levenshtein_distance;
        assert_eq!(levenshtein_distance("backend", "bakcend"), 2);
        assert_eq!(levenshtein_distance("backend", "backend"), 0);
        assert_eq!(levenshtein_distance("api", "apii"), 1);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_duplicate_edge_lint_flags_exact_duplicate() {
        // Two identical edges (same src, tgt, empty label) should produce a
        // single warning via `lint --json` — silent failure mode because they
        // draw on top of each other.
        let dir = std::env::temp_dir();
        let tmp = dir.join(format!("dup_edge_exact_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, "## Nodes\n- [a] Alpha\n- [b] Bravo\n## Flow\na --> b\na --> b\n").unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json output not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_dup = warnings
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("Duplicate edge"));
        assert!(has_dup, "expected Duplicate edge warning, got: {}", stdout);
    }

    #[test]
    fn test_duplicate_edge_lint_distinguishes_by_label() {
        // Same src+tgt but different labels should NOT be flagged as duplicates.
        let dir = std::env::temp_dir();
        let tmp = dir.join(format!("dup_edge_diff_label_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(
            &tmp,
            "## Nodes\n- [a] Alpha\n- [b] Bravo\n## Flow\na --> b: yes\na --> b: no\n",
        )
        .unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_dup = warnings
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("Duplicate edge"));
        assert!(
            !has_dup,
            "different labels must not be flagged as duplicates, got: {}",
            stdout
        );
    }

    #[test]
    fn test_empty_frame_lint_flags_frame_with_no_contents() {
        // A frame with no non-frame nodes inside its bounding box should be
        // flagged. Use pinned positions so auto-layout doesn't move things.
        let dir = std::env::temp_dir();
        let tmp = dir.join(format!("empty_frame_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(
            &tmp,
            "## Nodes\n- [area] Empty Area {frame} {x:100} {y:100} {w:300} {h:200} {pinned}\n\
             - [outside] Outside Node {x:800} {y:800} {pinned}\n",
        )
        .unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_empty = warnings
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("Empty Area") && w.as_str().unwrap_or("").contains("empty"));
        assert!(has_empty, "expected empty-frame warning, got: {}", stdout);
    }

    #[test]
    fn test_empty_frame_lint_allows_populated_frame() {
        // A frame with a node inside must NOT trip the empty-frame lint.
        let dir = std::env::temp_dir();
        let tmp = dir.join(format!("populated_frame_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(
            &tmp,
            "## Nodes\n- [area] Good Area {frame} {x:50} {y:50} {w:400} {h:300} {pinned}\n\
             - [inside] Inside Node {x:150} {y:150} {pinned}\n",
        )
        .unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let any_empty = warnings
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("Good Area") && w.as_str().unwrap_or("").contains("empty"));
        assert!(
            !any_empty,
            "populated frame must not be flagged as empty, got: {}",
            stdout
        );
    }

    #[test]
    fn test_parse_error_line_extracts_leading_line_number() {
        use super::parse_error_line;
        assert_eq!(parse_error_line("Line 2: missing closing ]"), Some(2));
        assert_eq!(parse_error_line("Line 42: some error"), Some(42));
        // Non-line-anchored errors return None.
        assert_eq!(parse_error_line("random error text"), None);
        assert_eq!(parse_error_line("Line X: bad number"), None);
        assert_eq!(parse_error_line(""), None);
    }

    #[test]
    fn test_validate_json_valid_file_emits_valid_true() {
        // valid spec should emit {valid: true, error: null, line: null, ...}
        let dir = std::env::temp_dir();
        let tmp = dir.join(format!("validate_json_valid_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, "## Nodes\n- [a] Alpha\n- [b] Bravo\n## Flow\na --> b\n").unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["validate", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        assert!(out.status.success(), "valid file should exit 0, got: {}", stdout);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("validate --json not valid JSON: {}", stdout));
        assert_eq!(v["valid"], serde_json::Value::Bool(true));
        assert_eq!(v["error"], serde_json::Value::Null);
        assert_eq!(v["line"], serde_json::Value::Null);
        assert_eq!(v["node_count"], serde_json::Value::from(2));
        assert_eq!(v["edge_count"], serde_json::Value::from(1));
    }

    #[test]
    fn test_validate_json_invalid_file_emits_line_and_error() {
        // invalid spec should emit {valid: false, error: <clean>, line: N, ...}
        // and exit with a non-zero status so CI pipelines can detect failure.
        let dir = std::env::temp_dir();
        let tmp = dir.join(format!("validate_json_invalid_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, "## Nodes\n- [bad Label without closing bracket\n").unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["validate", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        assert!(!out.status.success(), "invalid file should exit non-zero");
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("validate --json not valid JSON: {}", stdout));
        assert_eq!(v["valid"], serde_json::Value::Bool(false));
        // Line prefix should be stripped from error and surfaced as a number.
        assert_eq!(v["line"], serde_json::Value::from(2));
        let err_str = v["error"].as_str().expect("error is string");
        assert!(
            !err_str.starts_with("Line "),
            "error should have Line prefix stripped, got: {:?}",
            err_str
        );
        assert!(err_str.contains("closing ]"), "error should describe cause, got: {:?}", err_str);
    }

    #[test]
    fn test_validate_json_missing_file_emits_read_error() {
        // Missing files should surface through JSON rather than panic or stderr.
        let bogus = std::env::temp_dir().join(format!("nope_{}.spec", uuid::Uuid::new_v4()));
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { return; }
        let out = std::process::Command::new(&bin)
            .args(["validate", "--json", bogus.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        assert!(!out.status.success(), "missing file should exit non-zero");
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("validate --json not valid JSON: {}", stdout));
        assert_eq!(v["valid"], serde_json::Value::Bool(false));
        let err = v["error"].as_str().unwrap_or("");
        assert!(err.contains("read error"), "expected read error, got: {:?}", err);
    }

    #[test]
    fn test_templates_list_json_is_valid_array() {
        // `templates list --json` should be a stable JSON array of
        // {name, category, description} so scripts and IDE integrations
        // can enumerate templates without text-parsing.
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { return; }
        let out = std::process::Command::new(&bin)
            .args(["templates", "list", "--json"])
            .output()
            .unwrap();
        assert!(out.status.success(), "templates list --json should succeed");
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("templates list --json not valid JSON: {}", stdout));
        let arr = v.as_array().expect("top level is an array");
        assert!(!arr.is_empty(), "template list must be non-empty");
        for item in arr {
            assert!(item.get("name").and_then(|x| x.as_str()).is_some());
            assert!(item.get("category").and_then(|x| x.as_str()).is_some());
            assert!(item.get("description").and_then(|x| x.as_str()).is_some());
        }
        // At least one known template should be present.
        let names: Vec<&str> = arr
            .iter()
            .filter_map(|i| i.get("name").and_then(|x| x.as_str()))
            .collect();
        assert!(names.contains(&"Architecture"), "Architecture template missing: {:?}", names);
    }

    #[test]
    fn test_templates_list_json_sorted_by_category_then_name() {
        // Stable ordering lets CI compare snapshots across runs.
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { return; }
        let out = std::process::Command::new(&bin)
            .args(["templates", "list", "--json"])
            .output()
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .expect("valid JSON");
        let arr = v.as_array().unwrap();
        let keys: Vec<(String, String)> = arr
            .iter()
            .map(|i| (
                i["category"].as_str().unwrap().to_string(),
                i["name"].as_str().unwrap().to_string(),
            ))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "templates list --json must be sorted by (category, name)");
    }

    #[test]
    fn test_templates_get_json_returns_metadata_and_content() {
        // `templates get <name> --json` should inline the HRF content along
        // with metadata, so an IDE can show a preview without a second call.
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { return; }
        let out = std::process::Command::new(&bin)
            .args(["templates", "get", "Architecture", "--json"])
            .output()
            .unwrap();
        assert!(out.status.success());
        let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .expect("valid JSON");
        assert_eq!(v["name"].as_str(), Some("Architecture"));
        assert_eq!(v["category"].as_str(), Some("Engineering"));
        let content = v["content"].as_str().expect("content is string");
        assert!(content.contains("## Nodes"), "content must include ## Nodes section");
        assert_eq!(v["written_to"], serde_json::Value::Null);
    }

    #[test]
    fn test_templates_get_json_not_found_lists_available() {
        // A bad template name must exit non-zero but still emit valid JSON
        // containing the set of available names so the user can recover.
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { return; }
        let out = std::process::Command::new(&bin)
            .args(["templates", "get", "TotallyBogus", "--json"])
            .output()
            .unwrap();
        assert!(!out.status.success(), "missing template must exit non-zero");
        let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .expect("valid JSON");
        let err = v["error"].as_str().expect("error is string");
        assert!(err.contains("not found"), "expected 'not found' error, got {:?}", err);
        let available = v["available"].as_array().expect("available is array");
        assert!(!available.is_empty(), "available list must be non-empty");
    }

    #[test]
    fn test_templates_get_json_with_out_writes_file() {
        // Combining --out and --json should both write the HRF to the file
        // AND emit metadata to stdout, with `written_to` set.
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { return; }
        let tmp = std::env::temp_dir().join(format!("tmpl_out_{}.spec", uuid::Uuid::new_v4()));
        let out = std::process::Command::new(&bin)
            .args([
                "templates", "get", "Architecture", "--json",
                "--out", tmp.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(out.status.success());
        let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .expect("valid JSON");
        assert_eq!(v["written_to"].as_str(), Some(tmp.display().to_string().as_str()));
        let file_content = std::fs::read_to_string(&tmp).expect("file should exist");
        let json_content = v["content"].as_str().unwrap();
        assert_eq!(file_content, json_content, "file must match JSON content");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_config_layer_keys_not_captured_as_unknown() {
        // `layer0 = Data Tier` is handled by the `_ if key.starts_with("layer")`
        // arm, so it must not leak into unknown_config_keys.
        let spec = "## Config\nlayer0 = Data Tier\nlayer 1 = Backend\n\n## Nodes\n- [a] A\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert!(doc.import_hints.unknown_config_keys.is_empty(),
            "layer keys must not appear in unknown_config_keys, got {:?}",
            doc.import_hints.unknown_config_keys);
    }

    #[test]
    fn test_unused_style_definition_captured_in_usage() {
        // A style defined but never referenced should land in
        // style_definition_usage with count 0.
        let spec = "## Style\nprimary = {fill:#0000ff}\nunused = {fill:#00ff00}\n\n\
                    ## Nodes\n- [a] A {primary}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let usage = &doc.import_hints.style_definition_usage;
        assert_eq!(usage.len(), 2, "should track both style definitions");
        let primary_count = usage
            .iter()
            .find(|(n, _)| n == "primary")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let unused_count = usage
            .iter()
            .find(|(n, _)| n == "unused")
            .map(|(_, c)| *c)
            .unwrap_or(usize::MAX);
        assert_eq!(primary_count, 1, "primary referenced once");
        assert_eq!(unused_count, 0, "unused never referenced");
    }

    #[test]
    fn test_style_used_multiple_times_counted_correctly() {
        // A style referenced on 3 different nodes should land with count 3.
        let spec = "## Style\nblue = {fill:#0000ff}\n\n## Nodes\n\
                    - [a] A {blue}\n- [b] B {blue}\n- [c] C {blue}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let count = doc
            .import_hints
            .style_definition_usage
            .iter()
            .find(|(n, _)| n == "blue")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        assert_eq!(count, 3, "blue referenced 3 times");
    }

    #[test]
    fn test_style_usage_empty_when_no_style_section() {
        // Documents without `## Style` should produce an empty usage vec.
        let spec = "## Nodes\n- [a] A\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert!(
            doc.import_hints.style_definition_usage.is_empty(),
            "no style section should produce empty usage, got {:?}",
            doc.import_hints.style_definition_usage
        );
    }

    #[test]
    fn test_unused_style_lint_flag_via_cli() {
        // End-to-end: a spec with an unused style should produce a lint
        // warning that mentions the style name and "dead code".
        let dir = std::env::temp_dir();
        let tmp = dir.join(format!("unused_style_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(
            &tmp,
            "## Style\nghost = {fill:#888888}\n\n## Nodes\n- [a] A\n",
        )
        .unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_dead = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("ghost") && s.contains("dead code")
        });
        assert!(has_dead, "expected dead-code warning for ghost, got: {}", stdout);
    }

    #[test]
    fn test_unused_palette_color_captured_in_usage() {
        // A palette entry referenced by nodes should have nonzero count, an
        // unreferenced one should have count 0.
        let spec = "## Palette\nprimary = #0000ff\nghost = #888888\n\n\
                    ## Nodes\n- [a] A {fill:primary}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let usage = &doc.import_hints.palette_definition_usage;
        assert_eq!(usage.len(), 2, "should track both palette entries");
        let primary = usage.iter().find(|(n, _)| n == "primary").map(|(_, c)| *c).unwrap_or(0);
        let ghost = usage.iter().find(|(n, _)| n == "ghost").map(|(_, c)| *c).unwrap_or(usize::MAX);
        assert_eq!(primary, 1, "primary referenced via {{fill:primary}}");
        assert_eq!(ghost, 0, "ghost never referenced");
    }

    #[test]
    fn test_palette_counted_across_multiple_tag_types() {
        // One palette name referenced via both {fill:} and {stroke:} should
        // count as 2 usages — not be flagged as dead code.
        let spec = "## Palette\naccent = #ff8800\n\n## Nodes\n\
                    - [a] A {fill:accent}\n- [b] B {stroke:accent}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        let count = doc
            .import_hints
            .palette_definition_usage
            .iter()
            .find(|(n, _)| n == "accent")
            .map(|(_, c)| *c)
            .unwrap_or(0);
        assert_eq!(count, 2, "accent referenced in fill and stroke");
    }

    #[test]
    fn test_palette_usage_empty_when_no_palette_section() {
        // Documents without `## Palette` should produce an empty usage vec.
        let spec = "## Nodes\n- [a] A {fill:#ffffff}\n";
        let doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
        assert!(
            doc.import_hints.palette_definition_usage.is_empty(),
            "no palette section should produce empty usage, got {:?}",
            doc.import_hints.palette_definition_usage
        );
    }

    #[test]
    fn test_unused_palette_lint_flag_via_cli() {
        // End-to-end: a spec with an unreferenced palette entry should
        // produce a lint warning mentioning the color name and "dead code".
        let dir = std::env::temp_dir();
        let tmp = dir.join(format!("unused_palette_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(
            &tmp,
            "## Palette\nspectre = #cccccc\n\n## Nodes\n- [a] A\n",
        )
        .unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {}", stdout));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let has_dead = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("spectre") && s.contains("Palette") && s.contains("dead code")
        });
        assert!(has_dead, "expected dead-code warning for spectre, got: {}", stdout);
    }

    #[test]
    fn test_diff_detects_edge_label_changes() {
        // Same endpoints, different label → should appear as a MODIFIED edge
        // rather than a paired add/remove pair.
        let before = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b: old label\n";
        let after  = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b: new label\n";
        let doc_a = crate::specgraph::hrf::parse_hrf(before).unwrap();
        let doc_b = crate::specgraph::hrf::parse_hrf(after).unwrap();
        assert_eq!(doc_a.edges.len(), 1);
        assert_eq!(doc_b.edges.len(), 1);
        assert_ne!(doc_a.edges[0].label, doc_b.edges[0].label);
        assert_eq!(doc_a.edges[0].label, "old label");
        assert_eq!(doc_b.edges[0].label, "new label");
    }

    #[test]
    fn test_diff_detects_edge_style_changes() {
        // Same endpoints + label, but style flipped dashed → solid.
        let before = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b {dashed}\n";
        let after  = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b {thick}\n";
        let doc_a = crate::specgraph::hrf::parse_hrf(before).unwrap();
        let doc_b = crate::specgraph::hrf::parse_hrf(after).unwrap();
        assert!(doc_a.edges[0].style.dashed);
        assert!(!doc_b.edges[0].style.dashed);
        assert!(doc_b.edges[0].style.width > 4.0);
    }

    #[test]
    fn test_lint_json_output_is_valid_json() {
        // Use the CLI binary to render JSON output and verify it parses +
        // has the expected top-level keys.
        use std::process::Command;
        let spec = "## Nodes\n- [a] A {daimond}\n- [b] B\n\n## Flow\na --> b\n";
        let tmp = std::env::temp_dir().join(format!("lint_json_{}.spec", uuid::Uuid::new_v4()));
        std::fs::write(&tmp, spec).unwrap();

        let exe = std::env::current_exe().unwrap();
        // Walk up from .../deps/main-<hash> to the target/release binary
        let target_dir = exe.parent().unwrap().parent().unwrap();
        let bin = target_dir.join("open-draftly");
        if !bin.exists() {
            // Debug build won't have release binary — skip quietly
            eprintln!("skipping test_lint_json_output_is_valid_json: release binary not found at {:?}", bin);
            return;
        }
        let out = Command::new(&bin)
            .arg("lint")
            .arg(&tmp)
            .arg("--json")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|e| panic!("lint --json did not emit valid JSON: {}\n---\n{}", e, stdout));
        assert!(parsed.get("errors").is_some(), "missing errors key");
        assert!(parsed.get("warnings").is_some(), "missing warnings key");
        assert!(parsed.get("error_count").is_some());
        assert!(parsed.get("warning_count").is_some());
        assert!(parsed.get("clean").is_some());
        // daimond is a shape typo — must show up as a warning
        let warnings = parsed.get("warnings").unwrap().as_array().unwrap();
        assert!(warnings.iter().any(|w| w.as_str().unwrap_or("").contains("daimond")));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_diff_json_output_is_valid_json() {
        // End-to-end: run the diff command with --json and verify the full
        // shape of the output. This covers adds, removes, modified nodes,
        // modified edges (label change), and the summary block all in one
        // pass so regressions in any branch fail loudly.
        use std::process::Command;
        let before = "## Nodes\n- [a] Start\n- [b] Middle\n- [c] End {diamond}\n\n## Flow\na --> b: original\nb --> c\n";
        let after  = "## Nodes\n- [a] Start\n- [b] Middle\n- [c] End {circle}\n- [d] NewNode\n\n## Flow\na --> b: updated label\nb --> c\nc --> d\n";
        let uid = uuid::Uuid::new_v4();
        let tmp_before = std::env::temp_dir().join(format!("diff_json_before_{}.spec", uid));
        let tmp_after  = std::env::temp_dir().join(format!("diff_json_after_{}.spec",  uid));
        std::fs::write(&tmp_before, before).unwrap();
        std::fs::write(&tmp_after,  after).unwrap();

        let exe = std::env::current_exe().unwrap();
        let target_dir = exe.parent().unwrap().parent().unwrap();
        let bin = target_dir.join("open-draftly");
        if !bin.exists() {
            eprintln!("skipping test_diff_json_output_is_valid_json: release binary not found at {:?}", bin);
            let _ = std::fs::remove_file(&tmp_before);
            let _ = std::fs::remove_file(&tmp_after);
            return;
        }
        let out = Command::new(&bin)
            .arg("diff")
            .arg(&tmp_before)
            .arg(&tmp_after)
            .arg("--json")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|e| panic!("diff --json did not emit valid JSON: {}\n---\n{}", e, stdout));

        // Top-level schema keys every consumer should rely on
        for key in ["before", "after", "added_nodes", "removed_nodes",
                    "modified_nodes", "added_edges", "removed_edges",
                    "modified_edges", "summary", "clean"] {
            assert!(parsed.get(key).is_some(), "missing top-level key `{}` in diff --json", key);
        }

        // clean must be a boolean, not an accidental stringified value
        assert_eq!(parsed.get("clean").and_then(|v| v.as_bool()), Some(false),
            "clean should be false when there are changes");

        // added_nodes: ["d"]
        let added = parsed.get("added_nodes").unwrap().as_array().unwrap();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].as_str(), Some("d"));

        // removed_nodes: empty
        assert_eq!(parsed.get("removed_nodes").unwrap().as_array().unwrap().len(), 0);

        // modified_nodes: [{ key: "c", changes: ["shape: diamond → circle"] }]
        let modified_nodes = parsed.get("modified_nodes").unwrap().as_array().unwrap();
        assert_eq!(modified_nodes.len(), 1);
        assert_eq!(modified_nodes[0].get("key").and_then(|v| v.as_str()), Some("c"));
        let c_changes = modified_nodes[0].get("changes").unwrap().as_array().unwrap();
        assert!(c_changes.iter().any(|c| c.as_str().unwrap_or("").contains("shape:")),
            "expected a shape change for node c, got {:?}", c_changes);

        // added_edges: [{ source: "c", target: "d", label: "" }]
        let added_edges = parsed.get("added_edges").unwrap().as_array().unwrap();
        assert_eq!(added_edges.len(), 1);
        assert_eq!(added_edges[0].get("source").and_then(|v| v.as_str()), Some("c"));
        assert_eq!(added_edges[0].get("target").and_then(|v| v.as_str()), Some("d"));

        // modified_edges: [{ source: "a", target: "b", changes: [label change] }]
        let modified_edges = parsed.get("modified_edges").unwrap().as_array().unwrap();
        assert_eq!(modified_edges.len(), 1);
        assert_eq!(modified_edges[0].get("source").and_then(|v| v.as_str()), Some("a"));
        assert_eq!(modified_edges[0].get("target").and_then(|v| v.as_str()), Some("b"));
        let ab_changes = modified_edges[0].get("changes").unwrap().as_array().unwrap();
        assert!(ab_changes.iter().any(|c| c.as_str().unwrap_or("").contains("label")),
            "expected a label change for edge a→b, got {:?}", ab_changes);

        // summary: sum of all deltas = total_changes
        let summary = parsed.get("summary").unwrap();
        assert_eq!(summary.get("added_nodes").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(summary.get("removed_nodes").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(summary.get("modified_nodes").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(summary.get("added_edges").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(summary.get("removed_edges").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(summary.get("modified_edges").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(summary.get("total_changes").and_then(|v| v.as_u64()), Some(4));

        let _ = std::fs::remove_file(&tmp_before);
        let _ = std::fs::remove_file(&tmp_after);
    }

    #[test]
    fn test_diff_json_clean_when_identical() {
        // Regression: identical specs must produce clean=true and zero
        // deltas across every array. Catches off-by-one errors in the
        // delta accumulation.
        use std::process::Command;
        let spec = "## Nodes\n- [a] A\n- [b] B\n\n## Flow\na --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp_a = std::env::temp_dir().join(format!("diff_clean_a_{}.spec", uid));
        let tmp_b = std::env::temp_dir().join(format!("diff_clean_b_{}.spec", uid));
        std::fs::write(&tmp_a, spec).unwrap();
        std::fs::write(&tmp_b, spec).unwrap();

        let exe = std::env::current_exe().unwrap();
        let bin = exe.parent().unwrap().parent().unwrap().join("open-draftly");
        if !bin.exists() {
            eprintln!("skipping test_diff_json_clean_when_identical: release binary not found");
            let _ = std::fs::remove_file(&tmp_a);
            let _ = std::fs::remove_file(&tmp_b);
            return;
        }
        let out = Command::new(&bin).arg("diff").arg(&tmp_a).arg(&tmp_b).arg("--json").output().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .expect("identical diff --json should still be valid JSON");
        assert_eq!(parsed.get("clean").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(parsed.pointer("/summary/total_changes").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(parsed.get("added_nodes").unwrap().as_array().unwrap().len(), 0);
        assert_eq!(parsed.get("removed_nodes").unwrap().as_array().unwrap().len(), 0);
        assert_eq!(parsed.get("modified_nodes").unwrap().as_array().unwrap().len(), 0);
        assert_eq!(parsed.get("added_edges").unwrap().as_array().unwrap().len(), 0);
        assert_eq!(parsed.get("removed_edges").unwrap().as_array().unwrap().len(), 0);
        assert_eq!(parsed.get("modified_edges").unwrap().as_array().unwrap().len(), 0);

        let _ = std::fs::remove_file(&tmp_a);
        let _ = std::fs::remove_file(&tmp_b);
    }

    #[test]
    fn test_lint_lane_ref_typo_via_cli() {
        // `{lane:Enginering}` (typo for `Engineering`) used to silently
        // auto-create a phantom empty lane in layout.rs with no warning.
        // Verify cli_lint now emits a did-you-mean hint against the
        // declared lane vocabulary from ## Lane N: sections.
        let spec = "## Lane 1: Engineering\n\
                    ## Lane 2: Design\n\
                    ## Nodes\n\
                    - [a] Alpha {lane:Enginering}\n\
                    - [b] Beta {lane:Design}\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("lane_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("{lane:Enginering}")
                && joined.contains("did you mean `{lane:Engineering}`"),
            "expected lane:Enginering did-you-mean hint, got: {stdout}"
        );
        // Valid `{lane:Design}` on the same node must NOT warn.
        let bad_valid = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("{lane:Design}") && s.contains("does not match any declared lane")
        });
        assert!(!bad_valid, "valid {{lane:Design}} must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_lane_ref_nonsense_falls_back_via_cli() {
        // Unresolvable lane references with no close match (e.g. `zxqwrty`)
        // should still warn, falling back to a generic "does not match any
        // declared lane" message instead of a did-you-mean suggestion.
        let spec = "## Lane 1: Engineering\n\
                    ## Lane 2: Design\n\
                    ## Nodes\n\
                    - [a] Alpha {lane:zxqwrty}\n\
                    - [b] Beta {lane:Design}\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("lane_nonsense_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("{lane:zxqwrty}")
                && joined.contains("does not match any declared lane in"),
            "expected generic fallback for unresolvable lane, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_lane_ref_auto_discovered_not_flagged_via_cli() {
        // Backwards-compat: when NO lane is explicitly declared via a
        // ## Swimlane: / ## Lane N: / ## Kanban: section, `{lane:X}` tags
        // should continue to auto-discover lanes without warning. The new
        // lint only fires against an explicit declaration vocabulary.
        let spec = "## Nodes\n\
                    - [a] Alpha {lane:Engineering}\n\
                    - [b] Beta {lane:Design}\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("lane_auto_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("does not match any declared lane")
        });
        assert!(!bad, "auto-discovered lanes must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_phase_ref_typo_via_cli() {
        // `{phase:Planing}` (typo for `Planning`) used to silently drop
        // the node into an "unperioded" bucket far below the grid with
        // no feedback — effectively vanishing the node off-canvas.
        // Verify cli_lint now emits a did-you-mean hint against the
        // declared period vocabulary from ## Period N: sections.
        let spec = "## Period 1: Planning\n\
                    ## Period 2: Execution\n\
                    ## Nodes\n\
                    - [a] Alpha {phase:Planing}\n\
                    - [b] Beta {phase:Execution}\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("phase_typo_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("{phase:Planing}")
                && joined.contains("did you mean `{phase:Planning}`"),
            "expected phase:Planing did-you-mean hint, got: {stdout}"
        );
        // Valid `{phase:Execution}` on the same node must NOT warn.
        let bad_valid = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("{phase:Execution}") && s.contains("does not match any declared period")
        });
        assert!(!bad_valid, "valid {{phase:Execution}} must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_phase_ref_nonsense_falls_back_via_cli() {
        // Unresolvable phase references with no close match should still
        // warn, falling back to a generic "does not match any declared
        // period" message instead of a did-you-mean suggestion.
        let spec = "## Period 1: Planning\n\
                    ## Period 2: Execution\n\
                    ## Nodes\n\
                    - [a] Alpha {phase:zxqwrty}\n\
                    - [b] Beta {phase:Execution}\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("phase_nonsense_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("{phase:zxqwrty}")
                && joined.contains("does not match any declared period in ## Timeline"),
            "expected generic fallback for unresolvable phase, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_phase_ref_auto_discovered_not_flagged_via_cli() {
        // Backwards-compat: when NO period is declared via a ## Period N:
        // section, `{phase:X}` tags should continue to auto-discover
        // periods without warning.
        let spec = "## Nodes\n\
                    - [a] Alpha {phase:Q1}\n\
                    - [b] Beta {phase:Q2}\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("phase_auto_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("does not match any declared period")
        });
        assert!(!bad, "auto-discovered phases must not warn, got: {stdout}");
    }

    #[test]
    fn test_lint_layer_config_bare_key_via_cli() {
        // `layer = Frontend` in ## Config (no digit suffix) used to be
        // silently dropped by the `if let Ok(idx) = ... parse` arm with
        // no else branch, AND the fallthrough unknown-config-keys arm
        // specifically skips `layer*` keys — so the user got zero feedback
        // and their "Frontend" label never appeared on any layer. Verify
        // cli_lint now surfaces it with a concrete fix example.
        let spec = "## Config\n\
                    title = Test\n\
                    layer = Frontend\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_bare_key_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("`layer = Frontend`")
                && joined.contains("not a valid layer index key")
                && joined.contains("layer0")
                && joined.contains("layer1 = Frontend"),
            "expected bare-layer-key warning with concrete fix example, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_layer_config_typo_key_via_cli() {
        // `layerfoo = Backend` — the `foo` suffix has no digits, so
        // `trim_matches(non_digit).parse::<i32>()` fails and the layer
        // name is silently dropped. Verify the typo surfaces in lint.
        let spec = "## Config\n\
                    layerfoo = Backend\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_typo_key_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let joined: String = warnings.iter()
            .filter_map(|w| w.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("`layerfoo = Backend`")
                && joined.contains("`layerfoo` is not a valid layer index key"),
            "expected layerfoo-key warning, got: {stdout}"
        );
    }

    #[test]
    fn test_lint_layer_config_valid_keys_not_flagged_via_cli() {
        // Regression guard: canonical `layer0 = Base`, `layer1 = Frontend`
        // must not fire the warning. Only truly invalid keys (no digit
        // anywhere) should surface.
        let spec = "## Config\n\
                    layer0 = Base\n\
                    layer1 = Frontend\n\
                    layer2 = Overlay\n\
                    ## Nodes\n\
                    - [a] Alpha\n\
                    - [b] Beta\n\
                    ## Flow\n\
                    a --> b\n";
        let uid = uuid::Uuid::new_v4();
        let tmp = std::env::temp_dir().join(format!("layer_valid_key_{}.spec", uid));
        std::fs::write(&tmp, spec).unwrap();
        let bin = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .and_then(|p| p.parent().map(|q| q.to_path_buf()))
            .map(|p| p.join("open-draftly"));
        let Some(bin) = bin else { let _ = std::fs::remove_file(&tmp); return; };
        if !bin.exists() { let _ = std::fs::remove_file(&tmp); return; }
        let out = std::process::Command::new(&bin)
            .args(["lint", "--json", tmp.to_str().unwrap()])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let _ = std::fs::remove_file(&tmp);
        let v: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("lint --json not valid JSON: {stdout}"));
        let warnings = v["warnings"].as_array().expect("warnings array");
        let bad = warnings.iter().any(|w| {
            let s = w.as_str().unwrap_or("");
            s.contains("is not a valid layer index key")
        });
        assert!(!bad, "canonical layerN keys must not warn, got: {stdout}");
    }
}
