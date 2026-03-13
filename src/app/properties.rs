use egui::{Color32, CornerRadius, FontId, Pos2, Rect, SidePanel, Stroke};
use crate::model::*;
use super::FlowchartApp;
use super::theme::{PROPERTIES_WIDTH, to_color32};

impl FlowchartApp {
    pub(crate) fn draw_properties_panel(&mut self, ctx: &egui::Context) {
        // Collapsed: show a thin strip with just an expand button
        if self.properties_collapsed {
            SidePanel::right("properties")
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
                            egui::RichText::new("◀").size(11.0).color(self.theme.text_dim)
                        ).fill(egui::Color32::TRANSPARENT).frame(false);
                        if ui.add(btn).on_hover_text("Expand properties").clicked() {
                            self.properties_collapsed = false;
                        }
                    });
                });
            return;
        }

        SidePanel::right("properties")
            .resizable(false)
            .exact_width(PROPERTIES_WIDTH)
            .frame(egui::Frame {
                fill: self.theme.mantle,
                inner_margin: egui::Margin { left: 6, right: 10, top: 10, bottom: 8 },
                stroke: Stroke::new(1.0, self.theme.surface1),
                ..Default::default()
            })
            .show(ctx, |ui| {
                // Header with collapse button
                ui.horizontal(|ui| {
                    let btn = egui::Button::new(
                        egui::RichText::new("▶").size(10.0).color(self.theme.text_dim)
                    ).fill(egui::Color32::TRANSPARENT).frame(false);
                    if ui.add(btn).on_hover_text("Collapse properties").clicked() {
                        self.properties_collapsed = true;
                    }
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Properties")
                            .size(14.0)
                            .color(self.theme.text_primary)
                            .strong(),
                    );
                });
                ui.add_space(8.0);

                let sel_nodes = self.selection.node_ids.len();
                let sel_edges = self.selection.edge_ids.len();
                let total = sel_nodes + sel_edges;

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min).with_cross_justify(true), |ui| {
                            if total == 0 {
                                self.draw_empty_selection(ui);
                            } else if total > 1 {
                                self.draw_multi_selection_tools(ui, sel_nodes, total);
                            } else if sel_nodes == 1 {
                                self.draw_node_properties(ui);
                            } else if sel_edges == 1 {
                                self.draw_edge_properties(ui);
                            }
                        });
                    });
            });
    }

    fn draw_empty_selection(&self, ui: &mut egui::Ui) {
        let n_nodes = self.document.nodes.len();
        let n_edges = self.document.edges.len();

        if n_nodes == 0 {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("\u{25CB}").size(32.0).color(self.theme.surface1));
                ui.add_space(12.0);
                ui.label(egui::RichText::new("Canvas is empty").size(15.0).color(self.theme.text_secondary));
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Double-click anywhere\nor press R, C, or D to add a shape").size(12.0).color(self.theme.text_dim));
            });
            return;
        }

        // Graph statistics
        ui.add_space(8.0);
        self.draw_section_header(ui, "Graph Overview");
        ui.add_space(6.0);

        let stats: &[(&str, String)] = &[
            ("Nodes", n_nodes.to_string()),
            ("Edges", n_edges.to_string()),
        ];
        for (label, val) in stats {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(*label).size(11.0).color(self.theme.text_dim));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(val).size(11.0).strong().color(self.theme.text_secondary));
                });
            });
        }
        ui.add_space(8.0);

        // Orphan detection
        let orphans: Vec<&str> = self.document.nodes.iter()
            .filter(|n| {
                !self.document.edges.iter().any(|e| e.source.node_id == n.id || e.target.node_id == n.id)
            })
            .filter_map(|n| match &n.kind {
                crate::model::NodeKind::Shape { label, .. } => Some(label.as_str()),
                crate::model::NodeKind::Entity { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .take(5)
            .collect();

        if !orphans.is_empty() {
            self.draw_section_header(ui, "Unconnected");
            ui.add_space(4.0);
            for name in &orphans {
                ui.label(egui::RichText::new(format!("· {}", name)).size(11.5).color(self.theme.text_dim));
            }
            if orphans.len() == 5 && self.document.nodes.iter().filter(|n| {
                !self.document.edges.iter().any(|e| e.source.node_id == n.id || e.target.node_id == n.id)
            }).count() > 5 {
                ui.label(egui::RichText::new("…and more").size(11.0).color(self.theme.text_dim));
            }
            ui.add_space(8.0);
        }

        // Tags summary
        let tagged: Vec<_> = self.document.nodes.iter()
            .filter(|n| n.tag.is_some())
            .collect();
        if !tagged.is_empty() {
            self.draw_section_header(ui, "Tags");
            ui.add_space(4.0);
            let mut counts = [0usize; 4];
            for n in &tagged {
                match n.tag {
                    Some(crate::model::NodeTag::Critical) => counts[0] += 1,
                    Some(crate::model::NodeTag::Warning)  => counts[1] += 1,
                    Some(crate::model::NodeTag::Ok)       => counts[2] += 1,
                    Some(crate::model::NodeTag::Info)     => counts[3] += 1,
                    None => {}
                }
            }
            let names = ["Critical", "Warning", "OK", "Info"];
            let colors = [
                egui::Color32::from_rgb(243, 139, 168),
                egui::Color32::from_rgb(249, 226, 175),
                egui::Color32::from_rgb(166, 227, 161),
                egui::Color32::from_rgb(137, 180, 250),
            ];
            for (i, (name, c)) in names.iter().zip(colors.iter()).enumerate() {
                if counts[i] > 0 {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("● {}", name)).size(11.5).color(*c));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(counts[i].to_string()).size(11.5).color(self.theme.text_dim));
                        });
                    });
                }
            }
            ui.add_space(8.0);
        }

        // History summary
        let undo_count = self.history.undo_steps();
        let redo_count = self.history.redo_steps();
        if undo_count > 0 || redo_count > 0 {
            self.draw_section_header(ui, "History");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("↩ {} undo", undo_count)).size(11.0).color(
                    if undo_count > 0 { self.theme.text_secondary } else { self.theme.text_dim }
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{} ↪ redo", redo_count)).size(11.0).color(
                        if redo_count > 0 { self.theme.text_secondary } else { self.theme.text_dim }
                    ));
                });
            });
            // Visual bar: undo steps on left (filled), redo on right (dim)
            let total = (undo_count + redo_count).max(1);
            let undo_frac = undo_count as f32 / total as f32;
            let bar_response = ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), 6.0), egui::Sense::hover());
            let bar = bar_response.0;
            let painter = ui.painter();
            painter.rect_filled(bar, CornerRadius::same(3), self.theme.surface0);
            painter.rect_filled(
                Rect::from_min_size(bar.min, egui::Vec2::new(bar.width() * undo_frac, bar.height())),
                CornerRadius::same(3),
                self.theme.accent.gamma_multiply(0.7),
            );
            ui.add_space(8.0);
        }

        ui.add_space(8.0);
        self.draw_divider(ui);
        ui.add_space(8.0);

        // Keyboard shortcuts reference
        self.draw_section_header(ui, "Shortcuts");
        ui.add_space(6.0);

        let shortcut_groups: &[(&str, &[(&str, &str)])] = &[
            ("Navigation", &[
                ("V / E", "Select / Connect"),
                ("Tab / ⇧Tab", "Cycle nodes"),
                ("F", "Fit to content / Pres"),
                ("⌘+ / ⌘-", "Zoom in / out"),
                ("⌘0", "Reset zoom"),
                ("H", "Heatmap overlay"),
                ("⇧A", "Flow animation"),
                ("↵ Enter", "Chain node →right"),
                ("⇧↵", "Chain node ↓down"),
                ("⌥ Alt+hover", "Distance rulers"),
                ("⌥ Alt+drag", "Duplicate in place"),
            ]),
            ("Editing", &[
                ("⌘Z / ⌘⇧Z", "Undo / Redo"),
                ("⌘D", "Duplicate"),
                ("⌘A", "Select all"),
                ("Del / ⌫", "Delete"),
                ("⌘C / ⌘V", "Copy / Paste"),
                ("⌘L", "Lock / unlock"),
            ]),
            ("Layout", &[
                ("⇧L", "Force-directed layout"),
                ("⇧H / ⇧V", "Distribute H / V"),
                ("⌘] / ⌘[", "Raise / lower Z"),
                ("⌘⇧] / ⌘⇧[", "Front / Back"),
                ("↑↓←→", "Navigate connected"),
            ]),
        ];

        for (group_name, shortcuts) in shortcut_groups {
            ui.label(egui::RichText::new(*group_name).size(11.5).color(self.theme.text_secondary).strong());
            ui.add_space(4.0);
            for (key, action) in *shortcuts {
                ui.horizontal(|ui| {
                    let key_text = egui::RichText::new(*key)
                        .size(11.0)
                        .monospace()
                        .color(self.theme.accent)
                        .background_color(self.theme.accent_faint);
                    ui.label(key_text);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(*action).size(11.0).color(self.theme.text_dim));
                    });
                });
            }
            ui.add_space(10.0);
        }
    }

    fn draw_node_properties(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        let node_id = *self.selection.node_ids.iter().next().unwrap();
        let mut applied_quick_style: Option<&'static str> = None;
        if let Some(node) = self.document.find_node_mut(&node_id) {
            let kind_name = match &node.kind {
                NodeKind::Shape { shape, .. } => match shape {
                    NodeShape::Rectangle => "Rectangle",
                    NodeShape::RoundedRect => "Rounded Rect",
                    NodeShape::Diamond => "Diamond",
                    NodeShape::Circle => "Circle",
                    NodeShape::Parallelogram => "Parallelogram",
                    NodeShape::Hexagon => "Hexagon",
                    NodeShape::Connector => "Connector",
                },
                NodeKind::StickyNote { .. } => "Sticky Note",
                NodeKind::Entity { .. } => "Entity",
                NodeKind::Text { .. } => "Text",
            };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(kind_name).size(13.0).strong().color(theme.accent));
            });
            ui.add_space(12.0);

            let mut needs_entity_resize = false;
            match &mut node.kind {
                NodeKind::Shape {
                    label, description, ..
                } => {
                    ui.label(egui::RichText::new("Content").size(11.0).color(theme.text_secondary).strong());
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Label").size(11.0).color(theme.text_dim));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("⎘").on_hover_text("Copy label to clipboard").clicked() {
                                ui.ctx().copy_text(label.clone());
                            }
                        });
                    });
                    ui.add_space(2.0);
                    let label_response = ui.add(
                        egui::TextEdit::singleline(label)
                            .desired_width(f32::INFINITY)
                            .font(FontId::proportional(13.0)),
                    );
                    if self.focus_label_edit {
                        label_response.request_focus();
                        self.focus_label_edit = false;
                    }
                    ui.add_space(4.0);
                    // URL field
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🔗").size(11.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut node.url)
                                .hint_text("https://…")
                                .desired_width(f32::INFINITY)
                                .font(FontId::monospace(11.0)),
                        );
                        if !node.url.is_empty() && ui.small_button("Open").clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(&node.url));
                        }
                    });
                    // Node icon badge field
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Icon").size(11.0).color(theme.text_dim));
                        ui.add(
                            egui::TextEdit::singleline(&mut node.icon)
                                .hint_text("emoji…")
                                .desired_width(42.0)
                                .font(FontId::proportional(13.0)),
                        );
                        if !node.icon.is_empty() && ui.small_button("✕").on_hover_text("Clear icon").clicked() {
                            node.icon.clear();
                        }
                    });
                    ui.add_space(2.0);
                    ui.horizontal_wrapped(|ui| {
                        for badge in ["📦","🔧","⚙️","🗄️","🌐","🔒","💡","📊","🚀","❌","✅","⚠️","🔑","🧩","📌","🎯","🔵","🟢","🟡","🔴"] {
                            if ui.small_button(badge).clicked() { node.icon = badge.to_string(); }
                        }
                    });
                    // Quick emoji prefix buttons
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        for emoji in ["📦","🔧","⚙️","🗄️","🌐","🔒","💡","📊","🚀","❌","✅","⚠️"] {
                            if ui.small_button(emoji).on_hover_text(format!("Prefix {emoji}")).clicked() {
                                let trimmed = label.trim_start_matches(|c: char| !c.is_alphanumeric() && c != ' ');
                                *label = format!("{emoji} {trimmed}");
                            }
                        }
                    });
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Description").size(11.0).color(theme.text_dim));
                    ui.add_space(2.0);
                    ui.add(
                        egui::TextEdit::multiline(description)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .font(FontId::proportional(12.0)),
                    );
                    // Sublabel (small secondary text below node)
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Sublabel").size(11.0).color(theme.text_dim));
                    ui.add_space(2.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut node.sublabel)
                            .desired_width(f32::INFINITY)
                            .hint_text("e.g. v2.1 · running")
                            .font(FontId::proportional(11.5)),
                    );
                    // Comment field (also accessible via Cmd+M)
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("💬 Comment").size(11.0).color(theme.text_dim));
                        if ui.small_button("Edit…").on_hover_text("Cmd+M").clicked() {
                            self.comment_editing = Some(node_id);
                        }
                    });
                    if !node.comment.is_empty() {
                        ui.add_space(2.0);
                        ui.add(
                            egui::TextEdit::multiline(&mut node.comment)
                                .desired_width(f32::INFINITY)
                                .desired_rows(2)
                                .font(FontId::proportional(11.0)),
                        );
                    }
                }
                NodeKind::StickyNote { text, color } => {
                    ui.label(egui::RichText::new("Content").size(11.0).color(theme.text_secondary).strong());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Text").size(11.0).color(theme.text_dim));
                    ui.add_space(2.0);
                    let text_response = ui.add(
                        egui::TextEdit::multiline(text)
                            .desired_width(f32::INFINITY)
                            .desired_rows(4)
                            .font(FontId::proportional(13.0)),
                    );
                    if self.focus_label_edit {
                        text_response.request_focus();
                        self.focus_label_edit = false;
                    }
                    ui.add_space(12.0);

                    ui.label(egui::RichText::new("Color").size(11.0).color(theme.text_secondary).strong());
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        for sc in &StickyColor::ALL {
                            let fill = to_color32(sc.fill_rgba());
                            let is_active = *color == *sc;
                            let size = if is_active { 24.0 } else { 20.0 };
                            let (response, painter) = ui.allocate_painter(
                                egui::vec2(size, size),
                                egui::Sense::click(),
                            );
                            let r = response.rect;
                            painter.circle_filled(r.center(), size / 2.0, fill);
                            if is_active {
                                painter.circle_stroke(
                                    r.center(),
                                    size / 2.0,
                                    Stroke::new(2.0, self.theme.text_primary),
                                );
                            }
                            if response.clicked() {
                                *color = *sc;
                                node.style.fill_color = sc.fill_rgba();
                                node.style.text_color = sc.text_rgba();
                            }
                        }
                    });
                }
                NodeKind::Entity { name, attributes } => {
                    ui.label(egui::RichText::new("Content").size(11.0).color(theme.text_secondary).strong());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Name").size(11.0).color(theme.text_dim));
                    ui.add_space(2.0);
                    let name_response = ui.add(
                        egui::TextEdit::singleline(name)
                            .desired_width(f32::INFINITY)
                            .font(FontId::proportional(13.0)),
                    );
                    if self.focus_label_edit {
                        name_response.request_focus();
                        self.focus_label_edit = false;
                    }
                    ui.add_space(12.0);

                    ui.label(egui::RichText::new("Attributes").size(11.0).color(theme.text_secondary).strong());
                    ui.add_space(4.0);

                    let mut to_remove: Option<usize> = None;
                    for (i, attr) in attributes.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            let pk_text = if attr.is_primary_key {
                                egui::RichText::new("PK").size(10.5).strong().color(theme.accent)
                            } else {
                                egui::RichText::new("PK").size(10.5).color(theme.text_dim)
                            };
                            if ui
                                .add(egui::Button::new(pk_text).min_size(egui::vec2(28.0, 20.0)))
                                .on_hover_text(
                                    "Primary Key — uniquely identifies each row in this table",
                                )
                                .clicked()
                            {
                                attr.is_primary_key = !attr.is_primary_key;
                            }
                            let fk_text = if attr.is_foreign_key {
                                egui::RichText::new("FK")
                                    .size(10.5)
                                    .strong()
                                    .color(theme.fk_color)
                            } else {
                                egui::RichText::new("FK").size(10.5).color(theme.text_dim)
                            };
                            if ui
                                .add(egui::Button::new(fk_text).min_size(egui::vec2(28.0, 20.0)))
                                .on_hover_text(
                                    "Foreign Key — references a primary key in another table",
                                )
                                .clicked()
                            {
                                attr.is_foreign_key = !attr.is_foreign_key;
                            }
                            ui.add(
                                egui::TextEdit::singleline(&mut attr.name)
                                    .desired_width(60.0)
                                    .font(FontId::proportional(11.0)),
                            )
                            .on_hover_text("Attribute name (e.g. id, name, email)");
                            ui.add(
                                egui::TextEdit::singleline(&mut attr.attr_type)
                                    .desired_width(50.0)
                                    .font(FontId::monospace(10.0)),
                            )
                            .on_hover_text("Data type (e.g. INT, VARCHAR, TIMESTAMP)");
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("x").size(10.0).color(theme.text_dim),
                                    )
                                    .min_size(egui::vec2(18.0, 18.0)),
                                )
                                .on_hover_text("Remove this attribute")
                                .clicked()
                            {
                                to_remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = to_remove {
                        attributes.remove(i);
                        needs_entity_resize = true;
                    }
                    ui.add_space(4.0);
                    if ui
                        .add(egui::Button::new(
                            egui::RichText::new("+ Add Attribute").size(11.0).color(theme.accent),
                        ))
                        .clicked()
                    {
                        attributes.push(EntityAttribute {
                            name: String::from("field"),
                            attr_type: String::from("INT"),
                            is_primary_key: false,
                            is_foreign_key: false,
                        });
                        needs_entity_resize = true;
                    }
                }
                NodeKind::Text { content } => {
                    ui.label(egui::RichText::new("Content").size(11.0).color(theme.text_secondary).strong());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Content").size(11.0).color(theme.text_dim));
                    ui.add_space(2.0);
                    let text_response = ui.add(
                        egui::TextEdit::multiline(content)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .font(FontId::proportional(13.0)),
                    );
                    if self.focus_label_edit {
                        text_response.request_focus();
                        self.focus_label_edit = false;
                    }
                }
            }
            ui.add_space(16.0);

            if needs_entity_resize {
                node.auto_size_entity();
            }

            // Quick style presets — complete style preset in one click
            ui.label(egui::RichText::new("Quick Styles").size(11.0).color(theme.text_secondary).strong());
            ui.add_space(4.0);
            // (label, fill, border, text, shadow, bold)
            let quick_styles: &[(&str, [u8;4], [u8;4], [u8;4], bool, bool)] = &[
                ("Primary",   [137, 180, 250, 255], [100, 150, 220, 255], [30, 30, 46, 255],  true,  true),
                ("Success",   [166, 227, 161, 255], [120, 190, 115, 255], [30, 30, 46, 255],  false, false),
                ("Warning",   [249, 226, 175, 255], [210, 180, 100, 255], [30, 30, 46, 255],  false, true),
                ("Danger",    [243, 139, 168, 255], [200, 100, 130, 255], [30, 30, 46, 255],  true,  false),
                ("Ghost",     [30, 30, 46, 40],     [100, 100, 130, 120], [200, 200, 220, 255], false, false),
                ("Dark",      [17, 17, 27, 255],    [50, 50, 70, 255],   [205, 214, 244, 255], true,  false),
            ];
            let mut qs_pick: Option<usize> = None;
            ui.horizontal_wrapped(|ui| {
                for (i, (label, fill, border, text, _, _)) in quick_styles.iter().enumerate() {
                    let c = Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], fill[3]);
                    let (resp, painter) = ui.allocate_painter(egui::vec2(52.0, 24.0), egui::Sense::click());
                    let r = resp.rect;
                    painter.rect_filled(r, egui::CornerRadius::same(4), c);
                    painter.rect_stroke(r, egui::CornerRadius::same(4),
                        egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(border[0], border[1], border[2], 200)),
                        egui::StrokeKind::Inside);
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, *label,
                        FontId::proportional(8.0),
                        Color32::from_rgba_unmultiplied(text[0], text[1], text[2], text[3]));
                    if resp.clicked() { qs_pick = Some(i); }
                    resp.on_hover_text(format!("Apply {} style", label));
                }
            });
            if let Some(idx) = qs_pick {
                let (name, fill, border, text, shadow, bold) = quick_styles[idx];
                node.style.fill_color = fill;
                node.style.border_color = border;
                node.style.text_color = text;
                node.style.shadow = shadow;
                node.style.bold = bold;
                applied_quick_style = Some(name);
            }
            ui.add_space(10.0);

            // Color theme presets
            ui.label(egui::RichText::new("Color Themes").size(11.0).color(theme.text_secondary).strong());
            ui.add_space(4.0);
            // Each entry: (name, fill_rgba, border_rgba, text_rgba)
            let themes: &[(&str, [u8;4], [u8;4], [u8;4])] = &[
                ("Default",  [49, 50, 68, 255],    [69, 71, 90, 255],     [205, 214, 244, 255]),
                ("Ocean",    [30, 102, 140, 255],   [87, 189, 220, 255],   [240, 250, 255, 255]),
                ("Sunset",   [140, 60, 30, 255],    [235, 130, 80, 255],   [255, 240, 225, 255]),
                ("Forest",   [40, 100, 55, 255],    [100, 190, 110, 255],  [220, 255, 230, 255]),
                ("Lavender", [90, 60, 130, 255],    [180, 140, 230, 255],  [245, 235, 255, 255]),
                ("Slate",    [50, 60, 80, 255],     [100, 120, 160, 255],  [210, 220, 240, 255]),
                ("Rose",     [130, 40, 70, 255],    [220, 100, 140, 255],  [255, 230, 240, 255]),
                ("Sand",     [120, 100, 60, 255],   [200, 175, 110, 255],  [255, 248, 225, 255]),
            ];
            ui.horizontal_wrapped(|ui| {
                for (name, fill, border, text) in themes {
                    let preview = egui::Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], fill[3]);
                    let (resp, painter) = ui.allocate_painter(egui::vec2(52.0, 28.0), egui::Sense::click());
                    let r = resp.rect;
                    painter.rect_filled(r, egui::CornerRadius::same(5), preview);
                    painter.rect_stroke(r, egui::CornerRadius::same(5),
                        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(border[0], border[1], border[2], 180)),
                        egui::StrokeKind::Inside);
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, *name,
                        FontId::proportional(8.5),
                        egui::Color32::from_rgba_unmultiplied(text[0], text[1], text[2], text[3]));
                    if resp.clicked() {
                        node.style.fill_color = *fill;
                        node.style.border_color = *border;
                        node.style.text_color = *text;
                    }
                    resp.on_hover_text(*name);
                }
            });
            ui.add_space(12.0);

            // Style section (collapsible)
            // Recent colors: clone before borrow to avoid split-borrow issue inside closure
            let recent_colors_snapshot = self.recent_colors.clone();
            let mut recent_color_pick: Option<[u8; 4]> = None;
            let mut style_changed = false;
            egui::CollapsingHeader::new(
                egui::RichText::new("Style").size(11.0).color(theme.text_secondary).strong()
            )
            .default_open(true)
            .id_salt("prop_style")
            .show(ui, |ui| {
                ui.add_space(4.0);
                if !recent_colors_snapshot.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Recent:").size(11.0).color(theme.text_dim));
                        for col in &recent_colors_snapshot {
                            let c = to_color32(*col);
                            let (r, painter) = ui.allocate_painter(egui::vec2(16.0, 16.0), egui::Sense::click());
                            painter.rect_filled(r.rect, egui::CornerRadius::same(3), c);
                            painter.rect_stroke(r.rect, egui::CornerRadius::same(3),
                                egui::Stroke::new(1.0, theme.surface1),
                                egui::StrokeKind::Inside);
                            if r.clicked() { recent_color_pick = Some(*col); }
                            r.on_hover_text(format!("#{:02X}{:02X}{:02X}", col[0], col[1], col[2]));
                        }
                    });
                    ui.add_space(4.0);
                }
                if let Some(picked) = recent_color_pick {
                    node.style.fill_color = picked;
                    style_changed = true;
                }
            ui.horizontal(|ui| {
                let mut c = to_color32(node.style.fill_color);
                ui.label(egui::RichText::new("Fill").size(11.0).color(theme.text_dim));
                if ui.color_edit_button_srgba(&mut c).changed() {
                    node.style.fill_color = c.to_array();
                    // Track recent color
                    let arr = c.to_array();
                    self.recent_colors.retain(|&x| x != arr);
                    self.recent_colors.insert(0, arr);
                    self.recent_colors.truncate(10);
                }
                ui.add_space(8.0);
                let mut b = to_color32(node.style.border_color);
                ui.label(egui::RichText::new("Border").size(11.0).color(theme.text_dim));
                if ui.color_edit_button_srgba(&mut b).changed() {
                    node.style.border_color = b.to_array();
                }
                ui.add_space(8.0);
                let mut t = to_color32(node.style.text_color);
                ui.label(egui::RichText::new("Text").size(11.0).color(theme.text_dim));
                if ui.color_edit_button_srgba(&mut t).changed() {
                    node.style.text_color = t.to_array();
                }
                // Auto-contrast button: pick black or white based on fill luminance
                if ui.small_button("Auto").on_hover_text("Set text color to black or white based on fill brightness").clicked() {
                    let [r, g, b, _] = node.style.fill_color;
                    let luma = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
                    node.style.text_color = if luma > 140.0 {
                        [15, 15, 20, 255]  // dark text on light bg
                    } else {
                        [220, 220, 230, 255]  // light text on dark bg
                    };
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut node.style.border_width, 0.0..=10.0).text("Border"));
                ui.add_space(8.0);
                ui.checkbox(&mut node.style.border_dashed, egui::RichText::new("Dashed").size(11.0).color(theme.text_dim));
                ui.add_space(8.0);
                ui.checkbox(&mut node.style.gradient, egui::RichText::new("Gradient").size(11.0).color(theme.text_dim));
                if node.style.gradient {
                    for (angle, icon, tip) in [(0u8,"↓","Top→Bottom"),(90u8,"→","Left→Right"),(45u8,"↘","Diagonal ↘"),(135u8,"↗","Diagonal ↗")] {
                        let active = node.style.gradient_angle == angle;
                        let btn = egui::Button::new(egui::RichText::new(icon).size(11.0).color(if active { theme.accent } else { theme.text_dim }))
                            .fill(if active { theme.surface1 } else { Color32::TRANSPARENT });
                        if ui.add(btn).on_hover_text(tip).clicked() { node.style.gradient_angle = angle; }
                    }
                }
                ui.add_space(8.0);
                ui.checkbox(&mut node.style.shadow, egui::RichText::new("Shadow").size(11.0).color(theme.text_dim))
                    .on_hover_text("Render a soft drop shadow beneath the node");
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut node.style.font_size, 8.0..=48.0).text("Font"));
                ui.add_space(6.0);
                let b_col = if node.style.bold { theme.accent } else { theme.text_dim };
                let i_col = if node.style.italic { theme.accent } else { theme.text_dim };
                if ui.button(egui::RichText::new("B").strong().color(b_col)).on_hover_text("Bold").clicked() {
                    node.style.bold = !node.style.bold;
                }
                if ui.button(egui::RichText::new("I").italics().color(i_col)).on_hover_text("Italic").clicked() {
                    node.style.italic = !node.style.italic;
                }
                ui.add_space(4.0);
                // Text alignment
                for (align, icon, tip) in [
                    (crate::model::TextAlign::Left,   "≡", "Align left"),
                    (crate::model::TextAlign::Center, "☰", "Align center"),
                    (crate::model::TextAlign::Right,  "≡", "Align right"),
                ] {
                    let active = node.style.text_align == align;
                    let btn = egui::Button::new(
                        egui::RichText::new(icon).size(12.0).color(if active { theme.accent } else { theme.text_dim })
                    ).fill(if active { theme.surface1 } else { Color32::TRANSPARENT });
                    if ui.add(btn).on_hover_text(tip).clicked() {
                        node.style.text_align = align;
                    }
                }
                ui.add_space(4.0);
                // Vertical text alignment
                for (valign, icon, tip) in [
                    (crate::model::TextVAlign::Top,    "⬆", "Top"),
                    (crate::model::TextVAlign::Middle, "⊟", "Middle"),
                    (crate::model::TextVAlign::Bottom, "⬇", "Bottom"),
                ] {
                    let active = node.style.text_valign == valign;
                    let btn = egui::Button::new(
                        egui::RichText::new(icon).size(12.0).color(if active { theme.accent } else { theme.text_dim })
                    ).fill(if active { theme.surface1 } else { Color32::TRANSPARENT });
                    if ui.add(btn).on_hover_text(tip).clicked() {
                        node.style.text_valign = valign;
                    }
                }
            });
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut node.style.corner_radius, 0.0..=40.0).text("Radius"));
            ui.add_space(4.0);
            let mut opacity = node.style.fill_color[3] as f32 / 255.0 * 100.0;
            if ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).text("Fill alpha").suffix("%")).changed() {
                node.style.fill_color[3] = (opacity / 100.0 * 255.0).round() as u8;
            }
            let mut node_opacity = node.style.opacity * 100.0;
            if ui.add(egui::Slider::new(&mut node_opacity, 0.0..=100.0).text("Node opacity").suffix("%")).changed() {
                node.style.opacity = (node_opacity / 100.0).clamp(0.0, 1.0);
            }
            }); // end Style CollapsingHeader
            // Apply recent color pick to recent_colors list (split borrow resolved)
            if style_changed {
                if let Some(col) = recent_color_pick {
                    self.recent_colors.retain(|&x| x != col);
                    self.recent_colors.insert(0, col);
                    self.recent_colors.truncate(10);
                }
            }
            ui.add_space(8.0);

            // Dimensions (collapsible)
            egui::CollapsingHeader::new(
                egui::RichText::new("Dimensions").size(11.0).color(theme.text_secondary).strong()
            )
            .default_open(true)
            .id_salt("prop_dimensions")
            .show(ui, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for (label, w, h) in [("S", 80.0_f32, 50.0_f32), ("M", 150.0, 80.0), ("L", 240.0, 120.0)] {
                        if ui.small_button(label).on_hover_text(format!("{}×{}", w as i32, h as i32)).clicked() {
                            node.size[0] = w;
                            node.size[1] = h;
                        }
                    }
                    let fit_label = match &node.kind {
                        NodeKind::Shape { label, .. } => Some(label.clone()),
                        NodeKind::StickyNote { text, .. } => Some(text.clone()),
                        _ => None,
                    };
                    if let Some(text) = fit_label {
                        let font_px = node.style.font_size;
                        let ch_w = font_px * 0.6;
                        let line_h = font_px * 1.4;
                        let max_w = 200.0_f32;
                        let chars_per_line = (max_w / ch_w).max(1.0) as usize;
                        let lines = text.chars().count().div_ceil(chars_per_line).max(1);
                        let pad = 20.0;
                        let fit_w = ((text.chars().count().min(chars_per_line) as f32) * ch_w + pad * 2.0).max(80.0).min(max_w);
                        let fit_h = (lines as f32 * line_h + pad * 2.0).max(40.0);
                        if ui.small_button("⇲ Fit").on_hover_text("Resize to fit content").clicked() {
                            node.size[0] = fit_w;
                            node.size[1] = fit_h;
                        }
                    }
                });
                ui.add_space(4.0);
                ui.add(egui::Slider::new(&mut node.size[0], 40.0..=400.0).text("W"));
                ui.add_space(4.0);
                ui.add(egui::Slider::new(&mut node.size[1], 30.0..=400.0).text("H"));
                ui.add_space(4.0);
                // Position inline
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("X").size(11.0).color(theme.text_dim));
                    ui.add(egui::DragValue::new(&mut node.position[0]).speed(1.0).suffix(" px"));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Y").size(11.0).color(theme.text_dim));
                    ui.add(egui::DragValue::new(&mut node.position[1]).speed(1.0).suffix(" px"));
                });
            });
        }

        // Reset style
        ui.add_space(8.0);
        if ui.button("↺ Reset style").on_hover_text("Restore default colours, border, font size").clicked() {
            if let Some(node) = self.document.find_node_mut(&node_id) {
                node.style = crate::model::NodeStyle::default();
                self.history.push(&self.document);
            }
        }

        // Tag
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Tag").size(11.0).color(theme.text_secondary).strong());
        ui.add_space(4.0);
        if let Some(node) = self.document.find_node_mut(&node_id) {
            ui.horizontal_wrapped(|ui| {
                let none_selected = node.tag.is_none();
                let none_btn = egui::Button::new(
                    egui::RichText::new("None").size(11.0).color(if none_selected { theme.accent } else { theme.text_dim })
                ).fill(if none_selected { theme.accent_glow } else { Color32::TRANSPARENT }).corner_radius(4.0);
                if ui.add(none_btn).clicked() { node.tag = None; }
                for variant in [crate::model::NodeTag::Critical, crate::model::NodeTag::Warning, crate::model::NodeTag::Ok, crate::model::NodeTag::Info] {
                    let selected = node.tag == Some(variant);
                    let c = to_color32(variant.color());
                    let text_c = if selected { theme.accent } else { theme.text_primary };
                    let bg = if selected { theme.accent_glow } else { Color32::TRANSPARENT };
                    let btn = egui::Button::new(
                        egui::RichText::new(variant.label()).size(11.0).color(text_c)
                    ).fill(bg).corner_radius(4.0);
                    let r = ui.add(btn);
                    // draw colored swatch indicator
                    let swatch = Rect::from_min_size(
                        Pos2::new(r.rect.min.x, r.rect.max.y - 3.0),
                        egui::Vec2::new(r.rect.width(), 3.0),
                    );
                    ui.painter().rect_filled(swatch, egui::CornerRadius::ZERO, c);
                    if r.clicked() { node.tag = Some(variant); }
                }
            });
        }

        // Frame node controls
        if let Some(node) = self.document.find_node_mut(&node_id) {
            if node.is_frame {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Frame").size(11.0).color(theme.text_secondary).strong());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Fill").size(11.0).color(theme.text_dim));
                    let mut fc = egui::Color32::from_rgba_unmultiplied(
                        node.frame_color[0], node.frame_color[1],
                        node.frame_color[2], node.frame_color[3],
                    );
                    if ui.color_edit_button_srgba(&mut fc).changed() {
                        node.frame_color = [fc.r(), fc.g(), fc.b(), fc.a()];
                    }
                });
                // Preset frame color swatches
                let presets: &[([u8;4], &str)] = &[
                    ([89, 91, 118, 40], "Lavender"),
                    ([166, 227, 161, 30], "Green"),
                    ([137, 180, 250, 30], "Blue"),
                    ([243, 139, 168, 30], "Pink"),
                    ([249, 226, 175, 30], "Yellow"),
                ];
                ui.horizontal_wrapped(|ui| {
                    for (col, name) in presets {
                        let c = egui::Color32::from_rgba_unmultiplied(col[0], col[1], col[2], col[3]);
                        let (r, painter) = ui.allocate_painter(egui::vec2(18.0, 18.0), egui::Sense::click());
                        painter.rect_filled(r.rect, egui::CornerRadius::same(3), c);
                        painter.rect_stroke(r.rect, egui::CornerRadius::same(3),
                            egui::Stroke::new(1.0, theme.surface1),
                            egui::StrokeKind::Inside);
                        if r.clicked() { node.frame_color = *col; }
                        r.on_hover_text(*name);
                    }
                });
            }
        }

        // Pin toggle + collapse toggle
        let mut do_collapse = false;
        if let Some(node) = self.document.find_node_mut(&node_id) {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let pin_label = if node.pinned { "📌 Pinned" } else { "📍 Pin" };
                if ui.small_button(pin_label).on_hover_text("Pin prevents movement while dragging").clicked() {
                    node.pinned = !node.pinned;
                }
                let lock_label = if node.locked { "🔓 Unlock" } else { "🔒 Lock" };
                if ui.small_button(lock_label).on_hover_text("Lock prevents movement, resize, and deletion (Cmd+L)").clicked() {
                    node.locked = !node.locked;
                }
                if matches!(node.kind, NodeKind::Shape { .. }) && !node.is_frame {
                    let col_label = if node.collapsed { "▶ Expand" } else { "▼ Collapse" };
                    if ui.small_button(col_label).clicked() {
                        do_collapse = true;
                    }
                }
            });
        }
        if do_collapse {
            if let Some(node) = self.document.find_node_mut(&node_id) {
                node.toggle_collapsed();
                self.history.push(&self.document);
            }
        }

        // Layer order controls (need node_id outside the borrow)
        ui.add_space(12.0);
        ui.label(egui::RichText::new("Layer Order").size(11.0).color(theme.text_secondary).strong());
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("⬆ Front").on_hover_text("Bring to front").clicked() {
                let idx = self.document.nodes.iter().position(|n| n.id == node_id);
                if let Some(i) = idx {
                    let node = self.document.nodes.remove(i);
                    self.document.nodes.push(node);
                    self.history.push(&self.document);
                }
            }
            if ui.button("⬇ Back").on_hover_text("Send to back").clicked() {
                let idx = self.document.nodes.iter().position(|n| n.id == node_id);
                if let Some(i) = idx {
                    let node = self.document.nodes.remove(i);
                    self.document.nodes.insert(0, node);
                    self.history.push(&self.document);
                }
            }
        });

        // Node statistics footer
        ui.add_space(16.0);
        self.draw_divider(ui);
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Node Info").size(11.0).color(theme.text_secondary).strong());
        ui.add_space(4.0);
        if let Some(node) = self.document.find_node(&node_id) {
            let in_deg = self.document.edges.iter().filter(|e| e.target.node_id == node_id).count();
            let out_deg = self.document.edges.iter().filter(|e| e.source.node_id == node_id).count();
            let z_idx = self.document.nodes.iter().position(|n| n.id == node_id).unwrap_or(0);
            let stats: &[(&str, String)] = &[
                ("X", format!("{:.0}", node.position[0])),
                ("Y", format!("{:.0}", node.position[1])),
                ("W", format!("{:.0}", node.size[0])),
                ("H", format!("{:.0}", node.size[1])),
                ("Z", format!("{:.0}", node.z_offset)),
                ("In", in_deg.to_string()),
                ("Out", out_deg.to_string()),
                ("Stk", z_idx.to_string()),
            ];
            egui::Grid::new("node_stats_grid")
                .num_columns(4)
                .spacing([4.0, 2.0])
                .show(ui, |ui| {
                    for (i, (label, val)) in stats.iter().enumerate() {
                        ui.label(egui::RichText::new(*label).size(9.5).color(theme.text_dim));
                        ui.label(egui::RichText::new(val).size(10.0).strong().color(theme.text_secondary));
                        if (i + 1) % 2 == 0 { ui.end_row(); }
                    }
                });
        }
        // Apply quick style history push outside the borrow
        if let Some(style_name) = applied_quick_style {
            self.history.push(&self.document);
            self.status_message = Some((format!("{} style applied", style_name), std::time::Instant::now()));
        }
    }

    fn draw_edge_properties(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        let edge_id = *self.selection.edge_ids.iter().next().unwrap();
        if let Some(edge) = self.document.find_edge_mut(&edge_id) {
            ui.label(egui::RichText::new("Edge").size(13.0).strong().color(theme.accent));
            ui.add_space(12.0);

            ui.label(egui::RichText::new("Content").size(11.0).color(theme.text_secondary).strong());
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Label").size(11.0).color(theme.text_dim));
            ui.add_space(2.0);
            ui.add(
                egui::TextEdit::singleline(&mut edge.label)
                    .desired_width(f32::INFINITY)
                    .font(FontId::proportional(13.0)),
            );
            ui.add_space(12.0);

            ui.label(egui::RichText::new("Relationship").size(11.0).color(theme.text_secondary).strong());
            ui.add_space(4.0);

            let rel_presets: &[(&str, &str, Cardinality, Cardinality, &str)] = &[
                (
                    "None",
                    "──▶",
                    Cardinality::None,
                    Cardinality::None,
                    "No cardinality. A plain arrow.",
                ),
                (
                    "1 : 1",
                    "||──||",
                    Cardinality::ExactlyOne,
                    Cardinality::ExactlyOne,
                    "One to One\nEach record relates to exactly one on the other side.\nExample: User ↔ Profile",
                ),
                (
                    "1 : N",
                    "||──o<",
                    Cardinality::ExactlyOne,
                    Cardinality::ZeroOrMany,
                    "One to Many\nOne source record relates to many targets.\nExample: User → many Orders",
                ),
                (
                    "N : 1",
                    "o<──||",
                    Cardinality::ZeroOrMany,
                    Cardinality::ExactlyOne,
                    "Many to One\nMany source records relate to one target.\nExample: many Orders → one User",
                ),
                (
                    "M : N",
                    "o<──o<",
                    Cardinality::ZeroOrMany,
                    Cardinality::ZeroOrMany,
                    "Many to Many\nMany on both sides. Needs a junction table.\nExample: Students ↔ Courses",
                ),
                (
                    "1 : 0..1",
                    "||──o|",
                    Cardinality::ExactlyOne,
                    Cardinality::ZeroOrOne,
                    "One to Optional\nOne source relates to zero or one target.\nExample: User → optional Address",
                ),
                (
                    "1 : 1..N",
                    "||──|<",
                    Cardinality::ExactlyOne,
                    Cardinality::OneOrMany,
                    "One to One-or-Many\nOne source relates to at least one target.\nExample: Order → one or more Items",
                ),
            ];

            for (label, symbol, src, tgt, tooltip) in rel_presets {
                let is_selected =
                    edge.source_cardinality == *src && edge.target_cardinality == *tgt;
                let text_color = if is_selected { theme.accent } else { theme.text_primary };
                let bg = if is_selected {
                    theme.accent_glow
                } else {
                    Color32::TRANSPARENT
                };

                let btn = egui::Button::new(
                    egui::RichText::new(format!("{:<8} {}", label, symbol))
                        .size(11.0)
                        .family(egui::FontFamily::Monospace)
                        .color(text_color),
                )
                .fill(bg)
                .stroke(egui::Stroke::NONE)
                .min_size(egui::vec2(ui.available_width(), 24.0))
                .corner_radius(4.0);

                let resp = ui.add(btn);

                if resp.hovered() && !is_selected {
                    let hover_rect = resp.rect;
                    ui.painter().rect_filled(hover_rect, 4.0, theme.text_hover_bg);
                }

                let clicked = resp.clicked();
                resp.on_hover_text(*tooltip);
                if clicked {
                    edge.source_cardinality = *src;
                    edge.target_cardinality = *tgt;
                }
            }
            ui.add_space(8.0);

            // Text labels
            ui.label(egui::RichText::new("Text Labels").size(11.0).color(theme.text_secondary).strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Source").size(11.0).color(theme.text_dim));
                ui.add(
                    egui::TextEdit::singleline(&mut edge.source_label)
                        .desired_width(60.0)
                        .font(FontId::proportional(11.0)),
                );
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Target").size(11.0).color(theme.text_dim));
                ui.add(
                    egui::TextEdit::singleline(&mut edge.target_label)
                        .desired_width(60.0)
                        .font(FontId::proportional(11.0)),
                );
            });
            ui.add_space(12.0);

            ui.label(egui::RichText::new("Style").size(11.0).color(theme.text_secondary).strong());
            ui.horizontal(|ui| {
                let mut c = to_color32(edge.style.color);
                ui.label(egui::RichText::new("Color").size(11.0).color(theme.text_dim));
                if ui.color_edit_button_srgba(&mut c).changed() {
                    edge.style.color = c.to_array();
                }
                ui.add_space(8.0);
                ui.checkbox(&mut edge.style.dashed, egui::RichText::new("Dashed").size(11.0).color(theme.text_dim));
                ui.add_space(8.0);
                ui.checkbox(&mut edge.style.orthogonal, egui::RichText::new("Orthogonal").size(11.0).color(theme.text_dim));
                ui.add_space(8.0);
                ui.checkbox(&mut edge.style.glow, egui::RichText::new("Glow").size(11.0).color(theme.text_dim));
                ui.add_space(8.0);
                ui.checkbox(&mut edge.style.animated, egui::RichText::new("Flow ▶").size(11.0).color(theme.text_dim))
                    .on_hover_text("Animate dashes to show data flow direction");
            });
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut edge.style.width, 1.0..=10.0).text("Width"));
                // Visual thickness preview line
                let w = 40.0_f32;
                let h = 20.0_f32;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
                let mid_y = rect.center().y;
                let edge_col = to_color32(edge.style.color);
                ui.painter().line_segment(
                    [egui::pos2(rect.min.x + 4.0, mid_y), egui::pos2(rect.max.x - 4.0, mid_y)],
                    egui::Stroke::new(edge.style.width.clamp(1.0, 8.0), edge_col),
                );
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut edge.style.curve_bend, -200.0..=200.0).text("Bend"));
                if ui.small_button("↺").on_hover_text("Reset curve bend to 0").clicked() {
                    edge.style.curve_bend = 0.0;
                }
            });
            ui.add_space(8.0);

            // Edge annotation / note
            ui.add_space(8.0);
            ui.label(egui::RichText::new("💬 Note").size(11.0).color(theme.text_dim));
            ui.add_space(2.0);
            ui.add(
                egui::TextEdit::singleline(&mut edge.comment)
                    .desired_width(f32::INFINITY)
                    .hint_text("annotation shown on hover")
                    .font(FontId::proportional(11.0)),
            );

            ui.label(egui::RichText::new("Arrow Head").size(11.0).color(theme.text_secondary).strong());
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (variant, label, tooltip) in [
                    (ArrowHead::Filled,  "▶ Filled",  "Solid filled triangle"),
                    (ArrowHead::Open,    "⌄ Open",    "Open chevron (no fill)"),
                    (ArrowHead::Circle,  "● Circle",  "Circle endpoint"),
                    (ArrowHead::None,    "— None",    "No arrowhead"),
                ] {
                    let selected = edge.style.arrow_head == variant;
                    let text_color = if selected { theme.accent } else { theme.text_primary };
                    let bg = if selected { theme.accent_glow } else { Color32::TRANSPARENT };
                    let btn = egui::Button::new(
                        egui::RichText::new(label).size(11.0).color(text_color)
                    ).fill(bg).corner_radius(4.0);
                    if ui.add(btn).on_hover_text(tooltip).clicked() {
                        edge.style.arrow_head = variant;
                    }
                }
            });
        }
    }

    fn draw_multi_selection_tools(&mut self, ui: &mut egui::Ui, sel_nodes: usize, total: usize) {
        let theme = self.theme.clone();
        let sel_edges = self.selection.edge_ids.len();
        ui.label(
            egui::RichText::new(format!("{} items selected", total))
                .size(13.0)
                .color(theme.text_secondary),
        );
        ui.add_space(8.0);

        // Path inspection + quick connect: when exactly 2 nodes selected
        if sel_nodes == 2 && sel_edges == 0 {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            let (src, tgt) = (ids[0], ids[1]);
            let path_len = self.bfs_path_length(src, tgt);
            self.draw_section_header(ui, "Path Analysis");
            ui.add_space(4.0);
            if let Some(hops) = path_len {
                ui.label(egui::RichText::new(format!("Shortest path: {} hop(s)", hops)).size(11.0).color(theme.text_secondary));
            } else {
                ui.label(egui::RichText::new("No path between nodes").size(11.0).color(theme.text_dim));
            }
            ui.add_space(6.0);
            // Quick connect button
            let already_connected = self.document.edges.iter().any(|e| {
                (e.source.node_id == src && e.target.node_id == tgt) ||
                (e.source.node_id == tgt && e.target.node_id == src)
            });
            ui.horizontal(|ui| {
                let btn = ui.add_enabled(
                    !already_connected,
                    egui::Button::new(egui::RichText::new("→ Connect").size(11.0)),
                );
                if btn.clicked() {
                    let edge = Edge {
                        id: EdgeId::new(),
                        source: Port { node_id: src, side: crate::model::PortSide::Right },
                        target: Port { node_id: tgt, side: crate::model::PortSide::Left },
                        label: String::new(),
                        source_label: String::new(),
                        target_label: String::new(),
                        source_cardinality: crate::model::Cardinality::None,
                        target_cardinality: crate::model::Cardinality::None,
                        style: EdgeStyle::default(),
                        comment: String::new(),
                    };
                    self.document.edges.push(edge);
                    self.history.push(&self.document);
                }
                if already_connected {
                    ui.label(egui::RichText::new("(already connected)").size(10.0).color(theme.text_dim));
                }
            });
            ui.add_space(8.0);
        }
        ui.add_space(4.0);

        // Batch edge style when edges are selected
        if sel_edges >= 1 {
            self.draw_section_header(ui, "Batch Edge Style");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.small_button("Solid").clicked() {
                    let ids: Vec<EdgeId> = self.selection.edge_ids.iter().copied().collect();
                    for id in &ids {
                        if let Some(e) = self.document.find_edge_mut(id) { e.style.dashed = false; }
                    }
                    self.history.push(&self.document);
                }
                if ui.small_button("Dashed").clicked() {
                    let ids: Vec<EdgeId> = self.selection.edge_ids.iter().copied().collect();
                    for id in &ids {
                        if let Some(e) = self.document.find_edge_mut(id) { e.style.dashed = true; }
                    }
                    self.history.push(&self.document);
                }
                if ui.small_button("Orthog.").clicked() {
                    let ids: Vec<EdgeId> = self.selection.edge_ids.iter().copied().collect();
                    for id in &ids {
                        if let Some(e) = self.document.find_edge_mut(id) { e.style.orthogonal = true; }
                    }
                    self.history.push(&self.document);
                }
                if ui.small_button("Bezier").clicked() {
                    let ids: Vec<EdgeId> = self.selection.edge_ids.iter().copied().collect();
                    for id in &ids {
                        if let Some(e) = self.document.find_edge_mut(id) { e.style.orthogonal = false; }
                    }
                    self.history.push(&self.document);
                }
            });
            ui.add_space(4.0);
            // Batch arrow head
            ui.horizontal(|ui| {
                for (variant, label) in [(ArrowHead::Filled, "▶"), (ArrowHead::Open, "⌄"), (ArrowHead::Circle, "●"), (ArrowHead::None, "—")] {
                    if ui.small_button(label).on_hover_text(format!("{:?}", variant)).clicked() {
                        let ids: Vec<EdgeId> = self.selection.edge_ids.iter().copied().collect();
                        for id in &ids {
                            if let Some(e) = self.document.find_edge_mut(id) { e.style.arrow_head = variant; }
                        }
                        self.history.push(&self.document);
                    }
                }
            });
            ui.add_space(12.0);
        }

        if sel_nodes < 2 { return; }

        // Batch tag for multi-node selection
        self.draw_section_header(ui, "Batch Tag");
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let tags: &[(Option<NodeTag>, &str)] = &[
                (None, "⬜ None"),
                (Some(NodeTag::Critical), "🔴 Crit"),
                (Some(NodeTag::Warning),  "🟡 Warn"),
                (Some(NodeTag::Ok),       "🟢 OK"),
                (Some(NodeTag::Info),     "🔵 Info"),
            ];
            for (variant, label) in tags {
                if ui.small_button(*label).clicked() {
                    let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                    for id in &ids {
                        if let Some(n) = self.document.find_node_mut(id) {
                            n.tag = *variant;
                        }
                    }
                    self.history.push(&self.document);
                }
            }
        });
        ui.add_space(8.0);

        self.draw_section_header(ui, "Batch Color");
        ui.add_space(4.0);
        let palette: &[([u8;4], &str)] = &[
            ([137, 180, 250, 220], "Blue"),
            ([166, 227, 161, 220], "Green"),
            ([243, 139, 168, 220], "Red"),
            ([249, 226, 175, 220], "Yellow"),
            ([203, 166, 247, 220], "Purple"),
            ([148, 226, 213, 220], "Teal"),
            ([49,  50,  68, 255], "Default"),
        ];
        ui.horizontal_wrapped(|ui| {
            for (color, name) in palette {
                let c = to_color32(*color);
                let btn = egui::Button::new("  ").fill(c).min_size(egui::Vec2::new(22.0, 22.0));
                if ui.add(btn).on_hover_text(*name).clicked() {
                    let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                    for id in &ids {
                        if let Some(n) = self.document.find_node_mut(id) {
                            n.style.fill_color = *color;
                        }
                    }
                    self.history.push(&self.document);
                }
            }
        });
        ui.add_space(12.0);

        self.draw_section_header(ui, "Align");
        ui.add_space(6.0);

        // Row 1: horizontal alignment
        ui.horizontal(|ui| {
            if ui.button("⬤← Left").on_hover_text("Align left edges").clicked() {
                self.align_nodes_left();
            }
            if ui.button("⬤↔ Center").on_hover_text("Center horizontally").clicked() {
                self.align_nodes_center_h();
            }
            if ui.button("→⬤ Right").on_hover_text("Align right edges").clicked() {
                self.align_nodes_right();
            }
        });
        ui.add_space(4.0);
        // Row 2: vertical alignment
        ui.horizontal(|ui| {
            if ui.button("⬤↑ Top").on_hover_text("Align top edges").clicked() {
                self.align_nodes_top();
            }
            if ui.button("⬤↕ Mid").on_hover_text("Center vertically").clicked() {
                self.align_nodes_center_v();
            }
            if ui.button("↓⬤ Bot").on_hover_text("Align bottom edges").clicked() {
                self.align_nodes_bottom();
            }
        });
        ui.add_space(4.0);
        // Distribute
        ui.horizontal(|ui| {
            if ui.button("↔ Distrib H").on_hover_text("Distribute evenly (horizontal)").clicked() {
                self.distribute_nodes_h();
            }
            if ui.button("↕ Distrib V").on_hover_text("Distribute evenly (vertical)").clicked() {
                self.distribute_nodes_v();
            }
        });
    }

    // Alignment helpers

    fn selected_node_ids(&self) -> Vec<NodeId> {
        self.selection.node_ids.iter().copied().collect()
    }

    pub(crate) fn align_nodes_left(&mut self) {
        let ids = self.selected_node_ids();
        let min_x = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[0]).fold(f32::MAX, f32::min);
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[0] = min_x; }
        }
        self.history.push(&self.document);
    }

    pub(crate) fn align_nodes_right(&mut self) {
        let ids = self.selected_node_ids();
        let max_x = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[0] + n.size[0]).fold(f32::MIN, f32::max);
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[0] = max_x - n.size[0]; }
        }
        self.history.push(&self.document);
    }

    pub(crate) fn align_nodes_center_h(&mut self) {
        let ids = self.selected_node_ids();
        let centers: Vec<f32> = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[0] + n.size[0] / 2.0).collect();
        let avg = centers.iter().sum::<f32>() / centers.len() as f32;
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[0] = avg - n.size[0] / 2.0; }
        }
        self.history.push(&self.document);
    }

    pub(crate) fn align_nodes_top(&mut self) {
        let ids = self.selected_node_ids();
        let min_y = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[1]).fold(f32::MAX, f32::min);
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[1] = min_y; }
        }
        self.history.push(&self.document);
    }

    pub(crate) fn align_nodes_bottom(&mut self) {
        let ids = self.selected_node_ids();
        let max_y = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[1] + n.size[1]).fold(f32::MIN, f32::max);
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[1] = max_y - n.size[1]; }
        }
        self.history.push(&self.document);
    }

    pub(crate) fn align_nodes_center_v(&mut self) {
        let ids = self.selected_node_ids();
        let centers: Vec<f32> = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[1] + n.size[1] / 2.0).collect();
        let avg = centers.iter().sum::<f32>() / centers.len() as f32;
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[1] = avg - n.size[1] / 2.0; }
        }
        self.history.push(&self.document);
    }

    pub(crate) fn distribute_nodes_h(&mut self) {
        let ids = self.selected_node_ids();
        if ids.len() < 3 { return; }
        let mut nodes: Vec<(NodeId, f32, f32)> = ids.iter()
            .filter_map(|id| self.document.find_node(id).map(|n| (*id, n.position[0], n.size[0])))
            .collect();
        nodes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let total_w: f32 = nodes.iter().map(|(_, _, w)| w).sum();
        let span = nodes.last().map(|(_, x, w)| x + w).unwrap_or(0.0) - nodes.first().map(|(_, x, _)| *x).unwrap_or(0.0);
        let gap = (span - total_w) / (nodes.len() as f32 - 1.0);
        let mut x = nodes[0].1;
        for (id, _, w) in &nodes {
            if let Some(n) = self.document.find_node_mut(id) { n.position[0] = x; }
            x += w + gap;
        }
        self.history.push(&self.document);
    }

    pub(crate) fn distribute_nodes_v(&mut self) {
        let ids = self.selected_node_ids();
        if ids.len() < 3 { return; }
        let mut nodes: Vec<(NodeId, f32, f32)> = ids.iter()
            .filter_map(|id| self.document.find_node(id).map(|n| (*id, n.position[1], n.size[1])))
            .collect();
        nodes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let total_h: f32 = nodes.iter().map(|(_, _, h)| h).sum();
        let span = nodes.last().map(|(_, y, h)| y + h).unwrap_or(0.0) - nodes.first().map(|(_, y, _)| *y).unwrap_or(0.0);
        let gap = (span - total_h) / (nodes.len() as f32 - 1.0);
        let mut y = nodes[0].1;
        for (id, _, h) in &nodes {
            if let Some(n) = self.document.find_node_mut(id) { n.position[1] = y; }
            y += h + gap;
        }
        self.history.push(&self.document);
    }
}
