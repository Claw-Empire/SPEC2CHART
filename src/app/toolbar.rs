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
                                Ok(specgraph::export_hrf(&self.document, "Untitled Diagram"))
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
                            ("Node shapes", "{diamond} {circle} {parallelogram} {hexagon} {connector}"),
                            ("3D layers", "## Layer 0 / ## Layer 1 / {z:N}"),
                            ("Tags", "{critical} {warning} {ok} {info}"),
                            ("Style", "{fill:blue} {bold} {italic} {shadow} {dashed-border}"),
                            ("Size", "{w:200} {h:100} {r:8} {border:2}"),
                            ("Align", "{align:left/right} {valign:top/bottom}"),
                            ("Position", "{pinned} {x:100} {y:200}"),
                            ("Icon", "{icon:🔒}"),
                            ("Special", "{entity} {text}"),
                            ("Edge", "{dashed} {glow} {animated} {thick} {ortho}"),
                            ("Edge", "{arrow:open/circle/none} {bend:0.3}"),
                            ("Edge", "{color:blue} {from:label} {to:label}"),
                            ("Cardinality", "{c-src:1} {c-tgt:0..N}"),
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
