use egui::{Color32, FontId, SidePanel, Stroke};
use crate::model::*;
use super::FlowchartApp;
use super::theme::*;

impl FlowchartApp {
    pub(crate) fn draw_properties_panel(&mut self, ctx: &egui::Context) {
        SidePanel::right("properties")
            .resizable(false)
            .exact_width(PROPERTIES_WIDTH)
            .frame(egui::Frame {
                fill: MANTLE,
                inner_margin: egui::Margin::same(16),
                stroke: Stroke::new(1.0, SURFACE1),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("PROPERTIES")
                        .size(10.0)
                        .color(TEXT_DIM)
                        .strong(),
                );
                ui.add_space(12.0);

                let sel_nodes = self.selection.node_ids.len();
                let sel_edges = self.selection.edge_ids.len();
                let total = sel_nodes + sel_edges;

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
    }

    fn draw_empty_selection(&self, ui: &mut egui::Ui) {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            // Subtle icon-like indicator
            ui.label(
                egui::RichText::new("\u{25CB}")  // circle outline
                    .size(28.0)
                    .color(SURFACE1),
            );
            ui.add_space(12.0);
            ui.label(egui::RichText::new("No selection").size(13.0).color(TEXT_DIM));
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Click a node or edge\nto edit properties")
                    .size(11.0)
                    .color(TEXT_DIM),
            );
            ui.add_space(16.0);
            ui.label(
                egui::RichText::new("Tip: Double-click to edit labels")
                    .size(10.0)
                    .color(SURFACE1),
            );
        });
    }

    fn draw_node_properties(&mut self, ui: &mut egui::Ui) {
        let node_id = *self.selection.node_ids.iter().next().unwrap();
        if let Some(node) = self.document.find_node_mut(&node_id) {
            let kind_name = match &node.kind {
                NodeKind::Shape { shape, .. } => match shape {
                    NodeShape::Rectangle => "Rectangle",
                    NodeShape::RoundedRect => "Rounded Rect",
                    NodeShape::Diamond => "Diamond",
                    NodeShape::Circle => "Circle",
                    NodeShape::Parallelogram => "Parallelogram",
                    NodeShape::Connector => "Connector",
                },
                NodeKind::StickyNote { .. } => "Sticky Note",
                NodeKind::Entity { .. } => "Entity",
                NodeKind::Text { .. } => "Text",
            };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(kind_name).size(13.0).strong().color(ACCENT));
            });
            ui.add_space(12.0);

            let mut needs_entity_resize = false;
            match &mut node.kind {
                NodeKind::Shape {
                    label, description, ..
                } => {
                    Self::draw_section_header(ui, "CONTENT");
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Label").size(11.0).color(TEXT_DIM));
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
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Description").size(11.0).color(TEXT_DIM));
                    ui.add_space(2.0);
                    ui.add(
                        egui::TextEdit::multiline(description)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .font(FontId::proportional(12.0)),
                    );
                }
                NodeKind::StickyNote { text, color } => {
                    Self::draw_section_header(ui, "CONTENT");
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Text").size(11.0).color(TEXT_DIM));
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

                    Self::draw_section_header(ui, "COLOR");
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
                                    Stroke::new(2.0, Color32::WHITE),
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
                    Self::draw_section_header(ui, "CONTENT");
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Name").size(11.0).color(TEXT_DIM));
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

                    Self::draw_section_header(ui, "ATTRIBUTES");
                    ui.add_space(4.0);

                    let mut to_remove: Option<usize> = None;
                    for (i, attr) in attributes.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            let pk_text = if attr.is_primary_key {
                                egui::RichText::new("PK").size(9.0).strong().color(ACCENT)
                            } else {
                                egui::RichText::new("PK").size(9.0).color(TEXT_DIM)
                            };
                            if ui
                                .add(egui::Button::new(pk_text).min_size(egui::vec2(24.0, 18.0)))
                                .on_hover_text(
                                    "Primary Key — uniquely identifies each row in this table",
                                )
                                .clicked()
                            {
                                attr.is_primary_key = !attr.is_primary_key;
                            }
                            let fk_text = if attr.is_foreign_key {
                                egui::RichText::new("FK")
                                    .size(9.0)
                                    .strong()
                                    .color(FK_COLOR)
                            } else {
                                egui::RichText::new("FK").size(9.0).color(TEXT_DIM)
                            };
                            if ui
                                .add(egui::Button::new(fk_text).min_size(egui::vec2(24.0, 18.0)))
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
                                        egui::RichText::new("x").size(10.0).color(TEXT_DIM),
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
                            egui::RichText::new("+ Add Attribute").size(11.0).color(ACCENT),
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
                    Self::draw_section_header(ui, "CONTENT");
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Content").size(11.0).color(TEXT_DIM));
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

            // Style section
            Self::draw_section_header(ui, "STYLE");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let mut c = to_color32(node.style.fill_color);
                ui.label(egui::RichText::new("Fill").size(11.0).color(TEXT_DIM));
                if ui.color_edit_button_srgba(&mut c).changed() {
                    node.style.fill_color = c.to_array();
                }
                ui.add_space(16.0);
                let mut b = to_color32(node.style.border_color);
                ui.label(egui::RichText::new("Border").size(11.0).color(TEXT_DIM));
                if ui.color_edit_button_srgba(&mut b).changed() {
                    node.style.border_color = b.to_array();
                }
            });
            ui.add_space(8.0);
            ui.add(
                egui::Slider::new(&mut node.style.border_width, 0.0..=10.0).text("Border"),
            );
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut node.style.font_size, 8.0..=48.0).text("Font"));
            ui.add_space(4.0);
            let mut opacity = node.style.fill_color[3] as f32 / 255.0 * 100.0;
            if ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).text("Opacity").suffix("%")).changed() {
                node.style.fill_color[3] = (opacity / 100.0 * 255.0).round() as u8;
            }
            ui.add_space(16.0);

            // Dimensions
            Self::draw_section_header(ui, "DIMENSIONS");
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut node.size[0], 40.0..=400.0).text("W"));
            ui.add_space(4.0);
            ui.add(egui::Slider::new(&mut node.size[1], 30.0..=400.0).text("H"));
        }

        // Layer order controls (need node_id outside the borrow)
        ui.add_space(12.0);
        Self::draw_section_header(ui, "LAYER ORDER");
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
    }

    fn draw_edge_properties(&mut self, ui: &mut egui::Ui) {
        let edge_id = *self.selection.edge_ids.iter().next().unwrap();
        if let Some(edge) = self.document.find_edge_mut(&edge_id) {
            ui.label(egui::RichText::new("Edge").size(13.0).strong().color(ACCENT));
            ui.add_space(12.0);

            Self::draw_section_header(ui, "CONTENT");
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Label").size(11.0).color(TEXT_DIM));
            ui.add_space(2.0);
            ui.add(
                egui::TextEdit::singleline(&mut edge.label)
                    .desired_width(f32::INFINITY)
                    .font(FontId::proportional(13.0)),
            );
            ui.add_space(12.0);

            Self::draw_section_header(ui, "RELATIONSHIP");
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
                let text_color = if is_selected { ACCENT } else { TEXT_PRIMARY };
                let bg = if is_selected {
                    ACCENT_GLOW
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
                    ui.painter().rect_filled(hover_rect, 4.0, TEXT_HOVER_BG);
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
            Self::draw_section_header(ui, "TEXT LABELS");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Source").size(11.0).color(TEXT_DIM));
                ui.add(
                    egui::TextEdit::singleline(&mut edge.source_label)
                        .desired_width(60.0)
                        .font(FontId::proportional(11.0)),
                );
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Target").size(11.0).color(TEXT_DIM));
                ui.add(
                    egui::TextEdit::singleline(&mut edge.target_label)
                        .desired_width(60.0)
                        .font(FontId::proportional(11.0)),
                );
            });
            ui.add_space(12.0);

            Self::draw_section_header(ui, "STYLE");
            ui.horizontal(|ui| {
                let mut c = to_color32(edge.style.color);
                ui.label(egui::RichText::new("Color").size(11.0).color(TEXT_DIM));
                if ui.color_edit_button_srgba(&mut c).changed() {
                    edge.style.color = c.to_array();
                }
                ui.add_space(16.0);
                ui.checkbox(&mut edge.style.dashed, egui::RichText::new("Dashed").size(11.0).color(TEXT_DIM));
            });
            ui.add(egui::Slider::new(&mut edge.style.width, 1.0..=10.0).text("Width"));
        }
    }

    fn draw_multi_selection_tools(&mut self, ui: &mut egui::Ui, sel_nodes: usize, total: usize) {
        ui.label(
            egui::RichText::new(format!("{} items selected", total))
                .size(13.0)
                .color(TEXT_SECONDARY),
        );
        ui.add_space(16.0);

        if sel_nodes < 2 { return; }

        Self::draw_section_header(ui, "ALIGN");
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

    fn align_nodes_left(&mut self) {
        let ids = self.selected_node_ids();
        let min_x = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[0]).fold(f32::MAX, f32::min);
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[0] = min_x; }
        }
        self.history.push(&self.document);
    }

    fn align_nodes_right(&mut self) {
        let ids = self.selected_node_ids();
        let max_x = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[0] + n.size[0]).fold(f32::MIN, f32::max);
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[0] = max_x - n.size[0]; }
        }
        self.history.push(&self.document);
    }

    fn align_nodes_center_h(&mut self) {
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

    fn align_nodes_top(&mut self) {
        let ids = self.selected_node_ids();
        let min_y = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[1]).fold(f32::MAX, f32::min);
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[1] = min_y; }
        }
        self.history.push(&self.document);
    }

    fn align_nodes_bottom(&mut self) {
        let ids = self.selected_node_ids();
        let max_y = ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.position[1] + n.size[1]).fold(f32::MIN, f32::max);
        for id in &ids {
            if let Some(n) = self.document.find_node_mut(id) { n.position[1] = max_y - n.size[1]; }
        }
        self.history.push(&self.document);
    }

    fn align_nodes_center_v(&mut self) {
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

    fn distribute_nodes_h(&mut self) {
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

    fn distribute_nodes_v(&mut self) {
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
