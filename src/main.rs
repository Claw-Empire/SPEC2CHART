mod app;
mod export;
mod history;
mod io;
mod model;
mod specgraph;
mod templates;

use eframe::egui;

fn main() -> eframe::Result<()> {
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
