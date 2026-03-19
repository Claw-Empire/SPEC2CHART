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

fn cli_render(_input: PathBuf, _out: PathBuf) {
    eprintln!("render: not yet implemented (coming in Task 5.2)");
    std::process::exit(1);
}

fn cli_validate(_input: PathBuf) {
    eprintln!("validate: not yet implemented (coming in Task 5.2)");
    std::process::exit(1);
}

fn cli_schema(_template: &str) {
    eprintln!("schema: not yet implemented (coming in Task 5.2)");
    std::process::exit(1);
}

fn cli_diff(_before: PathBuf, _after: PathBuf) {
    eprintln!("diff: not yet implemented (coming in Task 5.2)");
    std::process::exit(1);
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
