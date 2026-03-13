// overlays.rs — Floating UI panels drawn on top of the canvas.
//
// Extracted from mod.rs to keep the main update() loop concise.
// Each method draws one overlay and manages its own open/close state.

use egui::{Color32, Pos2, Vec2};
use crate::app::FlowchartApp;
use crate::model::*;

// ---------------------------------------------------------------------------
// Spec editor syntax highlighter
// ---------------------------------------------------------------------------

/// Build a syntax-highlighted `LayoutJob` for the HRF spec editor.
/// Called as a TextEdit layouter closure.
fn spec_syntax_layout(ui: &egui::Ui, text: &str, wrap_width: f32) -> std::sync::Arc<egui::Galley> {
    use egui::text::{LayoutJob, TextFormat};
    use egui::FontId;

    // Colours (dark-theme palette)
    let c_section  = Color32::from_rgb(137, 180, 250); // blue  — ## headings
    let c_comment  = Color32::from_rgb(108, 112, 134); // gray  — // comments
    let c_arrow    = Color32::from_rgb(203, 166, 247); // mauve — --> edges
    let c_tag      = Color32::from_rgb(166, 227, 161); // green — {tags}
    let c_node     = Color32::from_rgb(205, 214, 244); // white — node lines
    let c_key      = Color32::from_rgb(250, 179, 135); // peach — key = value
    let c_default  = Color32::from_rgb(166, 173, 200); // subtext0

    let font = FontId::monospace(12.5);
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    job.wrap.break_anywhere = false;

    let fmt = |color: Color32| TextFormat { font_id: font.clone(), color, ..Default::default() };

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();

        if trimmed.starts_with("## ") || trimmed.starts_with("# ") {
            // Section / title headers
            job.append(line, 0.0, fmt(c_section));
        } else if trimmed.starts_with("//") {
            // Comment lines
            job.append(line, 0.0, fmt(c_comment));
        } else if trimmed.contains("-->") || trimmed.contains("->") || trimmed.contains("<--") || trimmed.contains("<->") {
            // Edge / flow lines — colour tags inline
            append_line_with_tags(&mut job, line, c_arrow, c_tag, &font);
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            // Node definition lines
            append_line_with_tags(&mut job, line, c_node, c_tag, &font);
        } else if trimmed.contains(" = ") && !trimmed.starts_with(' ') {
            // Config key = value lines
            if let Some(eq) = line.find(" = ") {
                job.append(&line[..eq + 3], 0.0, fmt(c_key));
                job.append(&line[eq + 3..], 0.0, fmt(c_default));
            } else {
                job.append(line, 0.0, fmt(c_key));
            }
        } else {
            // Description / other text
            job.append(line, 0.0, fmt(c_default));
        }
    }

    ui.fonts(|f| f.layout_job(job))
}

/// Append a line to the LayoutJob, colouring `{...}` tag spans in `tag_color`.
fn append_line_with_tags(
    job: &mut egui::text::LayoutJob,
    line: &str,
    base_color: Color32,
    tag_color: Color32,
    font: &egui::FontId,
) {
    use egui::text::TextFormat;
    let fmt_base = TextFormat { font_id: font.clone(), color: base_color, ..Default::default() };
    let fmt_tag  = TextFormat { font_id: font.clone(), color: tag_color,  ..Default::default() };

    let mut remaining = line;
    while !remaining.is_empty() {
        if let Some(open) = remaining.find('{') {
            // Append the part before the tag
            if open > 0 {
                job.append(&remaining[..open], 0.0, fmt_base.clone());
            }
            remaining = &remaining[open..];
            // Find closing brace
            if let Some(close) = remaining.find('}') {
                job.append(&remaining[..close + 1], 0.0, fmt_tag.clone());
                remaining = &remaining[close + 1..];
            } else {
                // No closing brace — rest of line is a tag
                job.append(remaining, 0.0, fmt_tag.clone());
                break;
            }
        } else {
            job.append(remaining, 0.0, fmt_base.clone());
            break;
        }
    }
}

impl FlowchartApp {
    // -----------------------------------------------------------------------
    // Zoom indicator pill (fades after zoom changes)
    // -----------------------------------------------------------------------
    pub(crate) fn draw_zoom_indicator(&mut self, ctx: &egui::Context) {
        let current_zoom = self.effective_zoom();
        let now = ctx.input(|i| i.time);
        if (current_zoom - self.last_zoom).abs() > 0.001 {
            self.zoom_indicator_time = Some(now);
            self.last_zoom = current_zoom;
        }
        let Some(birth) = self.zoom_indicator_time else { return };
        let age = (now - birth) as f32;
        let lifetime = 1.5_f32;
        if age >= lifetime {
            self.zoom_indicator_time = None;
            return;
        }
        ctx.request_repaint();
        let alpha = ((1.0 - (age / lifetime).powi(2)) * 255.0) as u8;
        let zoom_pct = (current_zoom * 100.0).round() as i32;
        let text = format!("{zoom_pct}%");
        egui::Area::new(egui::Id::new("zoom_indicator"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 52.0])
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                let galley = ui.fonts(|f| {
                    f.layout_no_wrap(
                        text.clone(),
                        egui::FontId::proportional(18.0),
                        Color32::from_rgba_premultiplied(self.theme.text_primary.r(), self.theme.text_primary.g(), self.theme.text_primary.b(), alpha),
                    )
                });
                let size = galley.size() + Vec2::new(20.0, 10.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let bg = Color32::from_rgba_premultiplied(self.theme.canvas_bg.r(), self.theme.canvas_bg.g(), self.theme.canvas_bg.b(), alpha.saturating_sub(30));
                ui.painter().rect_filled(rect, egui::CornerRadius::same(8), bg);
                ui.painter().rect_stroke(
                    rect,
                    egui::CornerRadius::same(8),
                    egui::Stroke::new(
                        1.0,
                        Color32::from_rgba_premultiplied(self.theme.accent.r(), self.theme.accent.g(), self.theme.accent.b(), alpha / 2),
                    ),
                    egui::StrokeKind::Outside,
                );
                ui.painter().galley(
                    Pos2::new(rect.min.x + 10.0, rect.center().y - galley.size().y / 2.0),
                    galley,
                    self.theme.text_primary,
                );
            });
    }

    // -----------------------------------------------------------------------
    // Find & Replace dialog (Cmd+H)
    // -----------------------------------------------------------------------
    pub(crate) fn draw_find_replace(&mut self, ctx: &egui::Context) {
        if !self.show_find_replace {
            return;
        }
        let mut open = self.show_find_replace;
        let mut do_replace_all = false;
        egui::Window::new("Find & Replace")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find:");
                    ui.text_edit_singleline(&mut self.find_query);
                });
                ui.horizontal(|ui| {
                    ui.label("Replace:");
                    ui.text_edit_singleline(&mut self.replace_query);
                });
                ui.add_space(4.0);
                let count = self
                    .document
                    .nodes
                    .iter()
                    .filter(|n| {
                        !self.find_query.is_empty()
                            && n.display_label()
                                .to_lowercase()
                                .contains(&self.find_query.to_lowercase())
                    })
                    .count();
                if !self.find_query.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("{count} match(es)"))
                            .size(10.5)
                            .color(self.theme.text_dim),
                    );
                }
                ui.add_space(4.0);
                if ui.button("Replace All").clicked() {
                    do_replace_all = true;
                }
            });
        if do_replace_all && !self.find_query.is_empty() {
            let find = self.find_query.to_lowercase();
            let replace = self.replace_query.clone();
            let mut changed = 0usize;
            for node in self.document.nodes.iter_mut() {
                match &mut node.kind {
                    NodeKind::Shape { label, .. } => {
                        if label.to_lowercase().contains(&find) {
                            *label = label.to_lowercase().replace(&find, &replace);
                            changed += 1;
                        }
                    }
                    NodeKind::StickyNote { text, .. } => {
                        if text.to_lowercase().contains(&find) {
                            *text = text.to_lowercase().replace(&find, &replace);
                            changed += 1;
                        }
                    }
                    NodeKind::Entity { name, .. } => {
                        if name.to_lowercase().contains(&find) {
                            *name = name.to_lowercase().replace(&find, &replace);
                            changed += 1;
                        }
                    }
                    NodeKind::Text { content } => {
                        if content.to_lowercase().contains(&find) {
                            *content = content.to_lowercase().replace(&find, &replace);
                            changed += 1;
                        }
                    }
                }
            }
            if changed > 0 {
                self.history.push(&self.document);
                self.status_message = Some((
                    format!("Replaced {changed} node(s)"),
                    std::time::Instant::now(),
                ));
            }
        }
        self.show_find_replace = open;
    }

    // -----------------------------------------------------------------------
    // Shape picker palette (N key)
    // -----------------------------------------------------------------------
    pub(crate) fn draw_shape_picker(&mut self, ctx: &egui::Context) {
        let Some(picker_pos) = self.shape_picker else {
            return;
        };
        let shapes: &[(&str, NodeKind)] = &[
            (
                "■ Rect",
                NodeKind::Shape {
                    shape: NodeShape::Rectangle,
                    label: String::new(),
                    description: String::new(),
                },
            ),
            (
                "⬮ Round",
                NodeKind::Shape {
                    shape: NodeShape::RoundedRect,
                    label: String::new(),
                    description: String::new(),
                },
            ),
            (
                "◆ Diamond",
                NodeKind::Shape {
                    shape: NodeShape::Diamond,
                    label: String::new(),
                    description: String::new(),
                },
            ),
            (
                "● Circle",
                NodeKind::Shape {
                    shape: NodeShape::Circle,
                    label: String::new(),
                    description: String::new(),
                },
            ),
            (
                "▱ Parallel",
                NodeKind::Shape {
                    shape: NodeShape::Parallelogram,
                    label: String::new(),
                    description: String::new(),
                },
            ),
            (
                "⬡ Hexagon",
                NodeKind::Shape {
                    shape: NodeShape::Hexagon,
                    label: String::new(),
                    description: String::new(),
                },
            ),
            (
                "📝 Sticky",
                NodeKind::StickyNote {
                    text: String::new(),
                    color: StickyColor::Yellow,
                },
            ),
            ("T Text", NodeKind::Text { content: String::new() }),
        ];
        let canvas_pos = self.viewport.screen_to_canvas(picker_pos);
        let mut chosen: Option<NodeKind> = None;
        let mut close = false;
        egui::Window::new("##shape_picker")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_pos(picker_pos)
            .frame(egui::Frame {
                fill: self.theme.surface0,
                inner_margin: egui::Margin::same(8),
                stroke: egui::Stroke::new(1.0, self.theme.surface1),
                corner_radius: egui::CornerRadius::same(8),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Insert node").size(10.0).color(self.theme.text_dim));
                ui.add_space(4.0);
                for (label, kind) in shapes {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new(*label).size(12.0))
                                .min_size(egui::vec2(110.0, 22.0)),
                        )
                        .clicked()
                    {
                        chosen = Some(kind.clone());
                        close = true;
                    }
                }
                if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
                if ui
                    .ctx()
                    .pointer_latest_pos()
                    .map_or(false, |_p| !ui.ctx().is_pointer_over_area())
                {
                    close = true;
                }
            });
        if let Some(kind) = chosen {
            let w = 120.0_f32;
            let h = 70.0_f32;
            let pos = egui::Pos2::new(canvas_pos.x - w / 2.0, canvas_pos.y - h / 2.0);
            let node = Node {
                id: NodeId::new(),
                kind,
                position: [pos.x, pos.y],
                size: [w, h],
                z_offset: 0.0,
                style: NodeStyle::default(),
                pinned: false,
                tag: None,
                collapsed: false,
                uncollapsed_size: None,
                url: String::new(),
                locked: false,
                comment: String::new(),
                is_frame: false,
                frame_color: default_frame_color(),
                icon: String::new(),
                sublabel: String::new(),
                depth_3d: 0.0,
                highlight: false,
                progress: 0.0,
            };
            let id = node.id;
            self.document.nodes.push(node);
            self.selection.select_node(id);
            self.focus_label_edit = true;
            self.history.push(&self.document);
            self.status_message = Some(("Node inserted".to_string(), std::time::Instant::now()));
        }
        if close {
            self.shape_picker = None;
        }
    }

    // -----------------------------------------------------------------------
    // Inline edge label editor (double-click edge)
    // -----------------------------------------------------------------------
    pub(crate) fn draw_edge_label_editor(&mut self, ctx: &egui::Context) {
        let Some((edge_id, pos)) = self.inline_edge_edit else {
            return;
        };
        let mut close_editor = false;
        egui::Window::new("##edge_label_editor")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_pos(pos)
            .frame(egui::Frame {
                fill: self.theme.surface0,
                inner_margin: egui::Margin::same(6),
                stroke: egui::Stroke::new(1.0, self.theme.accent),
                corner_radius: egui::CornerRadius::same(6),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Edge label").size(10.0).color(self.theme.text_dim));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let char_count = self
                            .document
                            .find_edge(&edge_id)
                            .map(|e| e.label.chars().count())
                            .unwrap_or(0);
                        let count_color = if char_count > 45 {
                            Color32::from_rgb(243, 139, 168)
                        } else {
                            self.theme.text_dim
                        };
                        ui.label(
                            egui::RichText::new(format!("{}/50", char_count))
                                .size(9.5)
                                .color(count_color),
                        );
                    });
                });
                if let Some(edge) = self.document.find_edge_mut(&edge_id) {
                    if edge.label.chars().count() > 50 {
                        let trimmed: String = edge.label.chars().take(50).collect();
                        edge.label = trimmed;
                    }
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut edge.label)
                            .desired_width(180.0)
                            .hint_text("e.g. depends on, owns, sends to…")
                            .font(egui::FontId::proportional(13.0)),
                    );
                    resp.request_focus();
                    if ui.ctx().input(|i| {
                        i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)
                    }) {
                        close_editor = true;
                        self.history.push(&self.document);
                    }
                } else {
                    close_editor = true;
                }
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Enter")
                            .monospace()
                            .size(9.5)
                            .color(self.theme.accent.gamma_multiply(0.7)),
                    );
                    ui.label(egui::RichText::new("save").size(9.5).color(self.theme.text_dim));
                    ui.label(egui::RichText::new("·").size(9.5).color(self.theme.text_dim));
                    ui.label(
                        egui::RichText::new("Esc")
                            .monospace()
                            .size(9.5)
                            .color(self.theme.accent.gamma_multiply(0.7)),
                    );
                    ui.label(egui::RichText::new("cancel").size(9.5).color(self.theme.text_dim));
                });
            });
        if close_editor {
            self.inline_edge_edit = None;
        }
    }

    // -----------------------------------------------------------------------
    // Comment editor (Cmd+M)
    // -----------------------------------------------------------------------
    pub(crate) fn draw_comment_editor(&mut self, ctx: &egui::Context) {
        let Some(node_id) = self.comment_editing else {
            return;
        };
        let mut close_comment = false;
        let node_screen_pos = self
            .document
            .find_node(&node_id)
            .map(|n| {
                let p = self.viewport.canvas_to_screen(n.pos());
                let s = n.size_vec() * self.viewport.zoom;
                Pos2::new(p.x + s.x + 8.0, p.y)
            })
            .unwrap_or(Pos2::new(200.0, 200.0));
        egui::Window::new("##comment_editor")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_pos(node_screen_pos)
            .frame(egui::Frame {
                fill: Color32::from_rgba_unmultiplied(249, 226, 175, 240),
                inner_margin: egui::Margin::same(8),
                stroke: egui::Stroke::new(
                    1.5,
                    Color32::from_rgba_unmultiplied(200, 175, 100, 255),
                ),
                corner_radius: egui::CornerRadius::same(8),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("💬 Comment")
                        .size(10.0)
                        .color(Color32::from_rgba_unmultiplied(80, 60, 20, 255)),
                );
                if let Some(node) = self.document.find_node_mut(&node_id) {
                    let resp = ui.add(
                        egui::TextEdit::multiline(&mut node.comment)
                            .desired_width(200.0)
                            .desired_rows(3)
                            .font(egui::FontId::proportional(12.0))
                            .text_color(Color32::from_rgba_unmultiplied(60, 40, 10, 255)),
                    );
                    resp.request_focus();
                    if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                        close_comment = true;
                        self.history.push(&self.document);
                    }
                } else {
                    close_comment = true;
                }
                ui.horizontal(|ui| {
                    if ui.small_button("✓ Done").clicked() {
                        close_comment = true;
                        self.history.push(&self.document);
                    }
                    if ui.small_button("🗑 Clear").clicked() {
                        if let Some(node) = self.document.find_node_mut(&node_id) {
                            node.comment.clear();
                        }
                        close_comment = true;
                        self.history.push(&self.document);
                    }
                });
            });
        if close_comment {
            self.comment_editing = None;
        }
    }

    // -----------------------------------------------------------------------
    // Keyboard shortcuts panel (F1 / ?)
    // -----------------------------------------------------------------------
    pub(crate) fn draw_shortcuts_panel(&mut self, ctx: &egui::Context) {
        if !self.show_shortcuts_panel {
            return;
        }
        let mut open = self.show_shortcuts_panel;
        egui::Window::new("Keyboard Shortcuts")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(460.0)
            .show(ctx, |ui| {
                type Section = (&'static str, &'static [(&'static str, &'static str)]);
                let sections: &[Section] = &[
                    (
                        "Tools",
                        &[
                            ("V", "Select tool"),
                            ("E", "Connect / edge tool"),
                            ("N", "Insert shape picker"),
                            ("R / C / D", "Quick-create Rect / Circle / Diamond"),
                            ("Double-click canvas", "Create node"),
                            ("Double-click node", "Edit label"),
                            ("Right-click", "Context menu"),
                        ],
                    ),
                    (
                        "Selection",
                        &[
                            ("⌘A", "Select all"),
                            ("Escape", "Deselect"),
                            ("Del / Backspace", "Delete selected"),
                            ("Arrow keys", "Nudge 1 px  (⇧ = 10 px)"),
                            ("⇧H / ⇧V", "Distribute selected horizontally / vertically"),
                            ("⌘G", "Group into frame"),
                        ],
                    ),
                    (
                        "Edit",
                        &[
                            ("⌘Z", "Undo"),
                            ("⌘⇧Z", "Redo"),
                            ("⌘C / ⌘V", "Copy / Paste (nodes + edges)"),
                            ("⌘D", "Duplicate"),
                            ("⌘B / ⌘I", "Toggle bold / italic"),
                            ("⌘⇧H", "Collapse / expand selected nodes"),
                            ("⌘L", "Auto-layout (hierarchical)"),
                            ("⌘⇧> / ⌘⇧<", "Increase / decrease font size"),
                        ],
                    ),
                    (
                        "View",
                        &[
                            ("⌘1", "Fit all to view"),
                            ("⌘2", "Zoom to selection"),
                            ("⌘= / ⌘-", "Zoom in / out"),
                            ("⌘0", "Reset zoom to 100%"),
                            ("F", "Focus mode — dim unconnected nodes"),
                            ("⇧T", "Toggle dark/light mode"),
                            ("G", "Toggle grid"),
                            (
                                "S",
                                "Toggle snap · S with edge selected = cycle edge style",
                            ),
                            ("O", "Bird's-eye overview"),
                            ("Alt+hover", "Show distance rulers"),
                        ],
                    ),
                    (
                        "Search & Navigate",
                        &[
                            ("⌘F", "Search nodes (spotlight) — dims non-matches"),
                            ("↑ / ↓", "Navigate search results"),
                            ("Enter", "Jump to search result"),
                            ("⌘H", "Find & replace node labels"),
                            ("⌘E", "Live spec editor — edit HRF, canvas updates in real time"),
                            ("⌘⇧1–5", "Save viewport bookmark"),
                            ("⇧1–5", "Jump to bookmark"),
                        ],
                    ),
                    (
                        "3D View",
                        &[
                            ("Tab", "Toggle 2D / 3D view"),
                            ("1", "Camera: Isometric (3D mode)"),
                            ("2", "Camera: Top-down (3D mode)"),
                            ("3", "Camera: Front elevation (3D mode)"),
                            ("4", "Camera: Right side (3D mode)"),
                            ("Drag (3D)", "Orbit camera"),
                            ("Right-drag (3D)", "Pan camera"),
                            ("Scroll (3D)", "Zoom camera"),
                        ],
                    ),
                    (
                        "Help",
                        &[
                            ("F1 / ?", "This shortcuts panel"),
                            ("⌘K", "Command palette"),
                            ("[", "Collapse / expand left toolbar"),
                            ("]", "Collapse / expand right panel"),
                            ("⇧R", "Toggle coordinate rulers"),
                        ],
                    ),
                ];
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        for (section, items) in sections {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(*section)
                                    .size(10.0)
                                    .color(self.theme.text_dim)
                                    .strong(),
                            );
                            egui::Grid::new(format!("sc_{}", section))
                                .striped(true)
                                .num_columns(2)
                                .spacing([16.0, 3.0])
                                .show(ui, |ui| {
                                    for (key, desc) in *items {
                                        ui.label(
                                            egui::RichText::new(*key)
                                                .monospace()
                                                .color(self.theme.accent)
                                                .size(11.5),
                                        );
                                        ui.label(
                                            egui::RichText::new(*desc)
                                                .size(11.5)
                                                .color(self.theme.text_secondary),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                    });
            });
        self.show_shortcuts_panel = open;
    }

    // -----------------------------------------------------------------------
    // Live HRF Spec Editor (Cmd+E) — side panel that shows the current spec
    // as editable text and re-parses after 400ms of idle typing.
    // -----------------------------------------------------------------------
    pub(crate) fn draw_spec_editor(&mut self, ctx: &egui::Context) {
        if !self.show_spec_editor { return; }

        let panel_w = 420.0_f32;
        let theme = self.theme.clone();

        let mut keep_open = true;
        egui::SidePanel::right("spec_editor_panel")
            .exact_width(panel_w)
            .resizable(true)
            .frame(egui::Frame::NONE
                .fill(theme.surface0)
                .inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                // Header bar
                let mut do_sync = false;
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.colored_label(theme.text_primary,
                        egui::RichText::new("Spec Editor").size(13.0).strong());
                    ui.add_space(4.0);
                    ui.colored_label(theme.text_dim,
                        egui::RichText::new("— edit to update canvas").size(10.5));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if ui.small_button("✕").on_hover_text("Close spec editor  Cmd+E").clicked() {
                            keep_open = false;
                        }
                        ui.add_space(4.0);
                        if ui.small_button("⎘").on_hover_text("Copy spec to clipboard").clicked() {
                            ui.ctx().copy_text(self.spec_editor_text.clone());
                            self.status_message = Some(("Spec copied to clipboard".to_string(), std::time::Instant::now()));
                        }
                        ui.add_space(4.0);
                        if ui.small_button("↻").on_hover_text("Sync text from current canvas state").clicked() {
                            do_sync = true;
                        }
                    });
                });
                if do_sync {
                    let title = self.document.title.clone();
                    self.spec_editor_text = crate::specgraph::hrf::export_hrf(&self.document, &title);
                    self.spec_editor_last_edit = None;
                    self.spec_editor_error = None;
                }

                // Thin divider
                let rect = ui.available_rect_before_wrap();
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_top()],
                    egui::Stroke::new(1.0, theme.surface1),
                );
                ui.add_space(1.0);

                // Error banner (if last parse failed) — try to extract line number
                if let Some(ref err) = self.spec_editor_error.clone() {
                    let err_color = Color32::from_rgb(243, 139, 168);
                    // Try to extract "Line N:" from the error message
                    let line_snippet: Option<String> = {
                        // Pattern: "Line N: ..."
                        let re_prefix = "Line ";
                        if let Some(pos) = err.find(re_prefix) {
                            let after = &err[pos + re_prefix.len()..];
                            if let Some(colon) = after.find(':') {
                                let line_num_str = &after[..colon];
                                if let Ok(n) = line_num_str.trim().parse::<usize>() {
                                    let lines: Vec<&str> = self.spec_editor_text.lines().collect();
                                    if n > 0 && n <= lines.len() {
                                        let raw = lines[n - 1].trim();
                                        let preview = if raw.len() > 50 { &raw[..50] } else { raw };
                                        Some(format!("→ {}", preview))
                                    } else { None }
                                } else { None }
                            } else { None }
                        } else { None }
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(10.0);
                        ui.colored_label(err_color, egui::RichText::new(format!("⚠ {}", err)).size(10.5));
                    });
                    if let Some(snippet) = line_snippet {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.colored_label(
                                err_color.gamma_multiply(0.7),
                                egui::RichText::new(snippet).size(10.0).monospace(),
                            );
                        });
                    }
                    ui.add_space(4.0);
                }

                // Main text editor
                let available = ui.available_rect_before_wrap();
                let mut ui2 = ui.new_child(
                    egui::UiBuilder::new().max_rect(available)
                );
                let resp = ui2.add_sized(
                    available.size(),
                    egui::TextEdit::multiline(&mut self.spec_editor_text)
                        .font(egui::FontId::monospace(12.5))
                        .desired_rows(40)
                        .lock_focus(false)
                        .layouter(&mut spec_syntax_layout),
                );
                if resp.changed() {
                    let now = ui2.ctx().input(|i| i.time);
                    self.spec_editor_last_edit = Some(now);
                    self.spec_editor_error = None;
                    // Request repaint so debounce fires promptly
                    ui2.ctx().request_repaint_after(std::time::Duration::from_millis(420));
                }
                // Escape closes panel
                if ui2.ctx().input(|i| i.key_pressed(egui::Key::Escape)) && resp.has_focus() {
                    keep_open = false;
                }

                // Status footer: show node/edge count or pending indicator
                let n = self.document.nodes.len();
                let e = self.document.edges.len();
                let total_lines = self.spec_editor_text.lines().count();
                let (footer_text, footer_color) = if let Some(_) = self.spec_editor_last_edit {
                    ("⏱ parsing…".to_string(), theme.text_dim)
                } else if self.spec_editor_error.is_some() {
                    ("✗ parse error".to_string(), Color32::from_rgb(243, 139, 168))
                } else {
                    (format!("✓  {} nodes  {}  edges", n, e), Color32::from_rgb(166, 227, 161))
                };
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.colored_label(footer_color, egui::RichText::new(footer_text).size(10.5));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.colored_label(theme.text_dim,
                            egui::RichText::new(format!("{} lines", total_lines)).size(10.0));
                        ui.add_space(8.0);
                        ui.colored_label(theme.text_dim,
                            egui::RichText::new("Cmd+E to close").size(10.0));
                    });
                });
            });

        if !keep_open {
            self.show_spec_editor = false;
        }
    }

    /// Parse `spec_editor_text` and apply to `document` if valid.
    /// Called by the debounce timer in `update()`.
    pub(crate) fn apply_spec_editor_text(&mut self) {
        use crate::specgraph::hrf::parse_hrf;
        match parse_hrf(&self.spec_editor_text) {
            Ok(doc) => {
                // Preserve viewport (don't jump around)
                let vp = self.viewport.clone();
                self.document = doc;
                self.viewport = vp;
                self.history.push(&self.document);
                self.spec_editor_error = None;
            }
            Err(e) => {
                self.spec_editor_error = Some(e.to_string());
            }
        }
    }
}
