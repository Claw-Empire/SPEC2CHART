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
#[command(name = "open-atlas", about = "openAtlas — lightweight diagramming tool")]
struct Cli {
    /// Optional .spec or .hrf file to open on launch
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a .spec file to SVG without opening the GUI
    Render {
        input: PathBuf,
        #[arg(short, long)]
        out: PathBuf,
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
    /// Watch a directory and regenerate SVG on file changes
    Watch {
        directory: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "")]
        template: String,
    },
    /// Start local HTTP render server (POST /render → SVG)
    Serve {
        #[arg(long, default_value = "8080")]
        port: u16,
    },
}

fn main() -> eframe::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Render { input, out }) => {
            cli_render(input, out);
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
        Some(Commands::Watch {
            directory,
            out,
            template,
        }) => {
            cli_watch(directory, out, &template);
            return Ok(());
        }
        Some(Commands::Serve { port }) => {
            cli_serve(port);
            return Ok(());
        }
        None => {} // Fall through to GUI
    }

    // GUI mode (no subcommand)
    let startup_file = cli.file;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 860.0])
            .with_title("openAtlas"),
        ..Default::default()
    };
    eframe::run_native(
        "openAtlas",
        options,
        Box::new(move |cc| Ok(Box::new(app::FlowchartApp::new_with_file(cc, startup_file)))),
    )
}

fn cli_render(input: PathBuf, out: PathBuf) {
    let spec = std::fs::read_to_string(&input)
        .unwrap_or_else(|e| { eprintln!("Error reading {:?}: {}", input, e); std::process::exit(1); });
    let mut doc = crate::specgraph::hrf::parse_hrf(&spec)
        .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    crate::specgraph::layout::auto_layout(&mut doc);
    crate::export::export_svg(&doc, &out)
        .unwrap_or_else(|e| { eprintln!("Export error: {}", e); std::process::exit(1); });
    println!("Rendered {:?} → {:?}", input, out);
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
        "HRF (Human-Readable Format) for openAtlas diagrams.\n\
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
    let ids_a: std::collections::HashSet<String> = doc_a.nodes.iter().map(|n| node_key(n)).collect();
    let ids_b: std::collections::HashSet<String> = doc_b.nodes.iter().map(|n| node_key(n)).collect();

    for id in ids_b.difference(&ids_a) {
        println!("+ node: {}", id);
    }
    for id in ids_a.difference(&ids_b) {
        println!("- node: {}", id);
    }

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

    for e in edges_b.difference(&edges_a) {
        println!("+ edge: {}", e);
    }
    for e in edges_a.difference(&edges_b) {
        println!("- edge: {}", e);
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

fn cli_watch(directory: PathBuf, out: PathBuf, template: &str) {
    use notify::{Watcher, RecursiveMode};
    use std::sync::mpsc::channel;

    println!("Watching {:?} → {:?}", directory, out);
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(tx)
        .unwrap_or_else(|e| { eprintln!("Watch error: {}", e); std::process::exit(1); });
    watcher.watch(&directory, RecursiveMode::Recursive)
        .unwrap_or_else(|e| { eprintln!("Watch error: {}", e); std::process::exit(1); });

    // Initial render
    regenerate_watch(&directory, &out, template);

    for event in rx.into_iter().flatten() {
        if event.paths.iter().any(|p| p.extension().is_some_and(|e| e == "spec")) {
            println!("Change detected — regenerating...");
            regenerate_watch(&directory, &out, template);
        }
    }
}

fn regenerate_watch(dir: &std::path::Path, out: &std::path::Path, _template: &str) {
    let mut spec_files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "spec"))
        .collect();
    spec_files.sort();
    if let Some(spec_path) = spec_files.into_iter().next() {
        cli_render(spec_path, out.to_path_buf());
    }
}

fn cli_serve(port: u16) {
    use tiny_http::{Server, Response, Header};
    let addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&addr).unwrap_or_else(|e| {
        eprintln!("Failed to start server on {}: {}", addr, e);
        std::process::exit(1);
    });
    println!("openAtlas render server listening on http://localhost:{}", port);
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
}
