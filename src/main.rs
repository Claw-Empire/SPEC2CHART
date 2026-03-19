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
#[command(name = "light-figma", about = "Lightweight diagramming tool")]
struct Cli {
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
    /// Generate HRF from prose via LLM (requires ANTHROPIC_API_KEY)
    Generate {
        #[arg(long, default_value = "")]
        template: String,
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
        Some(Commands::Generate { template }) => {
            cli_generate(&template);
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
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 860.0])
            .with_title("Light Figma"),
        ..Default::default()
    };
    eframe::run_native(
        "Light Figma",
        options,
        Box::new(|cc| Ok(Box::new(app::FlowchartApp::new(cc)))),
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
        "HRF (Human-Readable Format) for light-figma diagrams.\n\
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

    let ids_a: std::collections::HashSet<String> = doc_a.nodes.iter().map(|n| n.hrf_id.clone()).collect();
    let ids_b: std::collections::HashSet<String> = doc_b.nodes.iter().map(|n| n.hrf_id.clone()).collect();

    for id in ids_b.difference(&ids_a) {
        println!("+ node: {}", id);
    }
    for id in ids_a.difference(&ids_b) {
        println!("- node: {}", id);
    }

    let edges_a: std::collections::HashSet<String> = doc_a.edges.iter()
        .map(|e| format!("{} → {}", e.source.node_id.0, e.target.node_id.0)).collect();
    let edges_b: std::collections::HashSet<String> = doc_b.edges.iter()
        .map(|e| format!("{} → {}", e.source.node_id.0, e.target.node_id.0)).collect();

    for e in edges_b.difference(&edges_a) {
        println!("+ edge: {}", e);
    }
    for e in edges_a.difference(&edges_b) {
        println!("- edge: {}", e);
    }
}

fn cli_generate(template: &str) {
    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: ANTHROPIC_API_KEY environment variable not set.\nSet it with: export ANTHROPIC_API_KEY=your-key");
        std::process::exit(1);
    });
    let mut prose = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut prose).unwrap();
    match crate::specgraph::llm::prose_to_hrf(&prose, template, &api_key) {
        Ok(hrf) => print!("{}", hrf),
        Err(e) => {
            eprintln!("LLM error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cli_watch(directory: PathBuf, out: PathBuf, template: &str) {
    use notify::{Watcher, RecursiveMode, Event};
    use std::sync::mpsc::channel;

    println!("Watching {:?} → {:?}", directory, out);
    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx).unwrap();
    watcher.watch(&directory, RecursiveMode::Recursive).unwrap();

    // Initial render
    regenerate_watch(&directory, &out, template);

    for res in rx {
        if let Ok(event) = res {
            if event.paths.iter().any(|p| p.extension().map_or(false, |e| e == "spec")) {
                println!("Change detected — regenerating...");
                regenerate_watch(&directory, &out, template);
            }
        }
    }
}

fn regenerate_watch(dir: &std::path::Path, out: &std::path::Path, _template: &str) {
    if let Some(spec_path) = std::fs::read_dir(dir)
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(|e| e.ok())
                .find(|e| e.path().extension().map_or(false, |x| x == "spec"))
        })
        .map(|e| e.path())
    {
        cli_render(spec_path, out.to_path_buf());
    }
}

fn cli_serve(port: u16) {
    use tiny_http::{Server, Response, Header};
    let addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&addr).unwrap_or_else(|e| {
        eprintln!("Failed to start server on {}: {}", addr, e);
        std::process::exit(1);
    });
    println!("light-figma render server listening on http://localhost:{}", port);
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
                let tmp = std::env::temp_dir().join("lf_serve_tmp.svg");
                match crate::export::export_svg(&doc, &tmp) {
                    Ok(()) => {
                        let svg = std::fs::read_to_string(&tmp).unwrap_or_default();
                        let response = Response::from_string(svg).with_header(
                            Header::from_bytes("Content-Type", "image/svg+xml").unwrap(),
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
        let tmp = std::env::temp_dir().join("test_render_cli.svg");
        crate::export::export_svg(&doc, &tmp).unwrap();
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("<svg"));
        assert!(content.contains("Alpha"));
    }
}
