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
    List,
    /// Print a template's HRF content (use --out to write to a file)
    Get {
        /// Template name (case-insensitive, e.g. "Architecture")
        name: String,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

fn main() -> eframe::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Render { input, out, format }) => {
            cli_render(input, out, &format);
            return Ok(());
        }
        Some(Commands::Validate { input }) => {
            cli_validate(input);
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
                TemplatesCmd::List => cli_templates_list(),
                TemplatesCmd::Get { name, out } => cli_templates_get(&name, out.as_deref()),
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

fn cli_validate(input: PathBuf) {
    let spec = std::fs::read_to_string(&input)
        .unwrap_or_else(|e| { eprintln!("Error reading {:?}: {}", input, e); std::process::exit(1); });
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

fn cli_templates_list() {
    use crate::templates::TEMPLATES;
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

fn cli_templates_get(name: &str, out: Option<&std::path::Path>) {
    use crate::templates::TEMPLATES;
    let name_lower = name.to_lowercase();
    let template = TEMPLATES.iter().find(|t| t.name.to_lowercase() == name_lower)
        .unwrap_or_else(|| {
            eprintln!("Template {:?} not found. Run `templates list` to see available templates.", name);
            std::process::exit(1);
        });
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

    // Layout depth (longest path via BFS layering, same as layout engine)
    let layout_depth = {
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
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                let cand = layer[u] + 1;
                if cand > layer[v] { layer[v] = cand; }
                rem[v] -= 1;
                if rem[v] == 0 { queue.push_back(v); }
            }
        }
        layer.into_iter().max().unwrap_or(0) as usize + 1
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
        println!(
            "  Components:        {} subgraph{}",
            component_count,
            if component_count == 1 { "" } else { "s" }
        );
        println!("  Edge density:      {:.1}%", edge_density * 100.0);
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

fn cli_lint(input: PathBuf, strict: bool, json: bool) {
    let doc = load_doc(&input);
    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    // Shape-tag typo detection: inspect each node's `unknown_tags` (preserved
    // by the HRF parser for tags that no handler claimed) and suggest the
    // closest known shape alias when the distance is small enough.
    for node in &doc.nodes {
        for tag in &node.unknown_tags {
            if let Some(suggestion) = crate::specgraph::hrf::suggest_shape_alias(tag) {
                let id_str = if node.hrf_id.is_empty() {
                    node.display_label().to_string()
                } else {
                    format!("[{}]", node.hrf_id)
                };
                warnings.push(format!(
                    "Node {}: unknown tag {{{}}} — did you mean {{{}}}?",
                    id_str, tag, suggestion
                ));
            }
        }
    }

    // Edge-style tag typo detection: mirror the node check on edges, using
    // the edge-style vocabulary (dashed/dotted/thick/ortho/escalate/...). Also
    // runs `suggest_arrow_style` for `arrow:*` sub-tags, which the bare-word
    // suggestor explicitly skips.
    for edge in &doc.edges {
        for tag in &edge.unknown_tags {
            let suggestion = crate::specgraph::hrf::suggest_edge_style_alias(tag)
                .or_else(|| crate::specgraph::hrf::suggest_arrow_style(tag));
            if let Some(suggestion) = suggestion {
                let label = if edge.label.trim().is_empty() {
                    // Fall back to endpoint hrf_ids so the message is actionable
                    let src = doc.nodes.iter().find(|n| n.id == edge.source.node_id)
                        .map(|n| if n.hrf_id.is_empty() { n.display_label().to_string() } else { format!("[{}]", n.hrf_id) })
                        .unwrap_or_else(|| "?".into());
                    let tgt = doc.nodes.iter().find(|n| n.id == edge.target.node_id)
                        .map(|n| if n.hrf_id.is_empty() { n.display_label().to_string() } else { format!("[{}]", n.hrf_id) })
                        .unwrap_or_else(|| "?".into());
                    format!("{} → {}", src, tgt)
                } else {
                    format!("{:?}", edge.label)
                };
                warnings.push(format!(
                    "Edge {}: unknown tag {{{}}} — did you mean {{{}}}?",
                    label, tag, suggestion
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

    // Check for self-loops
    for edge in &doc.edges {
        if edge.source.node_id == edge.target.node_id {
            warnings.push(format!("Self-loop on node {}",
                doc.nodes.iter().find(|n| n.id == edge.source.node_id)
                    .map(|n| n.display_label().to_string())
                    .unwrap_or_else(|| "?".into())));
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
            let tmp = std::env::temp_dir().join(format!(
                "test_svg_export_{}_{}.svg",
                template.name.replace(' ', "_"),
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
}
