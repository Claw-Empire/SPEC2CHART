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
        Some(Commands::Diff { before, after }) => {
            cli_diff(before, after);
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
        Some(Commands::Lint { input, strict }) => {
            cli_lint(input, strict);
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

fn cli_diff(before: PathBuf, after: PathBuf) {
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

    let edges_a: std::collections::HashSet<String> = doc_a.edges.iter()
        .map(|e| format!("{} → {}",
            id_to_key_a.get(&e.source.node_id).map(String::as_str).unwrap_or("?"),
            id_to_key_a.get(&e.target.node_id).map(String::as_str).unwrap_or("?")))
        .collect();
    let edges_b: std::collections::HashSet<String> = doc_b.edges.iter()
        .map(|e| format!("{} → {}",
            id_to_key_b.get(&e.source.node_id).map(String::as_str).unwrap_or("?"),
            id_to_key_b.get(&e.target.node_id).map(String::as_str).unwrap_or("?")))
        .collect();

    let mut added_edges: Vec<&String> = edges_b.difference(&edges_a).collect();
    let mut removed_edges: Vec<&String> = edges_a.difference(&edges_b).collect();
    added_edges.sort();
    removed_edges.sort();

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
    for e in &added_edges {
        println!("+ edge: {}", e);
    }
    for e in &removed_edges {
        println!("- edge: {}", e);
    }

    let total_changes = added_nodes.len() + removed_nodes.len() + modified_nodes.len()
        + added_edges.len() + removed_edges.len();
    if total_changes == 0 {
        println!("✓ No differences");
    } else {
        println!();
        println!("Summary: +{} nodes, -{} nodes, ~{} modified, +{} edges, -{} edges",
            added_nodes.len(), removed_nodes.len(), modified_nodes.len(),
            added_edges.len(), removed_edges.len());
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
        let mut shape_list: Vec<_> = shapes.iter().collect();
        shape_list.sort_by(|a, b| b.1.cmp(a.1));
        let mut section_list: Vec<_> = sections.iter().collect();
        section_list.sort_by(|a, b| b.1.cmp(a.1));
        println!("{{");
        println!("  \"file\": {:?},", input.display().to_string());
        println!("  \"nodes\": {},", node_count);
        println!("  \"edges\": {},", edge_count);
        println!("  \"frames\": {},", frames);
        println!("  \"disconnected_nodes\": {},", disconnected);
        println!("  \"locked_nodes\": {},", locked);
        println!("  \"nodes_with_comments\": {},", with_comment);
        println!("  \"nodes_with_urls\": {},", with_url);
        println!("  \"nodes_with_owners\": {},", with_owner);
        println!("  \"labeled_edges\": {},", labeled_edges);
        println!("  \"max_in_degree\": {},", max_in);
        println!("  \"max_out_degree\": {},", max_out);
        println!("  \"layout_depth\": {},", layout_depth);
        println!("  \"connected_components\": {},", component_count);
        println!("  \"edge_density\": {:.3},", edge_density);
        let mut tag_list: Vec<_> = tags.iter().collect();
        tag_list.sort_by(|a, b| b.1.cmp(a.1));
        println!("  \"tags\": {{");
        for (i, (name, count)) in tag_list.iter().enumerate() {
            let comma = if i + 1 < tag_list.len() { "," } else { "" };
            println!("    \"{}\": {}{}", name, count, comma);
        }
        println!("  }},");
        println!("  \"shapes\": {{");
        for (i, (name, count)) in shape_list.iter().enumerate() {
            let comma = if i + 1 < shape_list.len() { "," } else { "" };
            println!("    \"{}\": {}{}", name, count, comma);
        }
        println!("  }},");
        println!("  \"sections\": {{");
        for (i, (name, count)) in section_list.iter().enumerate() {
            let comma = if i + 1 < section_list.len() { "," } else { "" };
            println!("    \"{}\": {}{}", name, count, comma);
        }
        println!("  }}");
        println!("}}");
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

fn cli_lint(input: PathBuf, strict: bool) {
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
}
