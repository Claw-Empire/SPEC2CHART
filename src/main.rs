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

fn cli_generate(_template: &str) {
    eprintln!("generate: not yet implemented (coming in Task 5.3)");
    std::process::exit(1);
}

fn cli_watch(_directory: PathBuf, _out: PathBuf, _template: &str) {
    eprintln!("watch: not yet implemented (coming in Task 5.3)");
    std::process::exit(1);
}

fn cli_serve(_port: u16) {
    eprintln!("serve: not yet implemented (coming in Task 5.3)");
    std::process::exit(1);
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
