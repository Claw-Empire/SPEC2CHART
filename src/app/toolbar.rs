use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Sense, SidePanel, Stroke, StrokeKind,
};
use crate::export;
use crate::io;
use crate::model::*;
use crate::specgraph;
use super::{FlowchartApp, DiagramMode, DragState, Tool};
use super::theme::{TOOLBAR_WIDTH, CANVAS_BG_PRESETS, to_color32};

impl FlowchartApp {
    pub(crate) fn draw_toolbar(&mut self, ctx: &egui::Context) {
        // Collapsed: show a thin strip with just an expand button
        if self.toolbar_collapsed {
            SidePanel::left("toolbar")
                .resizable(false)
                .exact_width(28.0)
                .frame(egui::Frame {
                    fill: self.theme.mantle,
                    inner_margin: egui::Margin::same(0),
                    stroke: Stroke::new(1.0, self.theme.surface1),
                    ..Default::default()
                })
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(8.0);
                        let btn = egui::Button::new(
                            egui::RichText::new("▶").size(11.0).color(self.theme.text_dim)
                        ).fill(egui::Color32::TRANSPARENT).frame(false);
                        if ui.add(btn).on_hover_text("Expand toolbar").clicked() {
                            self.toolbar_collapsed = false;
                        }
                    });
                });
            return;
        }

        SidePanel::left("toolbar")
            .resizable(false)
            .exact_width(TOOLBAR_WIDTH)
            .frame(egui::Frame {
                fill: self.theme.mantle,
                inner_margin: egui::Margin { left: 12, right: 6, top: 10, bottom: 8 },
                stroke: Stroke::new(1.0, self.theme.surface1),
                ..Default::default()
            })
            .show(ctx, |ui| {
                // Collapse button row at very top
                ui.horizontal(|ui| {
                    ui.add_space(0.0);
                    ui.label(
                        egui::RichText::new("Light Figma")
                            .size(18.0)
                            .strong()
                            .color(self.theme.text_primary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(6.0);
                        let btn = egui::Button::new(
                            egui::RichText::new("◀").size(10.0).color(self.theme.text_dim)
                        ).fill(egui::Color32::TRANSPARENT).frame(false);
                        if ui.add(btn).on_hover_text("Collapse toolbar").clicked() {
                            self.toolbar_collapsed = true;
                        }
                    });
                });
                ui.add_space(4.0);

                // All content in a scrollable area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min).with_cross_justify(true), |ui| {
                // Project title (shown as canvas watermark)
                ui.add(
                    egui::TextEdit::singleline(&mut self.project_title)
                        .hint_text("Project title…")
                        .desired_width(f32::INFINITY)
                        .font(egui::FontId::proportional(11.0)),
                );
                ui.add_space(8.0);

                // Undo / Redo buttons
                ui.horizontal(|ui| {
                    let can_undo = self.history.can_undo();
                    let can_redo = self.history.can_redo();
                    let u = self.history.undo_steps();
                    let r = self.history.redo_steps();
                    let undo_btn = egui::Button::new(egui::RichText::new(format!("↩ {u}")).size(12.0));
                    let redo_btn = egui::Button::new(egui::RichText::new(format!("{r} ↪")).size(12.0));
                    if ui.add_enabled(can_undo, undo_btn).on_hover_text("Undo (⌘Z)").clicked() {
                        if let Some(doc) = self.history.undo() {
                            self.document = doc.clone();
                            self.selection.clear();
                        }
                    }
                    if ui.add_enabled(can_redo, redo_btn).on_hover_text("Redo (⌘⇧Z)").clicked() {
                        if let Some(doc) = self.history.redo() {
                            self.document = doc.clone();
                            self.selection.clear();
                        }
                    }
                });
                ui.add_space(8.0);

                // File actions
                self.draw_divider(ui);
                ui.add_space(8.0);
                self.draw_section_header(ui, "File");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let btn_size = egui::vec2(84.0, 32.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(egui::RichText::new("Save").size(12.0)),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Flowchart", &["flow"])
                            .set_file_name("untitled.flow")
                            .save_file()
                        {
                            match io::save_document(&self.document, &path) {
                                Ok(()) => {
                                    self.status_message =
                                        Some(("Saved!".to_string(), std::time::Instant::now()));
                                }
                                Err(e) => eprintln!("Save error: {}", e),
                            }
                        }
                    }
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(egui::RichText::new("Open").size(12.0)),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Flowchart", &["flow"])
                            .pick_file()
                        {
                            match io::load_document(&path) {
                                Ok(doc) => {
                                    self.document = doc;
                                    self.selection.clear();
                                    self.history.push(&self.document);
                                }
                                Err(e) => eprintln!("Load error: {}", e),
                            }
                        }
                    }
                });
                ui.add_space(8.0);

                // Export
                self.draw_divider(ui);
                ui.add_space(8.0);
                self.draw_section_header(ui, "Export");
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    let btn_size = egui::vec2(54.0, 30.0);
                    for (label, ext, export_fn) in [
                        ("PNG", "png", "png" as &str),
                        ("SVG", "svg", "svg"),
                        ("PDF", "pdf", "pdf"),
                    ] {
                        if ui
                            .add_sized(
                                btn_size,
                                egui::Button::new(egui::RichText::new(label).size(11.0)),
                            )
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(label, &[ext])
                                .set_file_name(format!("flowchart.{}", ext))
                                .save_file()
                            {
                                let result = match export_fn {
                                    "png" => export::export_png(&self.document, &path),
                                    "svg" => export::export_svg(&self.document, &path),
                                    "pdf" => export::export_pdf(&self.document, &path),
                                    _ => Ok(()),
                                };
                                match result {
                                    Ok(()) => {
                                        self.status_message = Some((
                                            format!("Exported {}!", label),
                                            std::time::Instant::now(),
                                        ));
                                    }
                                    Err(e) => eprintln!("Export error: {}", e),
                                }
                            }
                        }
                    }
                });
                ui.add_space(8.0);

                // SpecGraph import/export (YAML, HRF, Prose)
                self.draw_divider(ui);
                ui.add_space(8.0);
                self.draw_section_header(ui, "Spec");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let btn_size = egui::vec2(84.0, 32.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(egui::RichText::new("Import").size(12.0)),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("All Specs", &["yaml", "yml", "spec", "txt", "md"])
                            .add_filter("YAML", &["yaml", "yml"])
                            .add_filter("Human Readable", &["spec", "md"])
                            .add_filter("Prose (LLM)", &["txt"])
                            .pick_file()
                        {
                            match std::fs::read_to_string(&path) {
                                Ok(text) => {
                                    let format = specgraph::detect_format(&text);
                                    let llm_cfg = if self.llm_config.api_key.is_empty() {
                                        None
                                    } else {
                                        Some(&self.llm_config)
                                    };
                                    match specgraph::import_auto(&text, llm_cfg) {
                                        Ok(doc) => {
                                            let fmt_name = match format {
                                                specgraph::SpecFormat::Yaml => "YAML",
                                                specgraph::SpecFormat::Hrf => "Spec",
                                                specgraph::SpecFormat::Prose => "Prose (LLM)",
                                            };
                                            // Apply import hints from ## Config section
                                            if let Some(z) = doc.import_hints.zoom {
                                                self.viewport.zoom = z;
                                            }
                                            if let Some(true) = doc.import_hints.view_3d {
                                                self.view_mode = super::ViewMode::ThreeD;
                                            }
                                            if let Some(yaw) = doc.import_hints.camera_yaw {
                                                self.camera3d.yaw = yaw;
                                            }
                                            if let Some(pitch) = doc.import_hints.camera_pitch {
                                                self.camera3d.pitch = pitch;
                                            }
                                            if let Some(bg) = doc.import_hints.canvas_bg {
                                                self.canvas_bg = bg;
                                            }
                                            if let Some(ref title) = doc.import_hints.project_title.clone() {
                                                self.project_title = title.clone();
                                            }
                                            self.document = doc;
                                            self.selection.clear();
                                            self.history.push(&self.document);
                                            self.pending_fit = true;
                                            self.status_message = Some((
                                                format!("Imported {}!", fmt_name),
                                                std::time::Instant::now(),
                                            ));
                                        }
                                        Err(e) => {
                                            self.status_message = Some((
                                                format!("Import error: {}", e),
                                                std::time::Instant::now(),
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    self.status_message = Some((
                                        format!("Read error: {}", e),
                                        std::time::Instant::now(),
                                    ));
                                }
                            }
                        }
                    }
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(egui::RichText::new("Export").size(12.0)),
                        )
                        .on_hover_text("Right-click for Human Readable format")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("YAML", &["yaml", "yml"])
                            .add_filter("Human Readable", &["spec"])
                            .set_file_name("diagram.yaml")
                            .save_file()
                        {
                            let is_hrf = path.extension()
                                .map_or(false, |ext| ext == "spec" || ext == "md");
                            let result = if is_hrf {
                                let bg_str = match self.bg_pattern {
                                    super::BgPattern::Dots => "dots",
                                    super::BgPattern::Lines => "lines",
                                    super::BgPattern::Crosshatch => "crosshatch",
                                    super::BgPattern::None => "none",
                                };
                                let is_3d = matches!(self.view_mode, super::ViewMode::ThreeD);
                                let vp = specgraph::hrf::ViewportExportConfig {
                                    bg_pattern: bg_str,
                                    snap: self.snap_to_grid,
                                    grid_size: self.grid_size,
                                    zoom: self.viewport.zoom,
                                    view_3d: is_3d,
                                    camera_yaw: if is_3d { Some(self.camera3d.yaw) } else { None },
                                    camera_pitch: if is_3d { Some(self.camera3d.pitch) } else { None },
                                };
                                Ok(specgraph::hrf::export_hrf_ex(&self.document, "Untitled Diagram", Some(&vp)))
                            } else {
                                specgraph::export_yaml(&self.document, "Untitled Diagram")
                            };
                            match result {
                                Ok(content) => match std::fs::write(&path, &content) {
                                    Ok(()) => {
                                        let fmt = if is_hrf { "Spec" } else { "YAML" };
                                        self.status_message = Some((
                                            format!("Exported {}!", fmt),
                                            std::time::Instant::now(),
                                        ));
                                    }
                                    Err(e) => {
                                        self.status_message = Some((
                                            format!("Write error: {}", e),
                                            std::time::Instant::now(),
                                        ));
                                    }
                                },
                                Err(e) => {
                                    self.status_message = Some((
                                        format!("Export error: {}", e),
                                        std::time::Instant::now(),
                                    ));
                                }
                            }
                        }
                    }
                });
                ui.add_space(2.0);
                // Live spec editor button
                if ui
                    .add_sized(
                        egui::vec2(ui.available_width(), 24.0),
                        egui::Button::new(
                            egui::RichText::new(if self.show_spec_editor { "▼ Live Spec Editor" } else { "▶ Live Spec Editor" })
                                .size(11.0)
                                .color(if self.show_spec_editor { self.theme.accent } else { self.theme.text_secondary }),
                        ).fill(Color32::TRANSPARENT),
                    )
                    .on_hover_text("Open side panel to edit HRF spec live (Cmd+E)")
                    .clicked()
                {
                    self.show_spec_editor = !self.show_spec_editor;
                    if self.show_spec_editor {
                        let title = self.document.title.clone();
                        self.spec_editor_text = crate::specgraph::hrf::export_hrf(&self.document, &title);
                        self.spec_editor_error = None;
                    }
                }
                ui.add_space(2.0);
                // Copy/paste spec clipboard row
                ui.horizontal(|ui| {
                    let half = (ui.available_width() - 4.0) / 2.0;
                    let btn_size = egui::vec2(half, 24.0);
                    if ui
                        .add_sized(btn_size, egui::Button::new(
                            egui::RichText::new("Copy Spec").size(11.0),
                        ).fill(Color32::TRANSPARENT))
                        .on_hover_text("Copy diagram as HRF spec (Cmd+Shift+S)")
                        .clicked()
                    {
                        let bg_str = match self.bg_pattern {
                            super::BgPattern::Dots => "dots",
                            super::BgPattern::Lines => "lines",
                            super::BgPattern::Crosshatch => "crosshatch",
                            super::BgPattern::None => "none",
                        };
                        let is_3d = matches!(self.view_mode, super::ViewMode::ThreeD);
                        let vp = specgraph::hrf::ViewportExportConfig {
                            bg_pattern: bg_str,
                            snap: self.snap_to_grid,
                            grid_size: self.grid_size,
                            zoom: self.viewport.zoom,
                            view_3d: is_3d,
                            camera_yaw: if is_3d { Some(self.camera3d.yaw) } else { None },
                            camera_pitch: if is_3d { Some(self.camera3d.pitch) } else { None },
                        };
                        let hrf = specgraph::hrf::export_hrf_ex(&self.document, "Untitled Diagram", Some(&vp));
                        ui.ctx().copy_text(hrf);
                        self.status_message = Some((
                            "Spec copied to clipboard".to_string(),
                            std::time::Instant::now(),
                        ));
                    }
                    if ui
                        .add_sized(btn_size, egui::Button::new(
                            egui::RichText::new(if self.show_spec_paste_area { "▼ Paste Spec" } else { "▶ Paste Spec" }).size(11.0),
                        ).fill(Color32::TRANSPARENT))
                        .on_hover_text("Open text area to paste/type a spec (Cmd+Shift+P)")
                        .clicked()
                    {
                        self.show_spec_paste_area = !self.show_spec_paste_area;
                        if self.show_spec_paste_area {
                            self.spec_paste_buf.clear();
                        }
                    }
                });
                // Paste spec text area
                if self.show_spec_paste_area {
                    ui.add_space(4.0);
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new("Paste spec text here:").size(10.0).color(self.theme.text_dim));
                        ui.add_space(2.0);
                        let te = egui::TextEdit::multiline(&mut self.spec_paste_buf)
                            .desired_width(ui.available_width())
                            .desired_rows(6)
                            .font(FontId::monospace(10.0))
                            .hint_text("# My Diagram\n\n## Nodes\n- [a] Node A\n\n## Flow\na --> b");
                        ui.add(te);
                        ui.add_space(4.0);
                        // Inline parse preview: show node/edge count + config hints if parseable
                        if !self.spec_paste_buf.trim().is_empty() {
                            let preview_text = match specgraph::hrf::parse_hrf(&self.spec_paste_buf) {
                                Ok(doc) => {
                                    let n = doc.nodes.len();
                                    let e = doc.edges.len();
                                    let layers = {
                                        let mut zs: Vec<f32> = doc.nodes.iter()
                                            .map(|nd| nd.z_offset)
                                            .collect();
                                        zs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                                        zs.dedup_by(|a, b| (*a - *b).abs() < 0.5);
                                        zs.len()
                                    };
                                    let title_bit = if doc.title.is_empty() { String::new() }
                                        else { format!("\"{}\"  ", doc.title) };
                                    let layers_bit = if layers > 1 { format!("  {}L", layers) } else { String::new() };
                                    let named_bit = if !doc.layer_names.is_empty() {
                                        format!("  ({} named)", doc.layer_names.len())
                                    } else { String::new() };
                                    // Config hints: 3D view, camera preset, zoom
                                    let view_bit = if doc.import_hints.view_3d == Some(true) {
                                        "  3D"
                                    } else { "" };
                                    let zoom_bit = if let Some(z) = doc.import_hints.zoom {
                                        format!("  zoom={:.0}%", z * 100.0)
                                    } else if doc.import_hints.auto_fit {
                                        "  zoom=fit".to_string()
                                    } else { String::new() };
                                    egui::RichText::new(format!(
                                        "✓ {}{} nodes, {} edges{}{}{}{}",
                                        title_bit, n, e, layers_bit, named_bit, view_bit, zoom_bit
                                    ))
                                        .size(9.5)
                                        .color(egui::Color32::from_rgb(166, 227, 161))
                                }
                                Err(ref err) => {
                                    let short = if err.len() > 72 { &err[..72] } else { err };
                                    egui::RichText::new(format!("✗ {}", short))
                                        .size(9.5)
                                        .color(egui::Color32::from_rgb(243, 139, 168))
                                }
                            };
                            ui.add_space(2.0);
                            ui.label(preview_text);
                            ui.add_space(2.0);
                        }
                        let import_btn = ui.add_sized(
                            egui::vec2(ui.available_width(), 26.0),
                            egui::Button::new(egui::RichText::new("Import").size(11.5).color(self.theme.accent)),
                        );
                        if import_btn.clicked() && !self.spec_paste_buf.trim().is_empty() {
                            let text = self.spec_paste_buf.clone();
                            let llm_cfg = if self.llm_config.api_key.is_empty() {
                                None
                            } else { Some(&self.llm_config) };
                            match specgraph::import_auto(&text, llm_cfg) {
                                Ok(doc) => {
                                    // Apply ## Config import hints before replacing document
                                    if let Some(bg) = doc.import_hints.bg_pattern.as_deref() {
                                        self.bg_pattern = match bg {
                                            "dots" | "dot" => super::BgPattern::Dots,
                                            "lines" | "line" | "grid" => super::BgPattern::Lines,
                                            "crosshatch" | "cross" | "hash" => super::BgPattern::Crosshatch,
                                            "none" | "off" | "blank" => super::BgPattern::None,
                                            _ => self.bg_pattern,
                                        };
                                    }
                                    if let Some(snap) = doc.import_hints.snap {
                                        self.snap_to_grid = snap;
                                    }
                                    if let Some(gs) = doc.import_hints.grid_size {
                                        self.grid_size = gs;
                                    }
                                    // Apply zoom: specific value skips auto-fit; "fit"/"auto" triggers it.
                                    let specific_zoom = doc.import_hints.zoom;
                                    if let Some(z) = specific_zoom {
                                        self.viewport.zoom = z;
                                    }
                                    // Apply 3D camera hints
                                    if let Some(yaw) = doc.import_hints.camera_yaw {
                                        self.camera3d.yaw = yaw;
                                    }
                                    if let Some(pitch) = doc.import_hints.camera_pitch {
                                        self.camera3d.pitch = pitch;
                                    }
                                    if let Some(true) = doc.import_hints.view_3d {
                                        self.view_mode = super::ViewMode::ThreeD;
                                    }
                                    if let Some(bg) = doc.import_hints.canvas_bg {
                                        self.canvas_bg = bg;
                                    }
                                    if let Some(ref title) = doc.import_hints.project_title.clone() {
                                        self.project_title = title.clone();
                                    }
                                    // Fit to content unless a specific zoom level was given
                                    let do_fit = doc.import_hints.auto_fit || specific_zoom.is_none();
                                    self.document = doc;
                                    self.selection.clear();
                                    self.history.push(&self.document);
                                    self.pending_fit = do_fit;
                                    self.show_spec_paste_area = false;
                                    self.spec_paste_buf.clear();
                                    self.status_message = Some((
                                        "Spec imported!".to_string(),
                                        std::time::Instant::now(),
                                    ));
                                }
                                Err(e) => {
                                    self.status_message = Some((
                                        format!("Import error: {}", e),
                                        std::time::Instant::now(),
                                    ));
                                }
                            }
                        }
                    });
                }
                ui.add_space(2.0);
                // Example specs — categorized
                let arch_specs: &[(&str, &str)] = &[
                    ("Flow", include_str!("../../assets/examples/flowchart.spec")),
                    ("3D Arch", include_str!("../../assets/examples/architecture_3d.spec")),
                    ("Cloud", include_str!("../../assets/examples/cloud_arch.spec")),
                    ("µSvc", include_str!("../../assets/examples/microservices.spec")),
                    ("ER", include_str!("../../assets/examples/er_diagram.spec")),
                    ("Network", include_str!("../../assets/examples/network.spec")),
                    ("Dashboard", include_str!("../../assets/examples/project_dashboard.spec")),
                    ("Showcase", include_str!("../../assets/examples/feature_showcase.spec")),
                ];
                let design_specs: &[(&str, &str)] = &[
                    ("Hypothesis", include_str!("../../assets/examples/hypothesis_map.spec")),
                    ("SWOT", include_str!("../../assets/examples/swot_analysis.spec")),
                    ("Roadmap", include_str!("../../assets/examples/timeline_roadmap.spec")),
                    ("Force Field", include_str!("../../assets/examples/force_field.spec")),
                    ("5 Whys", include_str!("../../assets/examples/five_whys.spec")),
                    ("OKR Tree", include_str!("../../assets/examples/okr_tree.spec")),
                    ("Lean Canvas", include_str!("../../assets/examples/lean_canvas.spec")),
                    ("Impact/Effort", include_str!("../../assets/examples/impact_effort.spec")),
                    ("Journey Map", include_str!("../../assets/examples/customer_journey.spec")),
                    ("Decision Log", include_str!("../../assets/examples/decision_record.spec")),
                    ("Empathy Map", include_str!("../../assets/examples/empathy_map.spec")),
                    ("Value Prop", include_str!("../../assets/examples/value_proposition.spec")),
                    ("Fishbone", include_str!("../../assets/examples/fishbone.spec")),
                    ("PESTLE", include_str!("../../assets/examples/pestle.spec")),
                    ("Mind Map", include_str!("../../assets/examples/mind_map.spec")),
                    ("Premortem", include_str!("../../assets/examples/premortem.spec")),
                    ("Rose·Bud·Thorn", include_str!("../../assets/examples/rose_bud_thorn.spec")),
                    ("Double Diamond", include_str!("../../assets/examples/double_diamond.spec")),
                    ("Assumption Map", include_str!("../../assets/examples/assumption_map.spec")),
                    ("Biz Model Canvas", include_str!("../../assets/examples/business_model_canvas.spec")),
                    ("Hypothesis Canvas", include_str!("../../assets/examples/hypothesis_canvas.spec")),
                    ("Story Map", include_str!("../../assets/examples/story_map.spec")),
                    ("ICE Scoring", include_str!("../../assets/examples/ice_scoring.spec")),
                    ("Jobs To Be Done", include_str!("../../assets/examples/jobs_to_be_done.spec")),
                    ("Causal Loop", include_str!("../../assets/examples/causal_loop.spec")),
                    ("Experiment Board", include_str!("../../assets/examples/experiment_board.spec")),
                    ("Theory of Change", include_str!("../../assets/examples/theory_of_change.spec")),
                    ("Competitive Analysis", include_str!("../../assets/examples/competitive_analysis.spec")),
                    ("What?SoWhat?NowWhat?", include_str!("../../assets/examples/what_so_what.spec")),
                    ("2×2 Matrix", include_str!("../../assets/examples/two_by_two_matrix.spec")),
                    ("Design Sprint", include_str!("../../assets/examples/design_sprint.spec")),
                    ("Problem/Solution Fit", include_str!("../../assets/examples/problem_solution_fit.spec")),
                ];
                let support_specs: &[(&str, &str)] = &[
                    ("Ticket Flow", include_str!("../../assets/examples/support_ticket_flow.spec")),
                    ("Incident Runbook", include_str!("../../assets/examples/incident_response.spec")),
                    ("Escalation Matrix", include_str!("../../assets/examples/support_escalation_matrix.spec")),
                    ("Bug Triage", include_str!("../../assets/examples/bug_triage.spec")),
                    ("KB Structure", include_str!("../../assets/examples/knowledge_base_structure.spec")),
                    ("Voice of Customer", include_str!("../../assets/examples/voice_of_customer.spec")),
                    ("Customer Onboarding", include_str!("../../assets/examples/customer_onboarding.spec")),
                    ("Health Dashboard", include_str!("../../assets/examples/support_health_dashboard.spec")),
                    ("Postmortem", include_str!("../../assets/examples/postmortem.spec")),
                    ("SLA Matrix", include_str!("../../assets/examples/support_sla_matrix.spec")),
                ];
                ui.label(egui::RichText::new("Architecture:").size(9.0).color(self.theme.text_dim));
                ui.add_space(1.0);
                ui.horizontal_wrapped(|ui| {
                    for (name, spec_text) in arch_specs {
                        if ui.add(egui::Button::new(
                            egui::RichText::new(*name).size(10.0).color(self.theme.accent)
                        ).fill(self.theme.surface0)).clicked() {
                            self.spec_paste_buf = spec_text.to_string();
                            self.show_spec_paste_area = true;
                        }
                    }
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Design Thinking:").size(9.0).color(self.theme.text_dim));
                ui.add_space(1.0);
                ui.horizontal_wrapped(|ui| {
                    for (name, spec_text) in design_specs {
                        if ui.add(egui::Button::new(
                            egui::RichText::new(*name).size(10.0).color(self.theme.accent)
                        ).fill(self.theme.surface0)).clicked() {
                            self.spec_paste_buf = spec_text.to_string();
                            self.show_spec_paste_area = true;
                        }
                    }
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Support Ops:").size(9.0).color(self.theme.text_dim));
                ui.add_space(1.0);
                ui.horizontal_wrapped(|ui| {
                    for (name, spec_text) in support_specs {
                        if ui.add(egui::Button::new(
                            egui::RichText::new(*name).size(10.0).color(self.theme.accent)
                        ).fill(self.theme.surface0)).clicked() {
                            self.spec_paste_buf = spec_text.to_string();
                            self.show_spec_paste_area = true;
                        }
                    }
                });
                ui.add_space(2.0);
                // Spec cheatsheet toggle
                if ui
                    .add_sized(
                        egui::vec2(ui.available_width(), 24.0),
                        egui::Button::new(
                            egui::RichText::new(if self.show_spec_cheatsheet {
                                "▼ Spec Reference"
                            } else {
                                "▶ Spec Reference"
                            })
                            .size(11.0)
                            .color(self.theme.text_dim),
                        )
                        .fill(Color32::TRANSPARENT),
                    )
                    .on_hover_text("Show HRF spec syntax reference")
                    .clicked()
                {
                    self.show_spec_cheatsheet = !self.show_spec_cheatsheet;
                }
                if self.show_spec_cheatsheet {
                    ui.add_space(2.0);
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        let entries: &[(&str, &str)] = &[
                            ("Sections", "## Nodes / ## Flow / ## Notes / ## Groups / ## Steps / ## Config / ## Hypotheses"),
                            ("## Config", "bg = dots | snap = true | grid-size = 20 | zoom = 1.5 | flow = LR"),
                            ("## Palette", "brand = #1e3a5f  →  use {fill:brand} anywhere"),
                            ("Inline edges", "- [api] Service → db, cache {dashed}"),
                            ("## Steps", "1. Step label {diamond}  (sequential flowchart)"),
                            ("3D layers", "## Layer 0: Database / ## Layer 1: Backend"),
                            ("3D layers", "{z:N} (px) / {layer:N} (N×120px) / {3d-depth:80}"),
                            ("Comments", "// this line is ignored"),
                            ("Shapes", "{diamond} {circle} {parallelogram} {hexagon} {connector} {triangle} {callout}"),
                            ("Semantic presets", "{server} {database} {cloud} {user} {service}"),
                            ("Semantic presets", "{queue} {cache} {internet} {decision} {start} {end}"),
                            ("Design thinking", "{hypothesis} {assumption} {evidence} {conclusion}"),
                            ("Design thinking", "{question} {cause} {effect} {idea} {risk} {goal}"),
                            ("Design thinking", "{experiment} {metric} {strength} {weakness} {opportunity}"),
                            ("Design keys", "H = hypothesis · Y = assumption · W = evidence  (quick-create)"),
                            ("Design sections", "## Hypotheses / ## Assumptions / ## Evidence / ## Questions"),
                            ("Design sections", "## Ideas / ## Causes / ## Effects / ## Goals / ## Risks"),
                            ("Status tags", "{critical} {warning} {ok} {info}  — badge only"),
                            ("Status+progress", "{done} {wip} {review} {blocked} {todo}  — badge+progress"),
                            ("Support priority", "{p1} {p2} {p3} {p4}  — badge+fill  ·  {escalated}  — Critical+glow  ·  {urgent}  — P1+red+glow"),
                            ("Support severity", "{sev1} {sev2} {sev3}  — same as p1/p2/p3 (SEV naming)"),
                            ("Support owner", "{assigned:Alice}  /  {owner:Bob}  →  sublabel with 👤 prefix"),
                            ("Support deadline", "{due:2026-03-20}  /  {deadline:Q2}  →  sublabel with 📅 prefix"),
                            ("Node glow", "{glow} / {neon}  — neon border halo on node"),
                            ("Color", "{fill:blue/green/red/yellow/purple/teal/orange/sky/lavender/gray/none}"),
                            ("Color", "{fill:#rrggbb} {border-color:red} {text-color:white}"),
                            ("Style", "{bold} {italic} {shadow} {gradient} {dashed-border} {highlight}"),
                            ("Opacity", "{dim} {ghost} {muted} {hidden} / {opacity:50} (50%)"),
                            ("Size", "{size:200x80} or {w:200} {h:100} {r:8} {border:2}"),
                            ("Position", "{pos:100,200} (pinned) or {x:100} {y:200} {pinned}"),
                            ("Align", "{align:left/right} {valign:top/bottom}"),
                            ("Icon/Frame", "{icon:🔒} {frame}"),
                            ("Tooltip", "{tooltip:description text}  (or indent lines below)"),
                            ("Sublabel", "{sublabel:v2 · running}  (small text below node)"),
                            ("Progress", "{progress:75}  (0–100% completion bar at node bottom)"),
                            ("Collapsed", "{collapsed}  (render as compact pill, shows only label)"),
                            ("Node note", "{note:deprecated — use /v2 instead}  (💬 tooltip on hover)"),
                            ("Inline group", "{group:backend}  (auto bounding frame)"),
                            ("Edge note", "a -> b {note:this path is deprecated}"),
                            ("Special", "{entity} {text} {locked} {url:https://...}"),
                            ("Edge flow", "{dashed} {glow} {animated} {thick} {ortho}"),
                            ("Edge style", "{arrow:open/circle/none} {bend:0.3} {weight:2}"),
                            ("Edge color", "{color:blue/#rrggbb} {from:label} {to:label}"),
                            ("Cardinality", "{c-src:1} {c-tgt:0..N}"),
                            ("Ports", "{src-port:top/l/r/bottom} {tgt-port:...}"),
                            ("Arrow aliases", "-> (short) / <-- (reverse) / <-> (bidir)"),
                            ("Style arrows", "-.-> (dashed) / ==> (thick) / ~~> (animated)"),
                            ("Unicode arrows", "a → b  a ⇒ b  a ↔ b  (same as -->/→/<->)"),
                            ("Edge label", "a -> b: label text  {dashed}  (suffix syntax)"),
                            ("Inline comment", "a -> b  // this comment is ignored by parser"),
                            ("Multi-target", "a -> [b, c, d] {tags}  (shared-style fan-out)"),
                            ("Multi-source", "[a, b, c] -> target {tags}  (fan-in)"),
                            ("## Style", "primary = {fill:blue} {highlight}  →  use {primary}"),
                            ("Quick cards", "⌘⇧E = Experiment card  ·  ⌘⇧T = Support ticket card"),
                            ("Support report", "⌘⇧R = copy status report as Markdown (selection or all)"),
                            ("Due shortcuts", "⇧D = set due today  ·  ⇧W = set due next week (+7d)"),
                            ("Inline section", "{section:Intake} {stage:Triage} {col:Done}  (set kanban column inline)"),
                            ("CSV export", "⌘⇧X = copy tickets as CSV (selection or all nodes)"),
                            ("3D cam keys", "1=Iso  2=Top  3=Front  4=Side  (in 3D mode)"),
                            ("3D cam cfg", "camera = iso|top|front|side  (## Config)"),
                            ("3D auto-z", "auto-z = true  (auto-assign z from topology)"),
                            ("3D named tier", "{layer:db}  {layer:api}  {layer:frontend}  {layer:edge}"),
                            ("3D tier-color", "{tier-color} / auto-tier-color = true  (tier fill tints)"),
                            ("Grid layout", "## Grid cols=3  (auto grid layout, 3 columns)"),
                            ("Label refs", "REST API --> Database  or  \"REST API\" --> \"Database\""),
                            ("Layout gap", "spacing = 120  (## Config, sets node gap in px)"),
                            ("Timeline", "timeline = true  (## Config, activates roadmap mode)"),
                            ("Timeline dir", "timeline-dir = LR  (default) or TB"),
                            ("Timeline period", "## Period 1: Q1 — Foundation  (ordered time block)"),
                            ("Timeline lane", "{lane:Backend}  (assign node to swim-lane)"),
                            ("Timeline lane decl", "## Lane 1: Backend  (declare lane order)"),
                        ];
                        for (section, tags) in entries {
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(*section).size(9.5).color(self.theme.text_secondary));
                            ui.label(egui::RichText::new(*tags).size(9.0).color(self.theme.text_dim).monospace());
                        }
                    });
                }
                ui.add_space(2.0);
                // LLM Settings button
                if ui
                    .add_sized(
                        egui::vec2(ui.available_width(), 26.0),
                        egui::Button::new(
                            egui::RichText::new(if self.llm_config.api_key.is_empty() {
                                "LLM Settings (not configured)"
                            } else {
                                "LLM Settings"
                            })
                            .size(11.5)
                            .color(self.theme.text_dim),
                        )
                        .fill(Color32::TRANSPARENT),
                    )
                    .clicked()
                {
                    self.show_llm_settings = !self.show_llm_settings;
                }

                if self.show_llm_settings {
                    ui.add_space(4.0);
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new("Endpoint:").size(11.5).color(self.theme.text_dim));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.llm_config.endpoint)
                                .desired_width(ui.available_width())
                                .font(FontId::monospace(11.5)),
                        );
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new("API Key:").size(11.5).color(self.theme.text_dim));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.llm_config.api_key)
                                .desired_width(ui.available_width())
                                .password(true)
                                .font(FontId::monospace(11.5)),
                        );
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new("Model:").size(11.5).color(self.theme.text_dim));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.llm_config.model)
                                .desired_width(ui.available_width())
                                .font(FontId::monospace(11.5)),
                        );
                    });
                }
                ui.add_space(8.0);

                // Tools
                self.draw_divider(ui);
                ui.add_space(8.0);
                self.draw_section_header(ui, "Tools");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let select_text = if self.tool == Tool::Select {
                        egui::RichText::new("Select").size(12.0).strong().color(self.theme.accent)
                    } else {
                        egui::RichText::new("Select").size(12.0).color(self.theme.text_secondary)
                    };
                    let connect_text = if self.tool == Tool::Connect {
                        egui::RichText::new("Connect").size(12.0).strong().color(self.theme.accent)
                    } else {
                        egui::RichText::new("Connect").size(12.0).color(self.theme.text_secondary)
                    };
                    let btn_size = egui::vec2(84.0, 32.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(select_text)
                                .fill(if self.tool == Tool::Select { self.theme.surface1 } else { self.theme.surface0 }),
                        )
                        .clicked()
                    {
                        self.tool = Tool::Select;
                    }
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(connect_text)
                                .fill(if self.tool == Tool::Connect { self.theme.surface1 } else { self.theme.surface0 }),
                        )
                        .clicked()
                    {
                        self.tool = Tool::Connect;
                    }
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("V  Select   ·   E  Connect")
                        .size(11.0)
                        .color(self.theme.text_dim),
                );
                ui.add_space(8.0);

                // Mode tabs
                self.draw_divider(ui);
                ui.add_space(8.0);
                self.draw_section_header(ui, "Mode");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let modes = [
                        (DiagramMode::Flowchart, "Flow"),
                        (DiagramMode::ER, "ER"),
                        (DiagramMode::FigJam, "FigJam"),
                    ];
                    for (mode, label) in modes {
                        let is_active = self.diagram_mode == mode;
                        let text = if is_active {
                            egui::RichText::new(label).size(11.0).strong().color(self.theme.accent)
                        } else {
                            egui::RichText::new(label).size(11.0).color(self.theme.text_secondary)
                        };
                        if ui
                            .add(
                                egui::Button::new(text)
                                    .fill(if is_active { self.theme.surface1 } else { self.theme.surface0 }),
                            )
                            .clicked()
                        {
                            self.diagram_mode = mode;
                        }
                    }
                });
                ui.add_space(8.0);

                // Shapes (mode-dependent)
                self.draw_divider(ui);
                ui.add_space(8.0);
                self.draw_section_header(ui, "Shapes");
                ui.add_space(6.0);

                match self.diagram_mode {
                    DiagramMode::Flowchart => {
                        self.draw_flowchart_shapes(ui, ctx);
                    }
                    DiagramMode::ER => {
                        let available_width = ui.available_width();
                        let er_resp = ui
                            .add_sized(
                                egui::vec2(available_width, 40.0),
                                egui::Button::new(
                                    egui::RichText::new("+ Entity").size(13.0).color(self.theme.text_primary),
                                )
                                .fill(self.theme.surface0),
                            )
                            .on_hover_text("Click or drag onto canvas");
                        if er_resp.clicked() {
                            let center_screen = self.canvas_rect.center();
                            let center_canvas = self.viewport.screen_to_canvas(center_screen);
                            let node = Node::new_entity(center_canvas);
                            self.selection.clear();
                            self.selection.node_ids.insert(node.id);
                            self.document.nodes.push(node);
                            self.history.push(&self.document);
                        }
                        if er_resp.drag_started() {
                            if let Some(pos) = er_resp.interact_pointer_pos() {
                                self.drag = DragState::DraggingNewNode {
                                    kind: NodeKind::Entity { name: "Entity".into(), attributes: vec![] },
                                    current_screen: pos,
                                };
                            }
                        }
                    }
                    DiagramMode::FigJam => {
                        self.draw_figjam_shapes(ui, ctx);
                    }
                }

                ui.add_space(8.0);

                // View
                self.draw_divider(ui);
                ui.add_space(8.0);
                self.draw_section_header(ui, "View");
                ui.add_space(4.0);

                // 2D/3D toggle
                ui.horizontal(|ui| {
                    let is_2d = self.view_mode == super::ViewMode::TwoD;
                    let is_3d = self.view_mode == super::ViewMode::ThreeD;
                    let btn_size = egui::vec2(84.0, 30.0);
                    let text_2d = if is_2d {
                        egui::RichText::new("2D").size(12.0).strong().color(self.theme.accent)
                    } else {
                        egui::RichText::new("2D").size(12.0).color(self.theme.text_secondary)
                    };
                    let text_3d = if is_3d {
                        egui::RichText::new("3D").size(12.0).strong().color(self.theme.accent)
                    } else {
                        egui::RichText::new("3D").size(12.0).color(self.theme.text_secondary)
                    };
                    if ui.add_sized(btn_size, egui::Button::new(text_2d)
                        .fill(if is_2d { self.theme.surface1 } else { self.theme.surface0 })
                    ).clicked() && !is_2d {
                        self.sync_viewport_to_camera();
                        self.view_mode = super::ViewMode::TwoD;
                        self.view_transition_target = 0.0;
                        self.status_message = Some(("2D View".to_string(), std::time::Instant::now()));
                    }
                    if ui.add_sized(btn_size, egui::Button::new(text_3d)
                        .fill(if is_3d { self.theme.surface1 } else { self.theme.surface0 })
                    ).clicked() && !is_3d {
                        self.sync_camera_to_viewport();
                        self.view_mode = super::ViewMode::ThreeD;
                        self.view_transition_target = 1.0;
                        self.status_message = Some(("3D View".to_string(), std::time::Instant::now()));
                    }
                });
                ui.add_space(4.0);

                // Camera preset buttons (3D mode only)
                if self.view_mode == super::ViewMode::ThreeD {
                    ui.horizontal(|ui| {
                        // Each preset: (label, yaw, pitch)
                        let presets: &[(&str, f32, f32, &str)] = &[
                            ("Iso",  -0.6,  0.5, "Isometric — classic 3D view"),
                            ("Top",   0.0,  1.55, "Top-down overhead"),
                            ("Front", 0.0,  0.05, "Front elevation"),
                            ("Side",  1.57, 0.05, "Right side elevation"),
                        ];
                        let btn_w = 40.0_f32;
                        let btn_h = 22.0_f32;
                        for (label, yaw, pitch, hint) in presets {
                            let active = (self.camera3d.yaw - yaw).abs() < 0.08
                                      && (self.camera3d.pitch - pitch).abs() < 0.08;
                            let text = if active {
                                egui::RichText::new(*label).size(10.5).strong().color(self.theme.accent)
                            } else {
                                egui::RichText::new(*label).size(10.5).color(self.theme.text_secondary)
                            };
                            if ui.add_sized([btn_w, btn_h], egui::Button::new(text)
                                .fill(if active { self.theme.surface1 } else { self.theme.surface0 })
                            ).on_hover_text(*hint).clicked() {
                                self.camera3d.yaw = *yaw;
                                self.camera3d.pitch = *pitch;
                                self.status_message = Some((
                                    format!("Camera: {} view", label),
                                    std::time::Instant::now(),
                                ));
                            }
                        }
                    });
                    ui.add_space(2.0);
                }

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_grid, "");
                    ui.label(egui::RichText::new("Grid").size(12.0).color(self.theme.text_secondary));
                    ui.add_space(12.0);
                    ui.checkbox(&mut self.snap_to_grid, "");
                    ui.label(egui::RichText::new("Snap").size(12.0).color(self.theme.text_secondary));
                });
                ui.add(egui::Slider::new(&mut self.grid_size, 10.0_f32..=80.0).text("Grid px").integer());
                ui.add_space(4.0);
                // Canvas background color picker
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Canvas").size(11.0).color(self.theme.text_dim));
                    let mut bg = egui::Color32::from_rgba_unmultiplied(
                        self.canvas_bg[0], self.canvas_bg[1], self.canvas_bg[2], self.canvas_bg[3],
                    );
                    if ui.color_edit_button_srgba(&mut bg).changed() {
                        self.canvas_bg = bg.to_array();
                    }
                    // Preset swatches
                    for (color, name) in CANVAS_BG_PRESETS {
                        let c = to_color32(*color);
                        if ui.add(egui::Button::new("  ").fill(c).min_size(egui::Vec2::new(14.0, 14.0)))
                            .on_hover_text(*name).clicked() {
                            self.canvas_bg = *color;
                        }
                    }
                });
                ui.add_space(8.0);

                // Zoom
                ui.horizontal(|ui| {
                    let zoom_label = egui::RichText::new(format!("{:.0}%", self.effective_zoom() * 100.0))
                        .size(12.0)
                        .color(self.theme.text_dim)
                        .monospace();
                    let zoom_resp = ui.add(egui::Label::new(zoom_label).sense(egui::Sense::click()))
                        .on_hover_text("Click for zoom presets");
                    if zoom_resp.clicked() {
                        ui.ctx().memory_mut(|m| m.toggle_popup(egui::Id::new("zoom_presets_popup")));
                    }
                    egui::popup_below_widget(ui, egui::Id::new("zoom_presets_popup"), &zoom_resp, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
                        ui.set_min_width(110.0);
                        for (label, zoom_val) in [("25%", 0.25), ("50%", 0.5), ("75%", 0.75), ("100%", 1.0), ("150%", 1.5), ("200%", 2.0)] {
                            let is_current = (self.viewport.zoom - zoom_val).abs() < 0.01;
                            let txt = egui::RichText::new(label).size(12.0);
                            let txt = if is_current { txt.strong() } else { txt };
                            if ui.selectable_label(is_current, txt).clicked() {
                                self.viewport.zoom = zoom_val;
                                ui.ctx().memory_mut(|m| m.close_popup());
                            }
                        }
                        ui.separator();
                        if ui.selectable_label(false, egui::RichText::new("Fit All (F)").size(12.0)).clicked() {
                            self.fit_to_content();
                            ui.ctx().memory_mut(|m| m.close_popup());
                        }
                    });
                });

                // Canvas state indicators
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if self.canvas_locked {
                        ui.label(egui::RichText::new("🔒 Locked").size(11.5).color(self.theme.text_dim));
                    }
                    if self.focus_mode {
                        ui.label(egui::RichText::new("🎯 Focus").size(11.5).color(self.theme.text_dim));
                    }
                });

                // Node count + quick actions
                ui.add_space(8.0);
                self.draw_divider(ui);
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "{} nodes  ·  {} edges",
                        self.document.nodes.len(),
                        self.document.edges.len()
                    ))
                    .size(11.0)
                    .color(self.theme.text_dim),
                );
                ui.add_space(4.0);
                if ui.small_button("Select All").clicked() {
                    self.selection.clear();
                    for n in &self.document.nodes { self.selection.node_ids.insert(n.id); }
                    for e in &self.document.edges { self.selection.edge_ids.insert(e.id); }
                }
                ui.add_space(16.0);

                        }); // end with_layout (top_down)
                    }); // end ScrollArea
            }); // end SidePanel::show
    }

    fn draw_flowchart_shapes(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let shapes = [
            (NodeShape::Rectangle, "Rectangle"),
            (NodeShape::RoundedRect, "Rounded"),
            (NodeShape::Diamond, "Diamond"),
            (NodeShape::Circle, "Circle"),
            (NodeShape::Parallelogram, "Parallel"),
            (NodeShape::Hexagon, "Hexagon"),
            (NodeShape::Connector, "Connector"),
            (NodeShape::Callout, "Callout"),
        ];

        let available_width = ui.available_width();
        let btn_width = (available_width - 10.0) / 2.0;
        let btn_height = 56.0;

        let mut i = 0;
        while i < shapes.len() {
            ui.horizontal(|ui| {
                for j in 0..2 {
                    if i + j < shapes.len() {
                        let (shape, name) = shapes[i + j];
                        let response = self.draw_shape_button(ui, shape, name, btn_width, btn_height);
                        if response.clicked() {
                            let center_screen = self.canvas_rect.center();
                            let center_canvas = self.viewport.screen_to_canvas(center_screen);
                            let node = Node::new(shape, center_canvas);
                            self.selection.clear();
                            self.selection.node_ids.insert(node.id);
                            self.document.nodes.push(node);
                            self.history.push(&self.document);
                        }
                        if response.drag_started() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                self.drag = DragState::DraggingNewNode {
                                    kind: NodeKind::Shape {
                                        shape,
                                        label: "New Node".into(),
                                        description: String::new(),
                                    },
                                    current_screen: pos,
                                };
                            }
                        }
                        if response.dragged() {
                            if let DragState::DraggingNewNode {
                                ref mut current_screen,
                                ..
                            } = self.drag
                            {
                                if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                                    *current_screen = pos;
                                }
                            }
                        }
                    }
                }
            });
            i += 2;
        }
    }

    fn draw_figjam_shapes(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let available_width = ui.available_width();
        // Sticky note button
        let sticky_resp = ui
            .add_sized(
                egui::vec2(available_width, 40.0),
                egui::Button::new(
                    egui::RichText::new("+ Sticky Note").size(13.0).color(self.theme.text_primary),
                )
                .fill(self.theme.surface0),
            )
            .on_hover_text("Click or drag onto canvas");
        if sticky_resp.clicked() {
            let center_screen = self.canvas_rect.center();
            let center_canvas = self.viewport.screen_to_canvas(center_screen);
            let node = Node::new_sticky(self.selected_sticky_color, center_canvas);
            self.selection.clear();
            self.selection.node_ids.insert(node.id);
            self.document.nodes.push(node);
            self.history.push(&self.document);
        }
        if sticky_resp.drag_started() {
            if let Some(pos) = sticky_resp.interact_pointer_pos() {
                self.drag = DragState::DraggingNewNode {
                    kind: NodeKind::StickyNote {
                        text: String::new(),
                        color: self.selected_sticky_color,
                    },
                    current_screen: pos,
                };
            }
        }
        ui.add_space(4.0);

        // Sticky color picker
        ui.horizontal(|ui| {
            for color in &StickyColor::ALL {
                let fill = to_color32(color.fill_rgba());
                let is_selected = self.selected_sticky_color == *color;
                let size = if is_selected { 22.0 } else { 18.0 };
                let (response, painter) =
                    ui.allocate_painter(egui::vec2(size, size), Sense::click());
                let r = response.rect;
                painter.circle_filled(r.center(), size / 2.0, fill);
                if is_selected {
                    painter.circle_stroke(r.center(), size / 2.0, Stroke::new(2.0, self.theme.text_primary));
                }
                if response.clicked() {
                    self.selected_sticky_color = *color;
                }
            }
        });
        ui.add_space(8.0);

        // Text node button
        let text_resp = ui
            .add_sized(
                egui::vec2(available_width, 36.0),
                egui::Button::new(
                    egui::RichText::new("+ Text").size(13.0).color(self.theme.text_primary),
                )
                .fill(self.theme.surface0),
            )
            .on_hover_text("Click or drag onto canvas");
        if text_resp.clicked() {
            let center_screen = self.canvas_rect.center();
            let center_canvas = self.viewport.screen_to_canvas(center_screen);
            let node = Node::new_text(center_canvas);
            self.selection.clear();
            self.selection.node_ids.insert(node.id);
            self.document.nodes.push(node);
            self.history.push(&self.document);
        }
        if text_resp.drag_started() {
            if let Some(pos) = text_resp.interact_pointer_pos() {
                self.drag = DragState::DraggingNewNode {
                    kind: NodeKind::Text { content: String::new() },
                    current_screen: pos,
                };
            }
        }

        // Frame node button
        ui.add_space(4.0);
        if ui
            .add_sized(
                egui::vec2(available_width, 36.0),
                egui::Button::new(
                    egui::RichText::new("⬜ Frame").size(13.0).color(self.theme.text_primary),
                )
                .fill(self.theme.surface0),
            )
            .on_hover_text("Group frame — translucent container behind other nodes (F key)")
            .clicked()
        {
            let center_screen = self.canvas_rect.center();
            let center_canvas = self.viewport.screen_to_canvas(center_screen);
            let node = Node::new_frame(center_canvas - egui::Vec2::new(150.0, 110.0));
            self.selection.clear();
            self.selection.node_ids.insert(node.id);
            self.document.nodes.push(node);
            self.history.push(&self.document);
        }
    }

    fn draw_shape_button(
        &self,
        ui: &mut egui::Ui,
        shape: NodeShape,
        name: &str,
        width: f32,
        height: f32,
    ) -> egui::Response {
        let (response, painter) =
            ui.allocate_painter(egui::vec2(width, height), Sense::click_and_drag());
        let rect = response.rect;

        let bg = if response.hovered() { self.theme.surface1 } else { self.theme.surface0 };
        painter.rect_filled(rect, CornerRadius::same(6), bg);
        if response.hovered() {
            painter.rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.0, self.theme.accent),
                StrokeKind::Inside,
            );
        }

        let preview_center = Pos2::new(rect.center().x, rect.min.y + height * 0.38);
        let pw = width * 0.35;
        let ph = height * 0.32;
        let shape_stroke = Stroke::new(1.5, self.theme.accent);
        let shape_fill = self.theme.accent_faint;

        match shape {
            NodeShape::Rectangle => {
                let r = egui::Rect::from_center_size(preview_center, egui::vec2(pw, ph));
                painter.rect_filled(r, CornerRadius::ZERO, shape_fill);
                painter.rect_stroke(r, CornerRadius::ZERO, shape_stroke, StrokeKind::Outside);
            }
            NodeShape::RoundedRect => {
                let r = egui::Rect::from_center_size(preview_center, egui::vec2(pw, ph));
                painter.rect_filled(r, CornerRadius::same(4), shape_fill);
                painter.rect_stroke(r, CornerRadius::same(4), shape_stroke, StrokeKind::Outside);
            }
            NodeShape::Diamond => {
                let c = preview_center;
                let hw = pw * 0.5;
                let hh = ph * 0.5;
                let pts = vec![
                    Pos2::new(c.x, c.y - hh),
                    Pos2::new(c.x + hw, c.y),
                    Pos2::new(c.x, c.y + hh),
                    Pos2::new(c.x - hw, c.y),
                ];
                painter.add(egui::Shape::convex_polygon(pts, shape_fill, shape_stroke));
            }
            NodeShape::Circle => {
                let r = pw.min(ph) * 0.5;
                painter.circle_filled(preview_center, r, shape_fill);
                painter.circle_stroke(preview_center, r, shape_stroke);
            }
            NodeShape::Parallelogram => {
                let skew = pw * 0.2;
                let half_w = pw * 0.5;
                let half_h = ph * 0.5;
                let pts = vec![
                    Pos2::new(preview_center.x - half_w + skew, preview_center.y - half_h),
                    Pos2::new(preview_center.x + half_w, preview_center.y - half_h),
                    Pos2::new(preview_center.x + half_w - skew, preview_center.y + half_h),
                    Pos2::new(preview_center.x - half_w, preview_center.y + half_h),
                ];
                painter.add(egui::Shape::convex_polygon(pts, shape_fill, shape_stroke));
            }
            NodeShape::Hexagon => {
                let hw = pw * 0.48;
                let hh = ph * 0.40;
                let inset = hw * 0.3;
                let pts = vec![
                    Pos2::new(preview_center.x - hw,    preview_center.y),
                    Pos2::new(preview_center.x - inset, preview_center.y - hh),
                    Pos2::new(preview_center.x + inset, preview_center.y - hh),
                    Pos2::new(preview_center.x + hw,    preview_center.y),
                    Pos2::new(preview_center.x + inset, preview_center.y + hh),
                    Pos2::new(preview_center.x - inset, preview_center.y + hh),
                ];
                painter.add(egui::Shape::convex_polygon(pts, shape_fill, shape_stroke));
            }
            NodeShape::Connector => {
                // Pill preview
                let pill_w = pw * 0.9;
                let pill_h = ph * 0.45;
                let pill_rect = egui::Rect::from_center_size(
                    preview_center,
                    egui::vec2(pill_w, pill_h),
                );
                let radius = pill_h / 2.0;
                painter.rect_filled(pill_rect, CornerRadius::same(radius as u8), shape_fill);
                painter.rect_stroke(pill_rect, CornerRadius::same(radius as u8), shape_stroke, StrokeKind::Outside);
            }
            NodeShape::Triangle => {
                let hw = pw * 0.5;
                let hh = ph * 0.5;
                let pts = vec![
                    Pos2::new(preview_center.x,       preview_center.y - hh),
                    Pos2::new(preview_center.x + hw,  preview_center.y + hh),
                    Pos2::new(preview_center.x - hw,  preview_center.y + hh),
                ];
                painter.add(egui::Shape::convex_polygon(pts, shape_fill, shape_stroke));
            }
            NodeShape::Callout => {
                let r = egui::Rect::from_center_size(preview_center, egui::Vec2::new(pw, ph * 0.75));
                painter.rect(r, egui::CornerRadius::same(4), shape_fill, shape_stroke, egui::StrokeKind::Middle);
                let tail = vec![
                    Pos2::new(r.min.x + 2.0, r.max.y),
                    Pos2::new(r.min.x + pw * 0.35, r.max.y),
                    Pos2::new(r.min.x - pw * 0.05, r.max.y + ph * 0.3),
                ];
                painter.add(egui::Shape::convex_polygon(tail, shape_fill, shape_stroke));
            }
        }

        painter.text(
            Pos2::new(rect.center().x, rect.max.y - 10.0),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(10.0),
            if response.hovered() {
                self.theme.text_primary
            } else {
                self.theme.text_secondary
            },
        );

        response
    }
}
