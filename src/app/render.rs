use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::model::*;
use super::FlowchartApp;
use super::interaction::{control_points_for_side, cubic_bezier_point};
use super::theme::{PORT_RADIUS, to_color32};

/// Infer a semantic watermark icon from node label keywords.
/// Returns a single emoji/symbol that represents the node's conceptual type.
fn semantic_icon_for_label(label: &str) -> Option<&'static str> {
    let lc = label.to_lowercase();
    // Check most-specific patterns first
    if lc.contains("database") || lc.contains(" db") || lc.starts_with("db ")
        || lc.contains("postgres") || lc.contains("mysql") || lc.contains("sqlite")
        || lc.contains("mongo") || lc.contains("redis") || lc.contains("store") { return Some("🗄️"); }
    if lc.contains("user") || lc.contains("person") || lc.contains("actor")
        || lc.contains("customer") || lc.contains("client") || lc.contains("member") { return Some("👤"); }
    if lc.contains("server") || lc.contains("backend") || lc.contains("service")
        || lc.contains("microservice") || lc.contains("worker") { return Some("⚙️"); }
    if lc.contains("api") || lc.contains("gateway") || lc.contains("endpoint")
        || lc.contains("rest") || lc.contains("graphql") { return Some("🔌"); }
    if lc.contains("web") || lc.contains("browser") || lc.contains("frontend")
        || lc.contains("ui") || lc.contains("app") || lc.contains("mobile") { return Some("🌐"); }
    if lc.contains("auth") || lc.contains("login") || lc.contains("oauth")
        || lc.contains("sso") || lc.contains("jwt") || lc.contains("security") { return Some("🔐"); }
    if lc.contains("email") || lc.contains("mail") || lc.contains("smtp")
        || lc.contains("notification") || lc.contains("alert") { return Some("📧"); }
    if lc.contains("queue") || lc.contains("kafka") || lc.contains("pubsub")
        || lc.contains("rabbitmq") || lc.contains("event") || lc.contains("message") { return Some("📨"); }
    if lc.contains("cloud") || lc.contains("aws") || lc.contains("azure")
        || lc.contains("gcp") || lc.contains("s3") { return Some("☁️"); }
    if lc.contains("docker") || lc.contains("container") || lc.contains("k8s")
        || lc.contains("kubernetes") || lc.contains("deploy") { return Some("🐳"); }
    if lc.contains("search") || lc.contains("query") || lc.contains("index")
        || lc.contains("elastic") || lc.contains("solr") { return Some("🔍"); }
    if lc.contains("file") || lc.contains("document") || lc.contains("report")
        || lc.contains("pdf") || lc.contains("csv") || lc.contains("export") { return Some("📄"); }
    if lc.contains("payment") || lc.contains("billing") || lc.contains("invoice")
        || lc.contains("stripe") || lc.contains("wallet") { return Some("💳"); }
    if lc.contains("log") || lc.contains("monitor") || lc.contains("metric")
        || lc.contains("telemetry") || lc.contains("analytics") { return Some("📊"); }
    if lc.contains("cache") || lc.contains("memory") || lc.contains("buffer") { return Some("⚡"); }
    if lc.contains("start") || lc.contains("begin") || lc.contains("init")
        || lc.contains("trigger") { return Some("▶"); }
    if lc.contains("end") || lc.contains("stop") || lc.contains("finish")
        || lc.contains("terminate") { return Some("⬛"); }
    if lc.contains("test") || lc.contains("qa") || lc.contains("check")
        || lc.contains("verify") { return Some("✅"); }
    if lc.contains("error") || lc.contains("fail") || lc.contains("exception")
        || lc.contains("catch") { return Some("❌"); }
    if lc.contains("ci") || lc.contains("cd") || lc.contains("build")
        || lc.contains("pipeline") { return Some("🔧"); }
    None
}

fn paint_gradient_rect_angle(painter: &egui::Painter, rect: Rect, color_a: Color32, color_b: Color32, angle_deg: u8) {
    let mut mesh = egui::Mesh::default();
    // Assign corner colors based on direction (TL, TR, BR, BL)
    let (tl, tr, br, bl): (Color32, Color32, Color32, Color32) = match angle_deg {
        90  => (color_a, color_b, color_b, color_a), // left→right
        45  => (color_a, color_a, color_b, color_b), // ↘ diagonal
        135 => (color_b, color_a, color_a, color_b), // ↗ diagonal
        _   => (color_a, color_a, color_b, color_b), // 0 = top→bottom
    };
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.min,                               uv: Pos2::ZERO, color: tl });
    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.max.x, rect.min.y),      uv: Pos2::ZERO, color: tr });
    mesh.vertices.push(egui::epaint::Vertex { pos: rect.max,                               uv: Pos2::ZERO, color: br });
    mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.min.x, rect.max.y),      uv: Pos2::ZERO, color: bl });
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    painter.add(egui::Shape::mesh(mesh));
}

/// Linearly interpolate between two Color32 values.
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

/// Darken a Color32 by blending toward black by `amount` (0.0 = unchanged, 1.0 = black).
fn darken(c: Color32, amount: f32) -> Color32 {
    let f = 1.0 - amount.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (c.r() as f32 * f) as u8,
        (c.g() as f32 * f) as u8,
        (c.b() as f32 * f) as u8,
        c.a(),
    )
}

impl FlowchartApp {
    pub(crate) fn draw_node(&self, node: &Node, painter: &egui::Painter, hover_pos: Option<Pos2>) {
        let top_left = self.viewport.canvas_to_screen(node.pos());
        let size = node.size_vec() * self.viewport.zoom;
        let screen_rect = Rect::from_min_size(top_left, size);

        let is_selected = self.selection.contains_node(&node.id);

        // Semantic zoom: at very low zoom, render as compact dot-label (LOD0)
        if self.viewport.zoom < 0.28 && !node.is_frame {
            let fill = to_color32(node.style.fill_color);
            // Hub nodes get larger dots: degree scales radius by up to 1.8×
            let degree = self.document.edges.iter()
                .filter(|e| e.source.node_id == node.id || e.target.node_id == node.id)
                .count();
            let hub_scale = 1.0 + (degree as f32 / 10.0).min(1.0) * 0.8;
            let base_r = (screen_rect.width().max(screen_rect.height()) * 0.35).clamp(3.0, 10.0);
            let dot_r = (base_r * hub_scale).clamp(3.0, 14.0);
            let center = screen_rect.center();
            // Highlight ring: amber pulsing even in LOD0
            if node.highlight {
                let time = painter.ctx().input(|i| i.time);
                let breath = ((time * std::f64::consts::PI).sin() as f32) * 0.3 + 0.7;
                painter.circle_filled(center, dot_r + 3.5, Color32::from_rgba_unmultiplied(255, 185, 0, (breath * 55.0) as u8));
                painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
            }
            // Ring for selection
            if is_selected {
                painter.circle_filled(center, dot_r + 3.0, self.theme.accent_glow);
            }
            // Hub glow: warm ring for high-degree nodes
            if degree >= 5 {
                let glow_alpha = ((degree as f32 / 12.0).min(1.0) * 80.0) as u8;
                painter.circle_filled(center, dot_r + 2.5, self.theme.fk_color.gamma_multiply(glow_alpha as f32 / 255.0));
            }
            painter.circle_filled(center, dot_r, fill);
            painter.circle_stroke(center, dot_r, Stroke::new(0.8, to_color32(node.style.border_color)));
            // Tiny label if there's room (at least 4px radius)
            if dot_r >= 4.0 {
                let label_str = match &node.kind {
                    NodeKind::Shape { label, .. } => label.as_str(),
                    NodeKind::StickyNote { text, .. } => text.as_str(),
                    NodeKind::Entity { name, .. } => name.as_str(),
                    NodeKind::Text { content } => content.as_str(),
                };
                let short: String = label_str.chars().take(12).collect();
                // Append ⭐ for highlighted nodes, 💬 for nodes with comments
                let suffix = if node.highlight { " ⭐" } else if !node.comment.is_empty() { " 💬" } else { "" };
                painter.text(
                    center + Vec2::new(dot_r + 2.0, 0.0),
                    Align2::LEFT_CENTER,
                    &format!("{}{}", short, suffix),
                    FontId::proportional(6.5),
                    to_color32(node.style.text_color).gamma_multiply(0.8),
                );
            }
            return;
        }
        let is_hovered = hover_pos.map_or(false, |hp| screen_rect.expand(6.0).contains(hp));

        // Selection-confirmation flash: brief bright overlay when node is first selected
        if is_selected {
            let now = painter.ctx().input(|i| i.time);
            if let Some(&sel_time) = self.selection_times.get(&node.id) {
                let age = (now - sel_time) as f32;
                if age < 0.25 {
                    // Flash: bright white fade from alpha=150→0 over 250ms, slight expand
                    let t = age / 0.25;
                    let flash_alpha = ((1.0 - t) * 150.0) as u8;
                    let expand = (1.0 - t) * 6.0;
                    painter.rect_filled(
                        screen_rect.expand(expand),
                        CornerRadius::same(6),
                        self.theme.accent.gamma_multiply(flash_alpha as f32 / 255.0),
                    );
                    painter.ctx().request_repaint_after(std::time::Duration::from_millis(16));
                }
            }
        }

        // Selection glow (pulsing animation)
        if is_selected {
            let time = painter.ctx().input(|i| i.time);
            // Pulse: oscillates between 0.3 and 1.0 at ~0.8 Hz
            let pulse = ((time * 1.6 * std::f64::consts::PI).sin() as f32) * 0.35 + 0.65;
            let glow_rect = screen_rect.expand(5.0 + pulse * 2.0);
            let glow_color = self.theme.accent_glow.gamma_multiply(pulse);
            painter.rect_filled(glow_rect, CornerRadius::same(6), glow_color);
            // Request continuous repaint for animation
            painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        } else if is_hovered {
            painter.rect_stroke(
                screen_rect.expand(2.0),
                CornerRadius::same(4),
                Stroke::new(1.5, self.theme.accent_hover),
                StrokeKind::Outside,
            );
        }

        // Node freshness ring: bright expanding border for "just created" nodes (0–3s)
        if let Some(&birth) = self.node_birth_times.get(&node.id) {
            let now = painter.ctx().input(|i| i.time);
            let age = (now - birth) as f32;
            let duration = 3.0_f32;
            if age < duration {
                // Phase 1 (0–0.4s): bright solid ring; Phase 2 (0.4–3s): fade away
                let alpha = if age < 0.4 {
                    220u8
                } else {
                    ((1.0 - (age - 0.4) / (duration - 0.4)).powf(1.5) * 120.0) as u8
                };
                let ring_color = self.theme.toast_success.gamma_multiply(alpha as f32 / 255.0);
                let expand = if age < 0.4 {
                    2.0 + age * 8.0  // expand outward during birth flash
                } else {
                    5.0
                };
                let cr = CornerRadius::same((node.style.corner_radius as f32 + expand * 0.5) as u8);
                painter.rect_stroke(
                    screen_rect.expand(expand),
                    cr,
                    Stroke::new(1.5, ring_color),
                    StrokeKind::Outside,
                );
                if age < duration - 0.1 {
                    painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
                }
            }
        }

        // Connection density ring: thin outer ring color-coded by node degree
        // Only visible at normal zoom and not for frames
        if !node.is_frame && self.viewport.zoom > 0.4 {
            let degree = self.document.edges.iter()
                .filter(|e| e.source.node_id == node.id || e.target.node_id == node.id)
                .count();
            if degree >= 3 {
                // 3–5: blue  6–9: orange  10+: red-orange
                let ring_color = if degree >= 10 {
                    Color32::from_rgba_unmultiplied(235, 120, 70, 70)
                } else if degree >= 6 {
                    Color32::from_rgba_unmultiplied(235, 175, 60, 65)
                } else {
                    Color32::from_rgba_unmultiplied(120, 180, 255, 60)
                };
                let cr = CornerRadius::same((node.style.corner_radius as f32 * self.viewport.zoom.sqrt() + 0.5) as u8);
                let ring_width = (0.8 + (degree as f32 / 10.0).min(1.0) * 1.5).clamp(0.8, 2.3);
                painter.rect_stroke(
                    screen_rect.expand(4.5),
                    cr,
                    Stroke::new(ring_width, ring_color),
                    StrokeKind::Outside,
                );
            }
        }

        // Highlight ring: slow amber pulse for nodes marked as important
        if node.highlight && !node.is_frame {
            let time = painter.ctx().input(|i| i.time);
            // Slow breath: ~0.5 Hz, oscillates between 0.4 and 1.0
            let breath = ((time * std::f64::consts::PI).sin() as f32) * 0.3 + 0.7;
            let expand = 5.0 + breath * 3.0;
            let cr = CornerRadius::same((node.style.corner_radius as f32 * self.viewport.zoom.sqrt() + expand * 0.4) as u8);
            // Outer soft glow fill
            let glow_fill = Color32::from_rgba_unmultiplied(255, 185, 0, (breath * 22.0) as u8);
            painter.rect_filled(screen_rect.expand(expand + 3.0), cr, glow_fill);
            // Inner sharp stroke
            let stroke_alpha = (breath * 180.0 + 40.0) as u8;
            painter.rect_stroke(
                screen_rect.expand(expand),
                cr,
                Stroke::new(1.8 * self.viewport.zoom.sqrt().max(0.6), Color32::from_rgba_unmultiplied(255, 200, 50, stroke_alpha)),
                StrokeKind::Outside,
            );
            painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }

        // Drop shadow (rendered before node so it appears behind)
        if node.style.shadow && !node.is_frame {
            let shadow_offset = Vec2::new(3.0, 5.0) * self.viewport.zoom.sqrt();
            let shadow_rect = screen_rect.translate(shadow_offset);
            let cr = CornerRadius::same((node.style.corner_radius * self.viewport.zoom.sqrt()) as u8);
            // Multi-layer shadow for softness
            for (expand, alpha) in [(8.0_f32, 15_u8), (5.0, 25), (2.0, 40)] {
                painter.rect_filled(
                    shadow_rect.expand(expand * self.viewport.zoom.sqrt()),
                    CornerRadius::same((cr.nw as f32 + expand * 0.5) as u8),
                    Color32::from_rgba_unmultiplied(0, 0, 0, alpha),
                );
            }
        }

        // Node glow effect: multi-layer neon halo around border
        if node.style.glow && !node.is_frame {
            let gc = node.style.border_color;
            let cr = CornerRadius::same((node.style.corner_radius * self.viewport.zoom.sqrt()) as u8);
            for (expand, alpha) in [(12.0_f32, 20_u8), (7.0, 40), (3.0, 70)] {
                let s = Stroke::new(expand * self.viewport.zoom.sqrt(), Color32::from_rgba_unmultiplied(gc[0], gc[1], gc[2], alpha));
                painter.rect_stroke(screen_rect, cr, s, StrokeKind::Outside);
            }
        }

        // Group frame: translucent container with label in top-left corner
        if node.is_frame {
            let fc = node.frame_color;
            let fill = Color32::from_rgba_unmultiplied(fc[0], fc[1], fc[2], fc[3]);
            let border_col = if is_selected { self.theme.selection_color } else {
                let bc = node.style.border_color;
                Color32::from_rgba_unmultiplied(bc[0], bc[1], bc[2], bc[3])
            };
            let cr = CornerRadius::same(node.style.corner_radius as u8);
            painter.rect_filled(screen_rect, cr, fill);
            painter.rect_stroke(screen_rect, cr, Stroke::new(1.5, border_col), StrokeKind::Outside);
            // Frame label at top-left corner, outside the rect
            if let NodeKind::Shape { label, .. } = &node.kind {
                let text_col = Color32::from_rgba_unmultiplied(
                    node.style.text_color[0], node.style.text_color[1],
                    node.style.text_color[2], node.style.text_color[3],
                );
                let font_size = (node.style.font_size * self.viewport.zoom.sqrt()).clamp(9.0, 14.0);
                let label_pos = screen_rect.left_top() + Vec2::new(4.0, -font_size - 4.0);
                painter.text(label_pos, Align2::LEFT_BOTTOM, label,
                    FontId::proportional(font_size), text_col);

                // Child count badge: count nodes whose rects are contained within this frame
                if self.viewport.zoom > 0.35 {
                    let frame_canvas_rect = node.rect();
                    let child_count = self.document.nodes.iter()
                        .filter(|n| n.id != node.id && !n.is_frame && frame_canvas_rect.contains_rect(n.rect()))
                        .count();
                    if child_count > 0 {
                        let badge_text = child_count.to_string();
                        let badge_font = FontId::proportional((font_size * 0.85).clamp(8.0, 12.0));
                        let galley = painter.ctx().fonts(|f| f.layout_no_wrap(badge_text.clone(), badge_font.clone(), self.theme.text_primary));
                        let badge_w = galley.size().x + 8.0;
                        let badge_h = galley.size().y + 4.0;
                        let badge_x = label_pos.x + painter.ctx().fonts(|f| f.layout_no_wrap(label.clone(), FontId::proportional(font_size), text_col)).size().x + 6.0;
                        let badge_rect = Rect::from_min_size(
                            Pos2::new(badge_x, label_pos.y - badge_h),
                            Vec2::new(badge_w, badge_h),
                        );
                        painter.rect_filled(badge_rect, CornerRadius::same(3), self.theme.surface1.gamma_multiply(0.7));
                        painter.text(badge_rect.center(), Align2::CENTER_CENTER, &badge_text, badge_font, self.theme.text_primary);
                    }
                }
            }
            return;
        }

        // Collapsed pill: render a compact rounded rect with just the label
        if node.collapsed {
            if let NodeKind::Shape { label, .. } = &node.kind {
                let fill = to_color32(node.style.fill_color);
                let border = if is_selected { self.theme.selection_color } else { to_color32(node.style.border_color) };
                let cr = CornerRadius::same((screen_rect.height() / 2.0) as u8);
                painter.rect_filled(screen_rect, cr, fill);
                painter.rect_stroke(screen_rect, cr, Stroke::new(1.5, border), StrokeKind::Outside);
                // Collapsed indicator chevron on left + label
                let text_col = to_color32(node.style.text_color);
                painter.text(
                    screen_rect.left_center() + Vec2::new(10.0, 0.0),
                    Align2::LEFT_CENTER, "▶",
                    FontId::proportional(9.0), text_col.gamma_multiply(0.6),
                );
                painter.text(
                    screen_rect.center(),
                    Align2::CENTER_CENTER, label,
                    FontId::proportional((node.style.font_size * self.viewport.zoom.sqrt()).min(13.0)),
                    text_col,
                );
            }
            return;
        }

        match &node.kind {
            NodeKind::Shape { shape, label, .. } => {
                self.draw_shape_node(painter, screen_rect, *shape, label, &node.style, is_selected);
            }
            NodeKind::StickyNote { text, .. } => {
                self.draw_sticky_node(painter, screen_rect, text, &node.style, is_selected);
            }
            NodeKind::Entity { name, attributes } => {
                self.draw_entity_node(painter, screen_rect, name, attributes, &node.style, is_selected);
            }
            NodeKind::Text { content } => {
                self.draw_text_node(painter, screen_rect, content, &node.style, is_selected);
            }
        }

        // Description indicator (small dot in bottom-right when node has a description)
        if self.viewport.zoom > 0.4 {
            let has_desc = match &node.kind {
                NodeKind::Shape { description, .. } => !description.is_empty(),
                _ => false,
            };
            if has_desc {
                let dot_pos = Pos2::new(screen_rect.max.x - 5.0, screen_rect.max.y - 5.0);
                painter.circle_filled(dot_pos, 3.5 * self.viewport.zoom.sqrt(), self.theme.accent.gamma_multiply(0.6));
            }
        }

        // URL indicator (shown as a small 🔗 in bottom-left when node has a URL)
        if !node.url.is_empty() && self.viewport.zoom > 0.5 {
            let icon_pos = Pos2::new(screen_rect.min.x + 4.0, screen_rect.max.y - 4.0);
            painter.text(icon_pos, Align2::LEFT_BOTTOM, "🔗", FontId::proportional(9.0 * self.viewport.zoom.sqrt()), self.theme.text_dim.gamma_multiply(0.85));
        }

        // Node icon badge (shown in top-left when node has an icon)
        if !node.icon.is_empty() && self.viewport.zoom > 0.4 {
            let icon_size = (14.0 * self.viewport.zoom.sqrt()).clamp(10.0, 20.0);
            let icon_pos = Pos2::new(screen_rect.min.x + 3.0, screen_rect.min.y + 3.0);
            painter.text(icon_pos, Align2::LEFT_TOP, &node.icon,
                FontId::proportional(icon_size), self.theme.text_primary);
        }

        // Sublabel: small secondary text below node (visible at normal+ zoom)
        if !node.sublabel.is_empty() && self.viewport.zoom > 0.45 {
            let sub_font_size = (9.5 * self.viewport.zoom.sqrt()).clamp(8.0, 14.0);
            let sub_col = to_color32(node.style.text_color).gamma_multiply(0.62);
            let sub_pos = Pos2::new(screen_rect.center().x, screen_rect.max.y + sub_font_size * 0.25 + 1.0);
            // Draw subtle background pill so it's legible over canvas bg
            let sub_galley = painter.layout_no_wrap(
                node.sublabel.clone(),
                FontId::proportional(sub_font_size),
                sub_col,
            );
            let pill_rect = Rect::from_center_size(sub_pos + Vec2::new(0.0, sub_font_size * 0.5), sub_galley.size() + Vec2::new(6.0, 3.0));
            painter.rect_filled(pill_rect, CornerRadius::same(3), self.theme.canvas_bg.gamma_multiply(0.75));
            painter.text(
                sub_pos + Vec2::new(0.0, sub_font_size * 0.5),
                Align2::CENTER_CENTER,
                &node.sublabel,
                FontId::proportional(sub_font_size),
                sub_col,
            );
        }

        // Progress bar (thin filled strip at node bottom; shown when progress > 0)
        if node.progress > 0.0 && self.viewport.zoom > 0.3 {
            let bar_h = (3.0 * self.viewport.zoom).clamp(2.0, 6.0);
            let bar_rect = Rect::from_min_size(
                Pos2::new(screen_rect.min.x, screen_rect.max.y - bar_h),
                Vec2::new(screen_rect.width() * node.progress.clamp(0.0, 1.0), bar_h),
            );
            // Track (empty part)
            let track_rect = Rect::from_min_size(
                Pos2::new(screen_rect.min.x, screen_rect.max.y - bar_h),
                Vec2::new(screen_rect.width(), bar_h),
            );
            let cr = CornerRadius { sw: 3, se: 3, nw: 0, ne: 0 };
            painter.rect_filled(track_rect, cr, Color32::BLACK.gamma_multiply(0.25));
            // Filled part — green at 100%, yellow-ish at mid, red at low
            let pct = node.progress.clamp(0.0, 1.0);
            let bar_col = if pct >= 1.0 {
                Color32::from_rgb(166, 227, 161)  // green
            } else if pct >= 0.6 {
                Color32::from_rgb(249, 226, 175)  // yellow
            } else {
                Color32::from_rgb(243, 139, 168)  // red/pink
            };
            painter.rect_filled(bar_rect, cr, bar_col);
            // Percentage label
            if self.viewport.zoom > 0.6 {
                let pct_str = format!("{}%", (pct * 100.0).round() as u32);
                let lbl_size = (8.0 * self.viewport.zoom.sqrt()).clamp(7.0, 11.0);
                painter.text(
                    Pos2::new(screen_rect.max.x - 2.0, screen_rect.max.y - bar_h - 1.0),
                    Align2::RIGHT_BOTTOM,
                    &pct_str,
                    FontId::proportional(lbl_size),
                    bar_col.gamma_multiply(0.9),
                );
            }
        }

        // Lock badge (shown as a small 🔒 in top-right when node is locked)
        if node.locked && self.viewport.zoom > 0.4 {
            let icon_size = (9.0 * self.viewport.zoom.sqrt()).clamp(8.0, 14.0);
            let icon_pos = Pos2::new(screen_rect.max.x - 3.0, screen_rect.min.y + 3.0);
            painter.text(icon_pos, Align2::RIGHT_TOP, "🔒", FontId::proportional(icon_size),
                self.theme.fk_color.gamma_multiply(0.86));
        }

        // Comment bubble (shown in top-right when node has a comment)
        if !node.comment.is_empty() && self.viewport.zoom > 0.4 {
            let x_offset = if node.locked { 18.0 } else { 3.0 };
            let badge_pos = Pos2::new(screen_rect.max.x - x_offset, screen_rect.min.y - 2.0);
            let bubble_col = Color32::from_rgba_unmultiplied(249, 226, 175, 230);
            let text_col = Color32::from_rgba_unmultiplied(80, 60, 20, 255);
            let r = (8.0 * self.viewport.zoom.sqrt()).clamp(7.0, 12.0);
            painter.circle_filled(badge_pos, r, bubble_col);
            painter.circle_stroke(badge_pos, r, Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 170, 90, 200)));
            painter.text(badge_pos, Align2::CENTER_CENTER, "💬",
                FontId::proportional(r * 1.0), text_col);
        }

        // Edge connection count badge (shown when hovered)
        if is_hovered && self.viewport.zoom > 0.5 {
            let conn_count = self.document.edges.iter()
                .filter(|e| e.source.node_id == node.id || e.target.node_id == node.id)
                .count();
            if conn_count > 0 {
                let badge_pos = Pos2::new(screen_rect.max.x - 4.0, screen_rect.min.y - 4.0);
                let badge_text = conn_count.to_string();
                let badge_r = 8.0_f32 * self.viewport.zoom.sqrt();
                painter.circle_filled(badge_pos, badge_r, self.theme.accent);
                painter.text(badge_pos, Align2::CENTER_CENTER, &badge_text,
                    FontId::proportional(badge_r * 1.2), {
                    let ac = self.theme.accent;
                    to_color32(crate::app::theme::auto_contrast_text([ac.r(), ac.g(), ac.b(), 255]))
                });
            }
        }

        // Node tag badge (top-left pill)
        if let Some(tag) = node.tag {
            if self.viewport.zoom > 0.35 {
                let tag_color = to_color32(tag.color());
                // Left accent stripe — 3px wide, full node height
                let stripe_w = (3.0 * self.viewport.zoom.sqrt()).max(2.0);
                let cr = node.style.corner_radius * self.viewport.zoom.sqrt();
                let stripe_rect = Rect::from_min_max(
                    screen_rect.min,
                    Pos2::new(screen_rect.min.x + stripe_w, screen_rect.max.y),
                );
                painter.rect_filled(stripe_rect, CornerRadius { nw: cr as u8, sw: cr as u8, ne: 0, se: 0 }, tag_color.gamma_multiply(0.7));
                let label = tag.label();
                let font_size = 8.5 * self.viewport.zoom.sqrt();
                let pad_x = 4.0 * self.viewport.zoom.sqrt();
                let pad_y = 2.0 * self.viewport.zoom.sqrt();
                // Draw tag pill or dot depending on zoom
                if font_size > 4.0 {
                    let text_w = font_size * label.len() as f32 * 0.55;
                    let pill_h = font_size + pad_y * 2.0;
                    let pill_w = text_w + pad_x * 2.0;
                    let pill_rect = Rect::from_min_size(
                        Pos2::new(screen_rect.min.x + 4.0, screen_rect.min.y + 4.0),
                        Vec2::new(pill_w, pill_h),
                    );
                    painter.rect_filled(pill_rect, CornerRadius::same(pill_h as u8 / 2), tag_color);
                    painter.text(
                        pill_rect.center(),
                        Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(font_size),
                        {
                            let tc = tag_color;
                            to_color32(crate::app::theme::auto_contrast_text([tc.r(), tc.g(), tc.b(), 255]))
                        },
                    );
                } else {
                    // Tiny dot
                    painter.circle_filled(
                        Pos2::new(screen_rect.min.x + 6.0, screen_rect.min.y + 6.0),
                        4.0 * self.viewport.zoom.sqrt(),
                        tag_color,
                    );
                }
            }
        }

        // Pin indicator — diagonal stripe overlay + pin badge
        if node.pinned {
            // Subtle diagonal stripe fill to indicate "immovable"
            if self.viewport.zoom > 0.3 {
                let clipped = painter.with_clip_rect(screen_rect);
                let stripe_color = self.theme.text_dim.gamma_multiply(0.2);
                let spacing = 10.0_f32;
                let w = screen_rect.width();
                let h = screen_rect.height();
                let count = ((w + h) / spacing) as i32 + 2;
                for i in 0..count {
                    let x0 = screen_rect.min.x + i as f32 * spacing - h;
                    let y0 = screen_rect.min.y;
                    let x1 = x0 + h;
                    let y1 = screen_rect.max.y;
                    clipped.line_segment(
                        [Pos2::new(x0, y0), Pos2::new(x1, y1)],
                        Stroke::new(1.0, stripe_color),
                    );
                }
            }
            // Pin badge: small circle with 📌 in top-left
            if self.viewport.zoom > 0.4 {
                let badge_r = (7.0 * self.viewport.zoom.sqrt()).clamp(6.0, 11.0);
                let badge_pos = Pos2::new(screen_rect.min.x + badge_r + 1.0, screen_rect.min.y + badge_r + 1.0);
                painter.circle_filled(badge_pos, badge_r, Color32::from_rgba_unmultiplied(245, 194, 97, 210));
                painter.circle_stroke(badge_pos, badge_r, Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 150, 40, 200)));
                painter.text(badge_pos, Align2::CENTER_CENTER, "📌",
                    FontId::proportional(badge_r * 1.1),
                    Color32::from_rgb(40, 20, 10));
            }
        }

        // Status icon strip (top-right): 💬 comment, 🔗 url, 🔒 locked
        if self.viewport.zoom > 0.45 {
            let icon_size = (9.5 * self.viewport.zoom.sqrt()).clamp(8.0, 14.0);
            let icon_font = FontId::proportional(icon_size);
            let mut icons: Vec<&str> = Vec::new(); // glyph only
            if !node.comment.is_empty() { icons.push("💬"); }
            if !node.url.is_empty()     { icons.push("🔗"); }
            if node.locked              { icons.push("🔒"); }
            if !icons.is_empty() {
                let gap = icon_size * 1.1;
                let strip_w = icons.len() as f32 * gap;
                let strip_x = screen_rect.max.x - strip_w - 3.0;
                let strip_y = screen_rect.min.y + 3.0;

                // Dim background pill behind the icons
                let bg_rect = Rect::from_min_size(
                    Pos2::new(strip_x - 2.0, strip_y - 1.0),
                    Vec2::new(strip_w + 4.0, icon_size + 4.0),
                );
                painter.rect_filled(bg_rect, CornerRadius::same(4),
                    self.theme.shadow_medium.gamma_multiply(2.8));

                for (i, glyph) in icons.iter().enumerate() {
                    let x = strip_x + i as f32 * gap + gap * 0.5;
                    let y = strip_y + icon_size * 0.5;
                    painter.text(
                        Pos2::new(x, y),
                        Align2::CENTER_CENTER,
                        *glyph,
                        icon_font.clone(),
                        self.theme.text_primary,
                    );
                }
            }
        }

        // Ports
        // Port hit area scales with zoom so it feels consistent across zoom levels
        let port_hit = (30.0 / self.viewport.zoom.sqrt()).clamp(15.0, 60.0);
        let show_ports = self.tool == super::Tool::Connect || {
            if let Some(hover) = hover_pos {
                screen_rect.expand(port_hit).contains(hover)
            } else {
                false
            }
        };
        if show_ports {
            for side in &ALL_SIDES {
                let canvas_port = node.port_position(*side);
                let screen_port = self.viewport.canvas_to_screen(canvas_port);
                let r = PORT_RADIUS * self.viewport.zoom.sqrt();
                let port_hovered =
                    hover_pos.map_or(false, |hp| (hp - screen_port).length() < r * 3.0);

                if port_hovered {
                    let glow_r = r * 2.5;
                    painter.circle_filled(screen_port, glow_r, self.theme.accent_glow);
                    painter.circle_filled(screen_port, r * 1.3, self.theme.accent);
                    painter.circle_stroke(screen_port, r * 1.3, Stroke::new(2.0, self.theme.text_primary));
                } else {
                    painter.circle_filled(screen_port, r, self.theme.port_fill);
                    painter.circle_stroke(screen_port, r, Stroke::new(1.5, self.theme.selection_color));
                }
            }
        }
    }

    fn draw_shape_node(
        &self,
        painter: &egui::Painter,
        screen_rect: Rect,
        shape: NodeShape,
        label: &str,
        style: &NodeStyle,
        is_selected: bool,
    ) {
        let shadow_offset = Vec2::new(2.0, 3.0) * self.viewport.zoom;
        let shadow_rect = screen_rect.translate(shadow_offset);
        painter.rect_filled(
            shadow_rect,
            CornerRadius::same(4),
            self.theme.shadow_light,
        );

        let opacity = style.opacity.clamp(0.0, 1.0);
        let fill = to_color32(style.fill_color).gamma_multiply(opacity);
        let border_color = if is_selected {
            self.theme.selection_color.gamma_multiply(opacity)
        } else {
            to_color32(style.border_color).gamma_multiply(opacity)
        };
        let border_width = if is_selected {
            style.border_width.max(2.5)
        } else {
            style.border_width
        };
        let stroke = Stroke::new(border_width * self.viewport.zoom.sqrt(), border_color);

        let cr_user = (style.corner_radius * self.viewport.zoom.sqrt()) as u8;

        // Dashed border helper: draw dashed rect outline
        let draw_dashed_rect = |painter: &egui::Painter, r: Rect, stroke: Stroke| {
            let dash = 8.0_f32;
            let gap = 5.0_f32;
            let perimeter: Vec<Pos2> = vec![
                r.min, Pos2::new(r.max.x, r.min.y),
                r.max, Pos2::new(r.min.x, r.max.y), r.min,
            ];
            let mut progress = 0.0_f32;
            let mut drawing = true;
            for w in perimeter.windows(2) {
                let seg_len = (w[1] - w[0]).length();
                let mut remain = seg_len;
                let mut cur = w[0];
                while remain > 0.0 {
                    let to_flip = if drawing { dash - progress } else { gap - progress };
                    let seg_start = cur;
                    if remain >= to_flip {
                        let _t = to_flip / seg_len;
                        let end = cur + (w[1] - w[0]) * (to_flip / remain.max(0.001));
                        if drawing { painter.line_segment([seg_start, end], stroke); }
                        cur = end; remain -= to_flip; progress = 0.0; drawing = !drawing;
                    } else {
                        progress += remain; remain = 0.0;
                    }
                }
            }
        };

        // Selection glow halo — draw progressively larger, lower-opacity rings
        if is_selected && self.viewport.zoom > 0.3 {
            let glow_col = self.theme.selection_color;
            for i in 1u8..=3 {
                let expand = i as f32 * 2.5 * self.viewport.zoom.sqrt();
                let alpha = (60 - i * 18) as u8;
                let glow_rect = screen_rect.expand(expand);
                painter.rect_stroke(
                    glow_rect,
                    CornerRadius::same((cr_user as f32 + expand * 0.5) as u8),
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(glow_col.r(), glow_col.g(), glow_col.b(), alpha)),
                    StrokeKind::Outside,
                );
            }
        }

        match shape {
            NodeShape::Rectangle => {
                if style.gradient {
                    paint_gradient_rect_angle(painter, screen_rect, fill, darken(fill, 0.35), style.gradient_angle);
                } else {
                    painter.rect_filled(screen_rect, CornerRadius::same(cr_user), fill);
                }
                if style.border_dashed {
                    draw_dashed_rect(painter, screen_rect, stroke);
                } else {
                    painter.rect_stroke(screen_rect, CornerRadius::same(cr_user), stroke, StrokeKind::Outside);
                }
            }
            NodeShape::RoundedRect => {
                let r = (10.0 * self.viewport.zoom).max(style.corner_radius * self.viewport.zoom.sqrt()) as u8;
                if style.gradient {
                    // Clip gradient to rounded rect by overdrawing rounded rect mask
                    paint_gradient_rect_angle(painter, screen_rect, fill, darken(fill, 0.35), style.gradient_angle);
                    // Re-punch the corners transparent by drawing the background color in the corner arcs
                    // (simplification: draw the stroke rect over — visually correct for typical sizes)
                } else {
                    painter.rect_filled(screen_rect, CornerRadius::same(r), fill);
                }
                if style.border_dashed {
                    draw_dashed_rect(painter, screen_rect, stroke);
                } else {
                    painter.rect_stroke(
                        screen_rect,
                        CornerRadius::same(r),
                        stroke,
                        StrokeKind::Outside,
                    );
                }
            }
            NodeShape::Diamond => {
                let center = screen_rect.center();
                let hw = screen_rect.width() / 2.0;
                let hh = screen_rect.height() / 2.0;
                let points = vec![
                    Pos2::new(center.x, center.y - hh),
                    Pos2::new(center.x + hw, center.y),
                    Pos2::new(center.x, center.y + hh),
                    Pos2::new(center.x - hw, center.y),
                ];
                painter.add(egui::Shape::convex_polygon(points, fill, stroke));
            }
            NodeShape::Circle => {
                let center = screen_rect.center();
                let radius = screen_rect.width().min(screen_rect.height()) / 2.0;
                painter.circle_filled(center, radius, fill);
                painter.circle_stroke(center, radius, stroke);
            }
            NodeShape::Parallelogram => {
                let skew = screen_rect.width() * 0.15;
                let points = vec![
                    Pos2::new(screen_rect.min.x + skew, screen_rect.min.y),
                    Pos2::new(screen_rect.max.x, screen_rect.min.y),
                    Pos2::new(screen_rect.max.x - skew, screen_rect.max.y),
                    Pos2::new(screen_rect.min.x, screen_rect.max.y),
                ];
                painter.add(egui::Shape::convex_polygon(points, fill, stroke));
            }
            NodeShape::Hexagon => {
                let cx = screen_rect.center().x;
                let cy = screen_rect.center().y;
                let hw = screen_rect.width() / 2.0;
                let hh = screen_rect.height() / 2.0;
                let inset = hw * 0.3; // horizontal flat sides
                let points = vec![
                    Pos2::new(cx - hw,      cy),
                    Pos2::new(cx - inset,   cy - hh),
                    Pos2::new(cx + inset,   cy - hh),
                    Pos2::new(cx + hw,      cy),
                    Pos2::new(cx + inset,   cy + hh),
                    Pos2::new(cx - inset,   cy + hh),
                ];
                painter.add(egui::Shape::convex_polygon(points, fill, stroke));
            }
            NodeShape::Connector => {
                // Pill shape: fully rounded capsule with subtle fill
                let radius = screen_rect.height() / 2.0;
                let connector_fill = Color32::from_rgba_unmultiplied(
                    fill.r(), fill.g(), fill.b(), 80,
                );
                painter.rect_filled(
                    screen_rect,
                    CornerRadius::same(radius as u8),
                    connector_fill,
                );
                // Dashed border drawn as two solid arcs + dashed line segments
                painter.rect_stroke(
                    screen_rect,
                    CornerRadius::same(radius as u8),
                    Stroke::new(
                        border_width * self.viewport.zoom.sqrt(),
                        border_color.linear_multiply(0.8),
                    ),
                    StrokeKind::Outside,
                );
                // Small diamond accent on left edge
                let diamond_size = 5.0 * self.viewport.zoom.sqrt();
                let left_center = Pos2::new(screen_rect.min.x - diamond_size * 0.5, screen_rect.center().y);
                let diamond_pts = vec![
                    Pos2::new(left_center.x, left_center.y - diamond_size),
                    Pos2::new(left_center.x + diamond_size, left_center.y),
                    Pos2::new(left_center.x, left_center.y + diamond_size),
                    Pos2::new(left_center.x - diamond_size, left_center.y),
                ];
                painter.add(egui::Shape::convex_polygon(
                    diamond_pts,
                    border_color,
                    Stroke::NONE,
                ));
            }
            NodeShape::Triangle => {
                // Isosceles triangle: apex at top-center, base at bottom
                let apex   = Pos2::new(screen_rect.center().x, screen_rect.min.y);
                let bl     = Pos2::new(screen_rect.min.x,      screen_rect.max.y);
                let br     = Pos2::new(screen_rect.max.x,      screen_rect.max.y);
                let points = vec![apex, br, bl];
                painter.add(egui::Shape::convex_polygon(points.clone(), fill, stroke));
            }
            NodeShape::Callout => {
                // Speech bubble: rounded rect body + small triangular tail at bottom-left
                let cr = egui::CornerRadius::same(8);
                painter.rect(screen_rect, cr, fill, stroke, egui::StrokeKind::Middle);
                // Tail: small triangle pointing down-left from bottom-left corner
                let tail_w = (screen_rect.width() * 0.15).max(12.0);
                let tail_h = (screen_rect.height() * 0.28).max(10.0);
                let tail_base_x = screen_rect.min.x + tail_w * 0.6;
                let tail_tip_x  = screen_rect.min.x - tail_w * 0.1;
                let tail_y      = screen_rect.max.y;
                let tail_pts = vec![
                    Pos2::new(screen_rect.min.x + 4.0, tail_y),        // left attach
                    Pos2::new(tail_base_x,             tail_y),        // right attach
                    Pos2::new(tail_tip_x,  tail_y + tail_h),           // tip
                ];
                painter.add(egui::Shape::convex_polygon(tail_pts, fill, stroke));
            }
        }

        // Semantic watermark icon — drawn behind label text at low opacity
        // Only visible at moderate zoom and when the node is large enough
        if self.viewport.zoom > 0.5 && screen_rect.area() > 1200.0 {
            if let Some(icon) = semantic_icon_for_label(label) {
                let icon_size = (screen_rect.height() * 0.55).clamp(14.0, 48.0);
                let icon_alpha = (40.0 * opacity) as u8; // very subtle
                painter.text(
                    screen_rect.center() + Vec2::new(screen_rect.width() * 0.28, 0.0),
                    Align2::CENTER_CENTER,
                    icon,
                    FontId::proportional(icon_size),
                    self.theme.text_primary.gamma_multiply(icon_alpha as f32 / 255.0),
                );
            }
        }

        // Adaptive font: scale with zoom, floor at 7px for readability, fade near LOD boundary
        let font_size_raw = style.font_size * self.viewport.zoom;
        let font_size = font_size_raw.clamp(7.0, 72.0);
        // At low zoom (< 0.5), fade text toward 0 so it disappears smoothly before LOD switch
        let text_fade = (self.viewport.zoom / 0.4).clamp(0.0, 1.0);
        let text_color = to_color32(style.text_color).gamma_multiply(opacity * text_fade);
        if font_size > 4.0 && text_fade > 0.05 && !label.is_empty() {
            let font = match shape {
                NodeShape::Connector => FontId::monospace(font_size * 0.88),
                _ => FontId::proportional(font_size),
            };
            let pad = (6.0 * self.viewport.zoom).min(12.0);
            let max_text_w = (screen_rect.width() - pad * 2.0).max(10.0);
            let max_text_h = (screen_rect.height() - pad * 2.0).max(6.0);
            // Build display label with bold/italic markers for visual hint
            let base_label: std::borrow::Cow<str> = match (style.bold, style.italic) {
                (true, true)  => std::borrow::Cow::Owned(format!("𝘽 𝘐 {}", label)),
                (true, false) => std::borrow::Cow::Owned(format!("𝗕 {}", label)),
                (false, true) => std::borrow::Cow::Owned(format!("𝘐 {}", label)),
                (false, false) => std::borrow::Cow::Borrowed(label),
            };
            // For narrow nodes (< 50 screen px), use single-line truncated label
            // to avoid ugly 1-2 character word-wrap lines.
            let display_label: std::borrow::Cow<str> = if max_text_w < 50.0 {
                let approx_chars = (max_text_w / (font_size * 0.52)).floor().max(1.0) as usize;
                let base = base_label.as_ref();
                let char_count = base.chars().count();
                if char_count > approx_chars && approx_chars >= 2 {
                    let truncated: String = base.chars().take(approx_chars.saturating_sub(1)).collect();
                    std::borrow::Cow::Owned(format!("{}…", truncated))
                } else {
                    base_label
                }
            } else {
                base_label
            };
            // Apply bold/italic via LayoutJob
            let galley = if style.bold {
                let bold_font = FontId::proportional(font_size * 1.06);
                painter.layout(display_label.into_owned(), bold_font, text_color, max_text_w)
            } else {
                painter.layout(display_label.into_owned(), font, text_color, max_text_w)
            };
            let text_x = match style.text_align {
                crate::model::TextAlign::Center => screen_rect.center().x - galley.size().x / 2.0,
                crate::model::TextAlign::Left   => screen_rect.min.x + pad,
                crate::model::TextAlign::Right  => screen_rect.max.x - pad - galley.size().x,
            };
            let text_y = match style.text_valign {
                crate::model::TextVAlign::Top    => screen_rect.min.y + pad,
                crate::model::TextVAlign::Middle => screen_rect.center().y - galley.size().y / 2.0,
                crate::model::TextVAlign::Bottom => screen_rect.max.y - pad - galley.size().y,
            };
            let text_pos = Pos2::new(text_x, text_y);
            // Clip text to node interior so it never overflows the shape
            let clip_rect = screen_rect.shrink(pad * 0.5);
            let clipped_painter = painter.with_clip_rect(clip_rect);
            clipped_painter.galley(text_pos, galley.clone(), Color32::TRANSPARENT);
            // Show fade-out ellipsis if text overflows height
            if galley.size().y > max_text_h + 2.0 {
                let fade_y = screen_rect.center().y + max_text_h / 2.0 - font_size * 0.5;
                let ellipsis_pos = Pos2::new(screen_rect.center().x, fade_y);
                // Fade-out strip: gradient over bottom 1.5 lines
                let fade_h = font_size * 1.2;
                let fade_rect = Rect::from_min_max(
                    Pos2::new(clip_rect.min.x, ellipsis_pos.y - fade_h * 0.3),
                    Pos2::new(clip_rect.max.x, clip_rect.max.y),
                );
                // Draw node-fill-colored rect to mask overflow (simulates clip fade)
                let fill_mask = to_color32(style.fill_color).gamma_multiply(opacity * 0.88);
                painter.rect_filled(fade_rect, CornerRadius::ZERO, fill_mask);
                // Ellipsis dots
                painter.text(ellipsis_pos, Align2::CENTER_CENTER, "…",
                    FontId::proportional(font_size * 0.9), text_color.gamma_multiply(0.85));
            }
        }
    }

    fn draw_sticky_node(
        &self,
        painter: &egui::Painter,
        screen_rect: Rect,
        text: &str,
        style: &NodeStyle,
        is_selected: bool,
    ) {
        let shadow_offset = Vec2::new(3.0, 4.0) * self.viewport.zoom;
        let shadow_rect = screen_rect.translate(shadow_offset);
        painter.rect_filled(
            shadow_rect,
            CornerRadius::same(4),
            self.theme.shadow_medium,
        );

        let fill = to_color32(style.fill_color);
        let corner = CornerRadius::same((8.0 * self.viewport.zoom) as u8);
        painter.rect_filled(screen_rect, corner, fill);

        if is_selected {
            painter.rect_stroke(
                screen_rect,
                corner,
                Stroke::new(2.5 * self.viewport.zoom.sqrt(), self.theme.selection_color),
                StrokeKind::Outside,
            );
        }

        // Folded corner ("dog ear") in bottom-right — proportional to node height
        let fold_size = (screen_rect.height() * 0.12).clamp(6.0, 20.0);
        if fold_size >= 5.0 {
            let br = screen_rect.max;
            let fold_tl = Pos2::new(br.x - fold_size, br.y - fold_size);
            // Cover the corner with the base fill (to "cut" it visually)
            painter.add(egui::Shape::convex_polygon(
                vec![fold_tl, Pos2::new(br.x, fold_tl.y), br, Pos2::new(fold_tl.x, br.y)],
                fill,
                Stroke::NONE,
            ));
            // Background corner triangle (matching canvas/shadow color)
            let bg = self.theme.surface0.gamma_multiply(0.8);
            painter.add(egui::Shape::convex_polygon(
                vec![fold_tl, Pos2::new(br.x, fold_tl.y), br],
                bg,
                Stroke::NONE,
            ));
            // Fold crease shadow
            let crease_color = darken(fill, 0.28);
            painter.add(egui::Shape::convex_polygon(
                vec![fold_tl, Pos2::new(fold_tl.x, br.y), br],
                crease_color,
                Stroke::NONE,
            ));
            // Fold crease line
            painter.line_segment(
                [fold_tl, Pos2::new(br.x, fold_tl.y)],
                Stroke::new(0.8, darken(fill, 0.45)),
            );
        }

        let text_color = to_color32(style.text_color);
        let font_size = style.font_size * self.viewport.zoom;
        if font_size > 4.0 && !text.is_empty() {
            let padding = 10.0 * self.viewport.zoom;
            let text_rect = screen_rect.shrink(padding);
            // Clip text so it doesn't flow into the dog-ear fold area
            let fold_size = (screen_rect.height() * 0.12).clamp(6.0, 20.0);
            let clipped_painter = painter.with_clip_rect(Rect::from_min_max(
                text_rect.min,
                Pos2::new(text_rect.max.x - fold_size * 0.5, text_rect.max.y),
            ));
            let galley = painter.layout(
                text.to_string(),
                FontId::proportional(font_size),
                text_color,
                text_rect.width() - fold_size * 0.5,
            );
            let text_pos = Pos2::new(text_rect.min.x, text_rect.min.y);
            clipped_painter.galley(text_pos, galley, Color32::TRANSPARENT);
        }
    }

    fn draw_entity_node(
        &self,
        painter: &egui::Painter,
        screen_rect: Rect,
        name: &str,
        attributes: &[EntityAttribute],
        style: &NodeStyle,
        is_selected: bool,
    ) {
        let shadow_offset = Vec2::new(2.0, 3.0) * self.viewport.zoom;
        let shadow_rect = screen_rect.translate(shadow_offset);
        painter.rect_filled(
            shadow_rect,
            CornerRadius::same(4),
            self.theme.shadow_light,
        );

        let fill = to_color32(style.fill_color);
        let border_color = if is_selected {
            self.theme.selection_color
        } else {
            to_color32(style.border_color)
        };
        let border_width = if is_selected {
            style.border_width.max(2.5)
        } else {
            style.border_width
        };
        let stroke = Stroke::new(border_width * self.viewport.zoom.sqrt(), border_color);
        let zoom = self.viewport.zoom;

        painter.rect_filled(screen_rect, CornerRadius::same(3), fill);
        painter.rect_stroke(screen_rect, CornerRadius::same(3), stroke, StrokeKind::Outside);

        // Header
        let header_h = ENTITY_HEADER_HEIGHT * zoom;
        let header_rect = Rect::from_min_size(
            screen_rect.min,
            Vec2::new(screen_rect.width(), header_h),
        );
        let header_color = to_color32(style.border_color);
        painter.rect_filled(
            header_rect,
            CornerRadius {
                nw: 3,
                ne: 3,
                sw: 0,
                se: 0,
            },
            header_color,
        );

        let divider_y = screen_rect.min.y + header_h;
        painter.line_segment(
            [
                Pos2::new(screen_rect.min.x, divider_y),
                Pos2::new(screen_rect.max.x, divider_y),
            ],
            Stroke::new(1.0, border_color),
        );

        let font_size = (style.font_size + 1.0) * zoom;
        if font_size > 4.0 {
            painter.text(
                header_rect.center(),
                Align2::CENTER_CENTER,
                name,
                FontId::proportional(font_size),
                {
                    let hc = to_color32(style.fill_color);
                    to_color32(crate::app::theme::auto_contrast_text([hc.r(), hc.g(), hc.b(), 255]))
                },
            );
        }

        // Attributes
        let row_h = ENTITY_ROW_HEIGHT * zoom;
        let attr_font = style.font_size * zoom * 0.9;
        let text_color = to_color32(style.text_color);
        let pk_color = self.theme.accent;
        let fk_color = self.theme.fk_color;

        if attr_font > 3.0 {
            for (i, attr) in attributes.iter().enumerate() {
                let row_y = divider_y + (i as f32) * row_h;
                let row_center_y = row_y + row_h / 2.0;

                if i > 0 {
                    painter.line_segment(
                        [
                            Pos2::new(screen_rect.min.x + 4.0, row_y),
                            Pos2::new(screen_rect.max.x - 4.0, row_y),
                        ],
                        Stroke::new(0.5, self.theme.row_divider),
                    );
                }

                let left_x = screen_rect.min.x + 6.0 * zoom;

                if attr.is_primary_key {
                    painter.text(
                        Pos2::new(left_x, row_center_y),
                        Align2::LEFT_CENTER,
                        "PK",
                        FontId::monospace(attr_font * 0.7),
                        pk_color,
                    );
                } else if attr.is_foreign_key {
                    painter.text(
                        Pos2::new(left_x, row_center_y),
                        Align2::LEFT_CENTER,
                        "FK",
                        FontId::monospace(attr_font * 0.7),
                        fk_color,
                    );
                }

                let name_x = left_x + 22.0 * zoom;
                painter.text(
                    Pos2::new(name_x, row_center_y),
                    Align2::LEFT_CENTER,
                    &attr.name,
                    FontId::proportional(attr_font),
                    text_color,
                );

                let type_x = screen_rect.max.x - 6.0 * zoom;
                painter.text(
                    Pos2::new(type_x, row_center_y),
                    Align2::RIGHT_CENTER,
                    &attr.attr_type,
                    FontId::monospace(attr_font * 0.85),
                    self.theme.text_dim,
                );
            }

            if attributes.is_empty() {
                let row_center_y = divider_y + row_h / 2.0;
                painter.text(
                    Pos2::new(screen_rect.center().x, row_center_y),
                    Align2::CENTER_CENTER,
                    "no attributes",
                    FontId::proportional(attr_font * 0.85),
                    self.theme.text_dim,
                );
            }
        }
    }

    fn draw_text_node(
        &self,
        painter: &egui::Painter,
        screen_rect: Rect,
        content: &str,
        style: &NodeStyle,
        is_selected: bool,
    ) {
        if is_selected {
            painter.rect_stroke(
                screen_rect,
                CornerRadius::same(2),
                Stroke::new(1.5, self.theme.accent_select_light),
                StrokeKind::Outside,
            );
        }

        let text_color = to_color32(style.text_color);
        let font_size = style.font_size * self.viewport.zoom;
        if font_size > 4.0 && !content.is_empty() {
            let galley = painter.layout(
                content.to_string(),
                FontId::proportional(font_size),
                text_color,
                screen_rect.width(),
            );
            let text_pos = Pos2::new(screen_rect.min.x, screen_rect.min.y);
            painter.galley(text_pos, galley, Color32::TRANSPARENT);
        }
    }

    // --- Edge rendering ---

    pub(crate) fn draw_edge(
        &self,
        edge: &Edge,
        painter: &egui::Painter,
        node_idx: &std::collections::HashMap<NodeId, usize>,
        hover_canvas_pos: Option<egui::Pos2>,
        // Extra perpendicular bend to separate parallel edges (0.0 = none).
        extra_bend: f32,
    ) {
        let src_node = node_idx
            .get(&edge.source.node_id)
            .and_then(|&i| self.document.nodes.get(i));
        let tgt_node = node_idx
            .get(&edge.target.node_id)
            .and_then(|&i| self.document.nodes.get(i));
        let (src_node, tgt_node) = match (src_node, tgt_node) {
            (Some(s), Some(t)) => (s, t),
            _ => return,
        };

        // Self-loop: same source and target node — draw a circular arc above the node
        if edge.source.node_id == edge.target.node_id {
            let is_selected = self.selection.contains_edge(&edge.id);
            let edge_color = to_color32(edge.style.color);
            let width = edge.style.width * self.viewport.zoom.sqrt();
            let node_rect = {
                let tl = self.viewport.canvas_to_screen(src_node.pos());
                let sz = src_node.size_vec() * self.viewport.zoom;
                Rect::from_min_size(tl, sz)
            };
            // Arc center offset above top-right corner
            let arc_cx = node_rect.max.x - node_rect.width() * 0.2;
            let arc_cy = node_rect.min.y - node_rect.height() * 0.35;
            let radius = node_rect.width() * 0.3 + 8.0;
            let stroke = Stroke::new(if is_selected { width * 1.4 } else { width },
                if is_selected { self.theme.selection_color } else { edge_color });
            // Draw a circular arc (approximated with segments)
            let steps = 24_usize;
            let start_angle = std::f32::consts::FRAC_PI_2; // start from bottom
            let end_angle = start_angle + std::f32::consts::PI * 2.0 * 0.75; // 270° arc
            let mut prev = Pos2::new(arc_cx + radius * start_angle.cos(), arc_cy + radius * start_angle.sin());
            for i in 1..=steps {
                let a = start_angle + (end_angle - start_angle) * i as f32 / steps as f32;
                let pt = Pos2::new(arc_cx + radius * a.cos(), arc_cy + radius * a.sin());
                painter.line_segment([prev, pt], stroke);
                prev = pt;
            }
            // Arrowhead at end
            let a_prev_angle = start_angle + (end_angle - start_angle) * (steps - 1) as f32 / steps as f32;
            let arrow_from = Pos2::new(arc_cx + radius * a_prev_angle.cos(), arc_cy + radius * a_prev_angle.sin());
            self.draw_arrow_head(painter, arrow_from, prev, edge_color, width, edge.style.arrow_head);
            return;
        }

        let src = self
            .viewport
            .canvas_to_screen(src_node.port_position(edge.source.side));
        let tgt = self
            .viewport
            .canvas_to_screen(tgt_node.port_position(edge.target.side));

        let is_selected = self.selection.contains_edge(&edge.id);

        // Control points (used for both hover detection and drawing)
        let offset = 60.0 * self.viewport.zoom;
        let (mut cp1, mut cp2) = control_points_for_side(src, tgt, edge.source.side, offset);

        // Apply curve bend (user-defined) plus extra_bend (parallel-edge auto-offset)
        let total_bend = edge.style.curve_bend + extra_bend;
        if total_bend.abs() > 0.02 {
            let dir = if (tgt - src).length() > 1.0 { (tgt - src).normalized() } else { Vec2::X };
            let perp = Vec2::new(-dir.y, dir.x);
            let bend_screen = total_bend * self.viewport.zoom;
            cp1 = cp1 + perp * bend_screen;
            cp2 = cp2 + perp * bend_screen;
        }

        // Edge draw-in animation: if this edge was recently created, clip the bezier
        let draw_end_t = if let Some(&birth) = self.edge_birth_times.get(&edge.id) {
            let now = painter.ctx().input(|i| i.time);
            let age = (now - birth) as f32;
            let duration = 0.30_f32;
            if age < duration {
                // Ease-out: t^0.4 gives fast start, slow end — "inking in" feel
                (age / duration).powf(0.45)
            } else {
                1.0
            }
        } else {
            1.0
        };

        // Hover detection: check if cursor is close to bezier curve
        let is_hovered = !is_selected && hover_canvas_pos.map(|hp| {
            let hp_screen = self.viewport.canvas_to_screen(hp);
            let threshold = 14.0;
            (0..=20).any(|i| {
                let t = i as f32 / 20.0;
                let p = cubic_bezier_point(src, cp1, cp2, tgt, t);
                (hp_screen - p).length() < threshold
            })
        }).unwrap_or(false);

        let edge_color = if is_selected {
            self.theme.selection_color
        } else if is_hovered {
            to_color32(edge.style.color).gamma_multiply(1.6)
        } else {
            to_color32(edge.style.color)
        };
        let base_width = edge.style.width * self.viewport.zoom.sqrt();
        let width = if is_selected {
            base_width.max(3.0)
        } else if is_hovered {
            base_width * 1.4
        } else {
            base_width
        };

        if is_selected {
            let glow = egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt],
                false,
                Color32::TRANSPARENT,
                Stroke::new(width + 6.0, self.theme.accent_select_bg),
            );
            painter.add(glow);
        }

        // Hover glow: soft warm halo under hovered edge
        if is_hovered {
            let halo_color = self.theme.fk_color.gamma_multiply(0.18);
            let halo_color2 = self.theme.fk_color.gamma_multiply(0.31);
            let halo_outer = egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt], false, Color32::TRANSPARENT,
                Stroke::new(width + 12.0, halo_color),
            );
            let halo_inner = egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt], false, Color32::TRANSPARENT,
                Stroke::new(width + 5.0, halo_color2),
            );
            painter.add(halo_outer);
            painter.add(halo_inner);
        }

        // Edge glow effect: draw a wider, semi-transparent halo beneath the edge
        if edge.style.glow && !is_selected {
            let glow_color = Color32::from_rgba_unmultiplied(
                edge_color.r(), edge_color.g(), edge_color.b(), 60,
            );
            let glow_shape = egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt],
                false,
                Color32::TRANSPARENT,
                Stroke::new(width + 10.0, glow_color),
            );
            painter.add(glow_shape);
            let glow_shape2 = egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt],
                false,
                Color32::TRANSPARENT,
                Stroke::new(width + 5.0, Color32::from_rgba_unmultiplied(
                    edge_color.r(), edge_color.g(), edge_color.b(), 100,
                )),
            );
            painter.add(glow_shape2);
        }

        if edge.style.dashed || edge.style.animated {
            // Approximate dashed/animated edge by sampling the bezier and drawing alternating segments
            let dash = (10.0 * self.viewport.zoom.sqrt()).max(3.0);
            let gap = (6.0 * self.viewport.zoom.sqrt()).max(2.0);
            // For animated edges: shift the starting progress by time to create flow effect
            let time_offset = if edge.style.animated {
                let t = painter.ctx().input(|i| i.time) as f32;
                painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
                let period = dash + gap;
                (t * period * 1.5) % period
            } else {
                0.0
            };
            let steps = 80;
            let pts: Vec<egui::Pos2> = (0..=steps)
                .map(|i| cubic_bezier_point(src, cp1, cp2, tgt, i as f32 / steps as f32))
                .collect();
            let mut drawing = true;
            let mut seg_start = pts[0];
            let mut progress = time_offset;
            // Skip initial gap for animated offset
            if progress > 0.0 && progress < gap {
                drawing = false;
            } else if progress >= gap {
                progress -= gap;
            }
            for i in 1..pts.len() {
                let seg_len = (pts[i] - pts[i - 1]).length();
                let mut remaining = seg_len;
                let mut cur = pts[i - 1];
                while remaining > 0.0 {
                    let to_flip = if drawing { dash - progress } else { gap - progress };
                    if remaining >= to_flip {
                        let _t = to_flip / seg_len;
                        let end = cur + (pts[i] - pts[i - 1]) * (to_flip / remaining.max(0.001));
                        if drawing {
                            painter.line_segment([seg_start, end], Stroke::new(width, edge_color));
                        }
                        cur = end;
                        remaining -= to_flip;
                        progress = 0.0;
                        drawing = !drawing;
                        seg_start = cur;
                    } else {
                        progress += remaining;
                        if drawing { seg_start = pts[i - 1]; }
                        remaining = 0.0;
                    }
                }
                if drawing { seg_start = pts[i - 1]; }
            }
            if drawing {
                painter.line_segment([seg_start, *pts.last().unwrap()], Stroke::new(width, edge_color));
            }
        } else if edge.style.orthogonal {
            // Orthogonal routing with rounded corners (8px radius)
            let mid_x = (src.x + tgt.x) / 2.0;
            let mid_y = (src.y + tgt.y) / 2.0;
            let pts: Vec<Pos2> = match edge.source.side {
                PortSide::Right | PortSide::Left => vec![src, Pos2::new(mid_x, src.y), Pos2::new(mid_x, tgt.y), tgt],
                PortSide::Top | PortSide::Bottom => vec![src, Pos2::new(src.x, mid_y), Pos2::new(tgt.x, mid_y), tgt],
            };
            let r = (8.0 * self.viewport.zoom.sqrt()).min(20.0); // corner radius in screen px
            // Draw segments with rounded elbows at interior corners
            let n = pts.len();
            for i in 0..n - 1 {
                let p0 = pts[i];
                let p1 = pts[i + 1];
                let seg_dir = (p1 - p0).normalized();
                // Start of this segment: skip radius if not first
                let seg_start = if i > 0 { p0 + seg_dir * r } else { p0 };
                // End of this segment: skip radius if not last
                let seg_end = if i < n - 2 { p1 - seg_dir * r } else { p1 };
                if (seg_end - seg_start).length() > 0.5 {
                    painter.line_segment([seg_start, seg_end], Stroke::new(width, edge_color));
                }
                // Rounded elbow at p1 (interior corner)
                if i < n - 2 {
                    let next_dir = (pts[i + 2] - p1).normalized();
                    // Draw a small quadratic arc from (p1 - r*seg_dir) through p1 to (p1 + r*next_dir)
                    let q0 = p1 - seg_dir * r;
                    let q2 = p1 + next_dir * r;
                    // Approximate with 5 line segments
                    let steps = 5_usize;
                    let mut prev = q0;
                    for step in 1..=steps {
                        let t = step as f32 / steps as f32;
                        // Quadratic bezier: B(t) = (1-t)^2 * q0 + 2(1-t)t * p1 + t^2 * q2
                        let s = 1.0 - t;
                        let pt = Pos2::new(
                            s * s * q0.x + 2.0 * s * t * p1.x + t * t * q2.x,
                            s * s * q0.y + 2.0 * s * t * p1.y + t * t * q2.y,
                        );
                        painter.line_segment([prev, pt], Stroke::new(width, edge_color));
                        prev = pt;
                    }
                }
            }
        } else if draw_end_t < 1.0 {
            // Draw-in animation: polyline up to draw_end_t fraction of the bezier
            let steps = 24_usize;
            let end_step = ((draw_end_t * steps as f32) as usize).clamp(1, steps);
            let pts: Vec<Pos2> = (0..=end_step)
                .map(|i| cubic_bezier_point(src, cp1, cp2, tgt, i as f32 / steps as f32))
                .collect();
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], Stroke::new(width, edge_color));
            }
        } else if !is_selected && !is_hovered {
            // Gradient edge: tint progressively from source node fill to target node fill
            // blended at 30% with the edge's own color for a subtle directional cue
            let src_fill = to_color32(src_node.style.fill_color);
            let tgt_fill = to_color32(tgt_node.style.fill_color);
            let steps = 14_usize;
            let mut prev_pt = src;
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let pt = cubic_bezier_point(src, cp1, cp2, tgt, t);
                let seg_t = (2.0 * i as f32 - 1.0) / (2.0 * steps as f32); // midpoint t
                let fill_tint = lerp_color(src_fill, tgt_fill, seg_t);
                let seg_color = lerp_color(edge_color, fill_tint, 0.28);
                painter.line_segment([prev_pt, pt], Stroke::new(width, seg_color));
                prev_pt = pt;
            }
        } else if self.viewport.zoom < 0.25 {
            // LOD: very low zoom — straight line for performance
            painter.line_segment([src, tgt], Stroke::new(width, edge_color));
        } else {
            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt],
                false,
                Color32::TRANSPARENT,
                Stroke::new(width, edge_color),
            );
            painter.add(bezier);
        }

        // Endpoints
        if edge.source_cardinality != Cardinality::None {
            self.draw_crow_foot(
                painter,
                cp1,
                src,
                edge.source_cardinality,
                edge_color,
                width,
            );
        }
        if edge.target_cardinality != Cardinality::None {
            self.draw_crow_foot(
                painter,
                cp2,
                tgt,
                edge.target_cardinality,
                edge_color,
                width,
            );
        } else {
            self.draw_arrow_head(painter, cp2, tgt, edge_color, width, edge.style.arrow_head);
        }

        // Source dot — small circle at the source endpoint for directionality cue
        if self.viewport.zoom > 0.45 && edge.source_cardinality == Cardinality::None {
            let dot_r = (2.0 * self.viewport.zoom.sqrt()).clamp(1.5, 4.0);
            let dot_color = if is_selected {
                self.theme.selection_color.gamma_multiply(0.8)
            } else {
                edge_color.gamma_multiply(0.6)
            };
            painter.circle_filled(src, dot_r, dot_color);
            painter.circle_stroke(src, dot_r, Stroke::new(0.5, edge_color.gamma_multiply(0.3)));
        }

        // Port pulse rings: animated expanding rings at src/tgt when edge is hovered
        if is_hovered && self.viewport.zoom > 0.35 {
            let t = painter.ctx().input(|i| i.time) as f32;
            // Two rings out of phase for a continuous pulse feel
            for phase in [0.0_f32, 0.5] {
                let cycle = ((t * 1.2 + phase) % 1.0) as f32; // 0→1 loop
                let ring_r = 6.0 + cycle * 14.0;
                let alpha = ((1.0 - cycle) * 130.0) as u8;
                let ring_color = Color32::from_rgba_unmultiplied(
                    edge_color.r(), edge_color.g(), edge_color.b(), alpha);
                painter.circle_stroke(src, ring_r, Stroke::new(1.2, ring_color));
                painter.circle_stroke(tgt, ring_r, Stroke::new(1.2, ring_color));
            }
            painter.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        }

        // Edge label (skip at very low zoom — illegible below ~0.45)
        if !edge.label.is_empty() && self.viewport.zoom > 0.45 {
            let mid = cubic_bezier_point(src, cp1, cp2, tgt, 0.5);
            let font_size = (12.0 * self.viewport.zoom).clamp(8.0, 24.0);
            if font_size > 4.0 {
                let galley = painter.layout_no_wrap(
                    edge.label.clone(),
                    FontId::proportional(font_size),
                    edge_color,
                );
                let text_rect = Rect::from_center_size(mid, galley.size()).expand2(Vec2::new(5.0, 3.0));
                // Subtle drop shadow
                painter.rect_filled(
                    text_rect.translate(Vec2::new(1.0, 1.5)),
                    CornerRadius::same(5),
                    self.theme.shadow_medium,
                );
                // Background pill
                painter.rect_filled(text_rect, CornerRadius::same(5), self.theme.edge_label_bg);
                // Border on selected
                if is_selected {
                    painter.rect_stroke(text_rect, CornerRadius::same(5),
                        Stroke::new(1.0, self.theme.accent.gamma_multiply(0.6)), StrokeKind::Outside);
                }
                painter.text(mid, Align2::CENTER_CENTER, &edge.label,
                    FontId::proportional(font_size), edge_color);
            }
        }

        // Edge comment bubble: show annotation as speech bubble above midpoint when hovered
        if is_hovered && !edge.comment.is_empty() && self.viewport.zoom > 0.3 {
            let mid = cubic_bezier_point(src, cp1, cp2, tgt, 0.5);
            let bubble_font = (11.5 * self.viewport.zoom).clamp(9.0, 18.0);
            let note_text = format!("💬 {}", edge.comment);
            let galley = painter.layout_no_wrap(
                note_text.clone(),
                FontId::proportional(bubble_font),
                Color32::from_rgb(200, 200, 220),
            );
            let offset_y = -(20.0 + galley.size().y * 0.5 + 6.0) * self.viewport.zoom.sqrt();
            let bubble_center = mid + Vec2::new(0.0, offset_y);
            let pad = Vec2::new(8.0, 5.0);
            let bubble_rect = Rect::from_center_size(bubble_center, galley.size() + pad * 2.0);
            // Drop shadow
            painter.rect_filled(
                bubble_rect.translate(Vec2::new(1.5, 2.0)),
                CornerRadius::same(6),
                Color32::from_black_alpha(60),
            );
            // Bubble background
            painter.rect_filled(
                bubble_rect,
                CornerRadius::same(6),
                Color32::from_rgba_unmultiplied(30, 30, 46, 230),
            );
            painter.rect_stroke(
                bubble_rect,
                CornerRadius::same(6),
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(137, 180, 250, 120)),
                StrokeKind::Inside,
            );
            // Little tail pointing down toward edge
            let tail_tip = Pos2::new(bubble_center.x, bubble_rect.max.y + 5.0 * self.viewport.zoom.sqrt());
            let tail_base_y = bubble_rect.max.y;
            let hw = 4.0;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(bubble_center.x - hw, tail_base_y),
                    Pos2::new(bubble_center.x + hw, tail_base_y),
                    tail_tip,
                ],
                Color32::from_rgba_unmultiplied(30, 30, 46, 230),
                Stroke::NONE,
            ));
            painter.text(
                bubble_center,
                Align2::CENTER_CENTER,
                &note_text,
                FontId::proportional(bubble_font),
                Color32::from_rgb(200, 200, 220),
            );
        }

        // Direction tick-marks at 25% and 75% along selected edges
        if is_selected && !edge.style.orthogonal {
            for t in [0.25_f32, 0.75] {
                let p = cubic_bezier_point(src, cp1, cp2, tgt, t);
                // Derivative direction: forward difference
                let p_next = cubic_bezier_point(src, cp1, cp2, tgt, (t + 0.02).min(1.0));
                let dir = (p_next - p).normalized();
                let perp = Vec2::new(-dir.y, dir.x);
                let tick_len = 5.0 * self.viewport.zoom.sqrt();
                let tick_stroke = Stroke::new(width * 0.8, edge_color.gamma_multiply(0.6));
                painter.line_segment([p + perp * tick_len, p - perp * tick_len], tick_stroke);
            }
        }

        // Curve bend drag handle (shown on selected non-orthogonal edges)
        if is_selected && !edge.style.orthogonal {
            let handle_pos = cubic_bezier_point(src, cp1, cp2, tgt, 0.5);
            let r = 5.0_f32;
            painter.circle_filled(handle_pos, r + 2.0, self.theme.accent_glow);
            painter.circle_filled(handle_pos, r, self.theme.accent);
            painter.circle_stroke(handle_pos, r, Stroke::new(1.5, self.theme.text_primary));
        }

        // Source/target text labels
        let card_font_size = 11.0 * self.viewport.zoom;
        if !edge.source_label.is_empty() && card_font_size > 3.0 {
            let near_src = cubic_bezier_point(src, cp1, cp2, tgt, 0.08);
            let lbl_offset = Vec2::new(0.0, -10.0 * self.viewport.zoom);
            painter.text(
                near_src + lbl_offset,
                Align2::CENTER_BOTTOM,
                &edge.source_label,
                FontId::proportional(card_font_size),
                edge_color,
            );
        }
        if !edge.target_label.is_empty() && card_font_size > 3.0 {
            let near_tgt = cubic_bezier_point(src, cp1, cp2, tgt, 0.92);
            let lbl_offset = Vec2::new(0.0, -10.0 * self.viewport.zoom);
            painter.text(
                near_tgt + lbl_offset,
                Align2::CENTER_BOTTOM,
                &edge.target_label,
                FontId::proportional(card_font_size),
                edge_color,
            );
        }
    }

    fn draw_crow_foot(
        &self,
        painter: &egui::Painter,
        from: Pos2,
        to: Pos2,
        cardinality: Cardinality,
        color: Color32,
        line_width: f32,
    ) {
        let dir = (to - from).normalized();
        if dir.length() < 0.01 {
            return;
        }
        let perp = Vec2::new(-dir.y, dir.x);
        let zoom = self.viewport.zoom.sqrt();

        let bar_half = 8.0 * zoom;
        let circle_r = 5.0 * zoom;
        let foot_spread = 8.0 * zoom;
        let foot_len = 12.0 * zoom;
        let outer_dist = 3.0 * zoom;
        let inner_dist = 15.0 * zoom;
        let stroke = Stroke::new(line_width.max(1.5 * zoom), color);

        match cardinality {
            Cardinality::None => {}
            Cardinality::ExactlyOne => {
                let outer_pt = to - dir * outer_dist;
                let inner_pt = to - dir * inner_dist;
                painter.line_segment(
                    [outer_pt + perp * bar_half, outer_pt - perp * bar_half],
                    stroke,
                );
                painter.line_segment(
                    [inner_pt + perp * bar_half, inner_pt - perp * bar_half],
                    stroke,
                );
            }
            Cardinality::ZeroOrOne => {
                let outer_pt = to - dir * outer_dist;
                let circle_center = to - dir * (inner_dist + circle_r);
                painter.line_segment(
                    [outer_pt + perp * bar_half, outer_pt - perp * bar_half],
                    stroke,
                );
                painter.circle_stroke(circle_center, circle_r, stroke);
            }
            Cardinality::OneOrMany => {
                let inner_pt = to - dir * inner_dist;
                let convergence = to - dir * foot_len;
                painter.line_segment([convergence, to + perp * foot_spread], stroke);
                painter.line_segment([convergence, to], stroke);
                painter.line_segment([convergence, to - perp * foot_spread], stroke);
                painter.line_segment(
                    [inner_pt + perp * bar_half, inner_pt - perp * bar_half],
                    stroke,
                );
            }
            Cardinality::ZeroOrMany => {
                let convergence = to - dir * foot_len;
                let circle_center = to - dir * (inner_dist + circle_r);
                painter.line_segment([convergence, to + perp * foot_spread], stroke);
                painter.line_segment([convergence, to], stroke);
                painter.line_segment([convergence, to - perp * foot_spread], stroke);
                painter.circle_stroke(circle_center, circle_r, stroke);
            }
        }
    }

    fn draw_arrow_head(
        &self,
        painter: &egui::Painter,
        from: Pos2,
        to: Pos2,
        color: Color32,
        width: f32,
        style: ArrowHead,
    ) {
        if style == ArrowHead::None {
            return;
        }
        let dir = (to - from).normalized();
        if dir.length() < 0.01 {
            return;
        }
        // Scale arrowhead with both zoom and edge width for visual consistency
        let base_scale = self.viewport.zoom.sqrt();
        let width_factor = (1.0 + (width - 1.0) * 0.4).max(1.0); // subtle scaling with thickness
        let arrow_len = 10.0 * base_scale * width_factor;
        let arrow_width = 6.0 * base_scale * width_factor;
        let perp = Vec2::new(-dir.y, dir.x);
        let tip = to;

        match style {
            ArrowHead::Filled => {
                let left = tip - dir * arrow_len + perp * arrow_width;
                let right = tip - dir * arrow_len - perp * arrow_width;
                painter.add(egui::Shape::convex_polygon(
                    vec![tip, left, right],
                    color,
                    Stroke::new(width * 0.5, color),
                ));
            }
            ArrowHead::Open => {
                let left = tip - dir * arrow_len + perp * arrow_width;
                let right = tip - dir * arrow_len - perp * arrow_width;
                let stroke = Stroke::new(width.max(1.5), color);
                painter.line_segment([left, tip], stroke);
                painter.line_segment([right, tip], stroke);
            }
            ArrowHead::Circle => {
                let r = arrow_width;
                let center = tip - dir * r;
                painter.circle_filled(center, r, color);
                painter.circle_stroke(center, r, Stroke::new(width * 0.5, color));
            }
            ArrowHead::None => {}
        }
    }

    /// Return a soft background color for a section name.
    /// Well-known design-thinking names get fixed hues; others use a hash.
    fn section_bg_color(name: &str) -> egui::Color32 {
        let name_lower = name.to_lowercase();
        // Well-known section names → preset pastel fills
        let rgb: [u8; 3] = if name_lower.contains("hypothes") {
            [250, 179, 135] // peach
        } else if name_lower.contains("evidence") || name_lower.contains("data") {
            [166, 227, 161] // green
        } else if name_lower.contains("assumption") {
            [137, 180, 250] // blue
        } else if name_lower.contains("conclusion") || name_lower.contains("decision") {
            [203, 166, 247] // mauve
        } else if name_lower.contains("option") || name_lower.contains("alternative") {
            [249, 226, 175] // yellow
        } else if name_lower.contains("risk") || name_lower.contains("consequence") || name_lower.contains("issue") {
            [243, 139, 168] // red/pink
        } else if name_lower.contains("context") || name_lower.contains("background") {
            [148, 226, 213] // teal
        } else if name_lower.contains("note") || name_lower.contains("summary") || name_lower.contains("about") {
            [180, 190, 254] // lavender
        } else if name_lower.contains("strength") || name_lower.contains("opportunity") {
            [166, 227, 161] // green
        } else if name_lower.contains("weakness") || name_lower.contains("threat") {
            [243, 139, 168] // red
        } else if name_lower.contains("objective") || name_lower.contains("goal") {
            [249, 226, 175] // yellow
        } else if name_lower.contains("initiative") || name_lower.contains("action") {
            [137, 180, 250] // blue
        } else {
            // Hash-based color from a curated palette
            let pallete: &[[u8; 3]] = &[
                [250, 179, 135], [137, 180, 250], [166, 227, 161],
                [203, 166, 247], [249, 226, 175], [148, 226, 213],
                [180, 190, 254], [243, 139, 168],
            ];
            let mut h: u32 = 5381;
            for b in name.bytes() {
                h = h.wrapping_mul(33).wrapping_add(b as u32);
            }
            pallete[(h as usize) % pallete.len()]
        };
        egui::Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], 28)
    }

    /// Draw soft colored backgrounds behind each group of nodes sharing a `section_name`.
    /// Drawn before edges and nodes so backgrounds appear underneath everything.
    pub(crate) fn draw_section_backgrounds(&self, painter: &egui::Painter, canvas_rect: egui::Rect) {
        use std::collections::HashMap;
        use egui::{Pos2, Rect};

        // Per section: (bounds, done_count, wip_count, tagged_count, total_count)
        let mut section_bounds: HashMap<&str, Rect> = HashMap::new();
        let mut section_counts: HashMap<&str, (u32, u32, u32, u32)> = HashMap::new();
        let mut section_order: Vec<&str> = Vec::new();

        for node in &self.document.nodes {
            if node.section_name.is_empty() { continue; }
            let sr = Rect::from_min_size(
                self.viewport.canvas_to_screen(node.pos()),
                node.size_vec() * self.viewport.zoom,
            );
            if !sr.expand(300.0).intersects(canvas_rect) { continue; }
            let key = node.section_name.as_str();
            if !section_bounds.contains_key(key) {
                section_order.push(key);
            }
            let entry = section_bounds.entry(key).or_insert(sr);
            *entry = entry.union(sr);
            let (done, wip, tagged, total) = section_counts.entry(key).or_insert((0, 0, 0, 0));
            *total += 1;
            match node.tag {
                Some(crate::model::NodeTag::Ok) => { *done += 1; *tagged += 1; }
                Some(crate::model::NodeTag::Info) => { *wip += 1; *tagged += 1; }
                Some(crate::model::NodeTag::Warning) if node.progress >= 0.5 => { *wip += 1; *tagged += 1; } // review counts as wip for progress
                Some(crate::model::NodeTag::Warning) => { *tagged += 1; } // todo
                Some(crate::model::NodeTag::Critical) => { *tagged += 1; } // blocked
                None => {}
            }
        }
        if section_bounds.is_empty() { return; }

        let pad = (24.0 * self.viewport.zoom).clamp(12.0, 36.0);
        let bar_h = (3.0 * self.viewport.zoom).clamp(2.0, 5.0);

        for section_name in section_order {
            let Some(bounds) = section_bounds.get(section_name) else { continue };
            let bg = Self::section_bg_color(section_name);
            let border = egui::Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), 55);
            let padded = bounds.expand(pad);
            painter.rect(
                padded,
                egui::CornerRadius::same(10),
                bg,
                egui::Stroke::new(1.0, border),
                egui::StrokeKind::Inside,
            );

            // Progress bar at bottom of section background
            if let Some(&(done, wip, tagged, total)) = section_counts.get(section_name) {
                if tagged > 0 && total > 0 {
                    let bar_y = padded.max.y - bar_h - 4.0;
                    let bar_x0 = padded.min.x + 8.0;
                    let bar_x1 = padded.max.x - 8.0;
                    let bar_w = bar_x1 - bar_x0;

                    // Track background
                    let track_rect = Rect::from_min_max(
                        Pos2::new(bar_x0, bar_y),
                        Pos2::new(bar_x1, bar_y + bar_h),
                    );
                    painter.rect_filled(track_rect, egui::CornerRadius::same(bar_h as u8 / 2),
                        egui::Color32::from_rgba_unmultiplied(40, 40, 60, 80));

                    // WIP fill (blue, done+wip fraction)
                    let wip_frac = ((done + wip) as f32 / total as f32).clamp(0.0, 1.0);
                    if wip_frac > 0.0 {
                        let wip_rect = Rect::from_min_max(
                            Pos2::new(bar_x0, bar_y),
                            Pos2::new(bar_x0 + bar_w * wip_frac, bar_y + bar_h),
                        );
                        painter.rect_filled(wip_rect, egui::CornerRadius::same(bar_h as u8 / 2),
                            egui::Color32::from_rgba_unmultiplied(137, 180, 250, 160));
                    }

                    // Done fill (green, done fraction — drawn on top)
                    let done_frac = (done as f32 / total as f32).clamp(0.0, 1.0);
                    if done_frac > 0.0 {
                        let done_rect = Rect::from_min_max(
                            Pos2::new(bar_x0, bar_y),
                            Pos2::new(bar_x0 + bar_w * done_frac, bar_y + bar_h),
                        );
                        painter.rect_filled(done_rect, egui::CornerRadius::same(bar_h as u8 / 2),
                            egui::Color32::from_rgba_unmultiplied(166, 227, 161, 180));
                    }
                }
            }
        }
    }

    /// Draw faint section labels above each cluster of nodes that share the same `section_name`.
    /// Only shown in 2D non-timeline mode when zoomed in enough.
    pub(crate) fn draw_section_labels(&self, painter: &egui::Painter, canvas_rect: egui::Rect) {
        use std::collections::HashMap;
        use egui::{Align2, FontId, Rect};

        // Collect bounding boxes per section_name (non-empty sections only)
        let mut section_bounds: HashMap<&str, Rect> = HashMap::new();
        for node in &self.document.nodes {
            if node.section_name.is_empty() { continue; }
            let sr = Rect::from_min_size(
                self.viewport.canvas_to_screen(node.pos()),
                node.size_vec() * self.viewport.zoom,
            );
            // Only include nodes that are at least partially visible
            if !sr.expand(200.0).intersects(canvas_rect) { continue; }
            let entry = section_bounds.entry(&node.section_name).or_insert(sr);
            *entry = entry.union(sr);
        }
        if section_bounds.is_empty() { return; }

        let font_size = (12.0 * self.viewport.zoom.sqrt()).clamp(9.5, 16.0);

        for (section_name, bounds) in &section_bounds {
            let pad = (24.0 * self.viewport.zoom).clamp(12.0, 36.0);
            let label_x = bounds.min.x - pad + 8.0;
            let label_y = bounds.min.y - pad + font_size + 4.0;
            let label_pos = egui::Pos2::new(label_x, label_y);
            if !canvas_rect.expand(50.0).contains(label_pos) { continue; }

            // Pick section-type icon and use section bg color (more opaque) for text
            let bg_col = Self::section_bg_color(section_name);
            let text_col = egui::Color32::from_rgba_unmultiplied(
                bg_col.r().saturating_add(40),
                bg_col.g().saturating_add(40),
                bg_col.b().saturating_add(40),
                210,
            );
            let s = section_name.to_lowercase();
            let icon = if s.contains("hypothes") { "💡 " }
                else if s.contains("evidence") || s.contains("data") { "📊 " }
                else if s.contains("assumption") { "💭 " }
                else if s.contains("conclusion") || s.contains("decision") { "✅ " }
                else if s.contains("risk") || s.contains("issue") || s.contains("block") { "⚠️ " }
                else if s.contains("observation") || s.contains("insight") { "👁 " }
                else if s.contains("strength") || s.contains("opportunit") { "🌱 " }
                else if s.contains("weakness") || s.contains("threat") { "⚡ " }
                else { "" };

            let display = format!("{icon}{section_name}");
            painter.text(
                label_pos,
                Align2::LEFT_TOP,
                &display,
                FontId::new(font_size, egui::FontFamily::Proportional),
                text_col,
            );
        }
    }

    /// Draw the timeline grid overlay: period columns, lane rows, headers, and connectors.
    /// Called only when `doc.timeline_mode` is true and 2D view is active.
    pub(crate) fn draw_timeline_grid(&self, painter: &egui::Painter, canvas_rect: egui::Rect) {
        use egui::{FontId, Align2};

        let periods = &self.document.timeline_periods;
        let lanes = &self.document.timeline_lanes;
        if periods.is_empty() { return; }

        // Layout constants (must match layout.rs timeline_layout constants)
        const CELL_PAD: f32 = 16.0;
        const HEADER_H: f32 = 36.0;
        const HEADER_W: f32 = 120.0;
        const CELL_GAP: f32 = 12.0;
        const ORIGIN_X: f32 = 140.0;
        const ORIGIN_Y: f32 = 60.0;
        const MIN_CELL_W: f32 = 180.0;
        const MIN_CELL_H: f32 = 80.0;

        let num_periods = periods.len();
        let num_lanes = if lanes.is_empty() { 1 } else { lanes.len() };

        // Compute cell sizes matching layout.rs
        let mut col_w: Vec<f32> = vec![MIN_CELL_W; num_periods];
        let mut row_h: Vec<f32> = vec![MIN_CELL_H; num_lanes];
        for node in &self.document.nodes {
            let p_idx = node.timeline_period.as_ref()
                .and_then(|p| periods.iter().position(|x| x == p));
            let l_idx = node.timeline_lane.as_ref()
                .and_then(|l| lanes.iter().position(|x| x == l));
            if let (Some(p), Some(l)) = (p_idx, l_idx) {
                col_w[p] = col_w[p].max(node.size[0] + CELL_PAD * 2.0);
                row_h[l] = row_h[l].max(node.size[1] + CELL_PAD * 2.0 + CELL_GAP);
            }
        }

        let mut col_x: Vec<f32> = Vec::new();
        let mut cx = ORIGIN_X;
        for w in &col_w { col_x.push(cx); cx += w; }
        let total_w = cx - ORIGIN_X;

        let mut row_y: Vec<f32> = Vec::new();
        let mut ry = ORIGIN_Y + HEADER_H;
        for h in &row_h { row_y.push(ry); ry += h; }
        let total_h = ry - ORIGIN_Y - HEADER_H;

        let vp = &self.viewport;
        let to_screen = |wx: f32, wy: f32| -> egui::Pos2 {
            vp.canvas_to_screen(egui::Pos2::new(wx, wy))
        };
        let scale = vp.zoom;

        let theme = &self.theme;

        // --- Lane row backgrounds (alternating) ---
        for (l, y_start) in row_y.iter().enumerate() {
            let y_end = *y_start + row_h[l];
            let grid_left = ORIGIN_X;
            let grid_right = ORIGIN_X + total_w;
            let rect = egui::Rect::from_min_max(
                to_screen(grid_left, *y_start),
                to_screen(grid_right, y_end),
            );
            if rect.width() < 1.0 || rect.height() < 1.0 { continue; }
            if !rect.intersects(canvas_rect) { continue; }
            let alpha = if l % 2 == 0 { 10u8 } else { 20u8 };
            let bg = egui::Color32::from_rgba_premultiplied(100, 120, 200, alpha);
            painter.rect_filled(rect, egui::CornerRadius::same(0), bg);
        }

        // --- Period column backgrounds ---
        for (p, x_start) in col_x.iter().enumerate() {
            let x_end = *x_start + col_w[p];
            let y_top = ORIGIN_Y + HEADER_H;
            let y_bot = y_top + total_h;
            let rect = egui::Rect::from_min_max(
                to_screen(*x_start, y_top),
                to_screen(x_end, y_bot),
            );
            if rect.width() < 1.0 || rect.height() < 1.0 { continue; }
            if !rect.intersects(canvas_rect) { continue; }
            let bg = egui::Color32::from_rgba_premultiplied(140, 160, 220, 8);
            painter.rect_filled(rect, egui::CornerRadius::same(0), bg);
        }

        // --- Grid lines ---
        let grid_color = egui::Color32::from_rgba_premultiplied(120, 130, 170, 50);
        let grid_stroke = egui::Stroke::new(1.0, grid_color);
        // Horizontal lines (lane boundaries)
        for y_start in &row_y {
            let p0 = to_screen(ORIGIN_X, *y_start);
            let p1 = to_screen(ORIGIN_X + total_w, *y_start);
            if canvas_rect.contains(p0) || canvas_rect.contains(p1) {
                painter.line_segment([p0, p1], grid_stroke);
            }
        }
        // Bottom edge
        let bot_y = ORIGIN_Y + HEADER_H + total_h;
        painter.line_segment([to_screen(ORIGIN_X, bot_y), to_screen(ORIGIN_X + total_w, bot_y)], grid_stroke);
        // Vertical lines (period boundaries)
        for x_start in &col_x {
            let p0 = to_screen(*x_start, ORIGIN_Y + HEADER_H);
            let p1 = to_screen(*x_start, bot_y);
            if canvas_rect.contains(p0) || canvas_rect.contains(p1) {
                painter.line_segment([p0, p1], grid_stroke);
            }
        }
        // Right edge
        let right_x = ORIGIN_X + total_w;
        painter.line_segment([to_screen(right_x, ORIGIN_Y + HEADER_H), to_screen(right_x, bot_y)], grid_stroke);

        // --- Period header row ---
        for (p, period_label) in periods.iter().enumerate() {
            let x_center = col_x[p] + col_w[p] * 0.5;
            let y_center = ORIGIN_Y + HEADER_H * 0.5;
            let hdr_rect = egui::Rect::from_min_max(
                to_screen(col_x[p], ORIGIN_Y),
                to_screen(col_x[p] + col_w[p], ORIGIN_Y + HEADER_H),
            );
            if !hdr_rect.intersects(canvas_rect) { continue; }
            // Header background
            let hdr_bg = egui::Color32::from_rgba_premultiplied(100, 130, 220, 35);
            painter.rect_filled(hdr_rect, egui::CornerRadius::same(0), hdr_bg);
            // Header text
            let font_size = (13.0 * scale).clamp(9.0, 20.0);
            let center_screen = to_screen(x_center, y_center);
            painter.text(
                center_screen,
                Align2::CENTER_CENTER,
                period_label,
                FontId::proportional(font_size),
                theme.text_primary,
            );

            // Arrow connector to next period (→)
            if p + 1 < num_periods {
                let arrow_x = col_x[p] + col_w[p];
                let arrow_y = ORIGIN_Y + HEADER_H * 0.5;
                let p_from = to_screen(arrow_x - 6.0, arrow_y);
                let p_to = to_screen(col_x[p + 1] + 4.0, arrow_y);
                let arrow_color = egui::Color32::from_rgba_premultiplied(120, 150, 220, 120);
                painter.arrow(p_from, p_to - p_from, egui::Stroke::new(1.5, arrow_color));
            }
        }

        // --- Lane labels (left sidebar) ---
        for (l, lane_label) in lanes.iter().enumerate() {
            let y_center = row_y[l] + row_h[l] * 0.5;
            let label_rect = egui::Rect::from_min_max(
                to_screen(ORIGIN_X - HEADER_W, row_y[l]),
                to_screen(ORIGIN_X, row_y[l] + row_h[l]),
            );
            if !label_rect.intersects(canvas_rect) { continue; }
            let font_size = (12.0 * scale).clamp(8.0, 16.0);
            let center_screen = to_screen(ORIGIN_X - HEADER_W * 0.5, y_center);
            painter.text(
                center_screen,
                Align2::CENTER_CENTER,
                lane_label,
                FontId::proportional(font_size),
                theme.text_secondary,
            );
        }
    }

    pub(crate) fn draw_resize_handles(&self, painter: &egui::Painter, screen_rect: Rect) {
        let handle_half = 5.0;
        let handles = Self::resize_handle_positions(screen_rect);
        for (_handle, pos) in &handles {
            let r = Rect::from_center_size(*pos, Vec2::splat(handle_half * 2.0));
            painter.rect_filled(r, CornerRadius::ZERO, self.theme.selection_color);
            painter.rect_stroke(
                r,
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::WHITE),
                StrokeKind::Outside,
            );
        }
    }
}
