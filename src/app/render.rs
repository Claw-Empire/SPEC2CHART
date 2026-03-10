use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::model::*;
use super::FlowchartApp;
use super::interaction::{control_points_for_side, cubic_bezier_point};
use super::theme::*;

impl FlowchartApp {
    pub(crate) fn draw_node(&self, node: &Node, painter: &egui::Painter, hover_pos: Option<Pos2>) {
        let top_left = self.viewport.canvas_to_screen(node.pos());
        let size = node.size_vec() * self.viewport.zoom;
        let screen_rect = Rect::from_min_size(top_left, size);

        let is_selected = self.selection.contains_node(&node.id);
        let is_hovered = hover_pos.map_or(false, |hp| screen_rect.expand(6.0).contains(hp));

        // Selection glow
        if is_selected {
            let glow_rect = screen_rect.expand(5.0);
            painter.rect_filled(glow_rect, CornerRadius::same(6), ACCENT_GLOW);
        } else if is_hovered {
            painter.rect_stroke(
                screen_rect.expand(2.0),
                CornerRadius::same(4),
                Stroke::new(1.5, ACCENT_HOVER),
                StrokeKind::Outside,
            );
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

        // Ports
        let show_ports = self.tool == super::Tool::Connect || {
            if let Some(hover) = hover_pos {
                screen_rect.expand(30.0).contains(hover)
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
                    painter.circle_filled(screen_port, glow_r, ACCENT_GLOW);
                    painter.circle_filled(screen_port, r * 1.3, ACCENT);
                    painter.circle_stroke(screen_port, r * 1.3, Stroke::new(2.0, Color32::WHITE));
                } else {
                    painter.circle_filled(screen_port, r, PORT_FILL);
                    painter.circle_stroke(screen_port, r, Stroke::new(1.5, SELECTION_COLOR));
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
            SHADOW_LIGHT,
        );

        let fill = to_color32(style.fill_color);
        let border_color = if is_selected {
            SELECTION_COLOR
        } else {
            to_color32(style.border_color)
        };
        let border_width = if is_selected {
            style.border_width.max(2.5)
        } else {
            style.border_width
        };
        let stroke = Stroke::new(border_width * self.viewport.zoom.sqrt(), border_color);

        match shape {
            NodeShape::Rectangle => {
                painter.rect_filled(screen_rect, CornerRadius::ZERO, fill);
                painter.rect_stroke(screen_rect, CornerRadius::ZERO, stroke, StrokeKind::Outside);
            }
            NodeShape::RoundedRect => {
                let r = (10.0 * self.viewport.zoom) as u8;
                painter.rect_filled(screen_rect, CornerRadius::same(r), fill);
                painter.rect_stroke(
                    screen_rect,
                    CornerRadius::same(r),
                    stroke,
                    StrokeKind::Outside,
                );
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
        }

        let text_color = to_color32(style.text_color);
        let font_size = style.font_size * self.viewport.zoom;
        if font_size > 4.0 {
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(font_size),
                text_color,
            );
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
            SHADOW_MEDIUM,
        );

        let fill = to_color32(style.fill_color);
        let corner = CornerRadius::same((8.0 * self.viewport.zoom) as u8);
        painter.rect_filled(screen_rect, corner, fill);

        if is_selected {
            painter.rect_stroke(
                screen_rect,
                corner,
                Stroke::new(2.5 * self.viewport.zoom.sqrt(), SELECTION_COLOR),
                StrokeKind::Outside,
            );
        }

        let text_color = to_color32(style.text_color);
        let font_size = style.font_size * self.viewport.zoom;
        if font_size > 4.0 && !text.is_empty() {
            let padding = 10.0 * self.viewport.zoom;
            let text_rect = screen_rect.shrink(padding);
            let galley = painter.layout(
                text.to_string(),
                FontId::proportional(font_size),
                text_color,
                text_rect.width(),
            );
            let text_pos = Pos2::new(text_rect.min.x, text_rect.min.y);
            painter.galley(text_pos, galley, Color32::TRANSPARENT);
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
            SHADOW_LIGHT,
        );

        let fill = to_color32(style.fill_color);
        let border_color = if is_selected {
            SELECTION_COLOR
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
                Color32::WHITE,
            );
        }

        // Attributes
        let row_h = ENTITY_ROW_HEIGHT * zoom;
        let attr_font = style.font_size * zoom * 0.9;
        let text_color = to_color32(style.text_color);
        let pk_color = ACCENT;
        let fk_color = FK_COLOR;

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
                        Stroke::new(0.5, ROW_DIVIDER),
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
                    TEXT_DIM,
                );
            }

            if attributes.is_empty() {
                let row_center_y = divider_y + row_h / 2.0;
                painter.text(
                    Pos2::new(screen_rect.center().x, row_center_y),
                    Align2::CENTER_CENTER,
                    "no attributes",
                    FontId::proportional(attr_font * 0.85),
                    TEXT_DIM,
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
                Stroke::new(1.5, ACCENT_SELECT_LIGHT),
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

        let src = self
            .viewport
            .canvas_to_screen(src_node.port_position(edge.source.side));
        let tgt = self
            .viewport
            .canvas_to_screen(tgt_node.port_position(edge.target.side));

        let is_selected = self.selection.contains_edge(&edge.id);
        let edge_color = if is_selected {
            SELECTION_COLOR
        } else {
            to_color32(edge.style.color)
        };
        let base_width = edge.style.width * self.viewport.zoom.sqrt();
        let width = if is_selected {
            base_width.max(3.0)
        } else {
            base_width
        };

        let offset = 60.0 * self.viewport.zoom;
        let (cp1, cp2) = control_points_for_side(src, tgt, edge.source.side, offset);

        if is_selected {
            let glow = egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt],
                false,
                Color32::TRANSPARENT,
                Stroke::new(width + 6.0, ACCENT_SELECT_BG),
            );
            painter.add(glow);
        }

        let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
            [src, cp1, cp2, tgt],
            false,
            Color32::TRANSPARENT,
            Stroke::new(width, edge_color),
        );
        painter.add(bezier);

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
            self.draw_arrow_head(painter, cp2, tgt, edge_color, width);
        }

        // Edge label
        if !edge.label.is_empty() {
            let mid = cubic_bezier_point(src, cp1, cp2, tgt, 0.5);
            let font_size = 12.0 * self.viewport.zoom;
            if font_size > 4.0 {
                let galley = painter.layout_no_wrap(
                    edge.label.clone(),
                    FontId::proportional(font_size),
                    Color32::WHITE,
                );
                let text_rect = Rect::from_min_size(
                    Pos2::new(
                        mid.x - galley.size().x / 2.0,
                        mid.y - galley.size().y / 2.0,
                    ),
                    galley.size(),
                )
                .expand(3.0);
                painter.rect_filled(text_rect, CornerRadius::same(3), EDGE_LABEL_BG);
                painter.text(
                    mid,
                    Align2::CENTER_CENTER,
                    &edge.label,
                    FontId::proportional(font_size),
                    edge_color,
                );
            }
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
    ) {
        let dir = (to - from).normalized();
        if dir.length() < 0.01 {
            return;
        }
        let arrow_len = 10.0 * self.viewport.zoom.sqrt();
        let arrow_width = 6.0 * self.viewport.zoom.sqrt();
        let perp = Vec2::new(-dir.y, dir.x);

        let tip = to;
        let left = tip - dir * arrow_len + perp * arrow_width;
        let right = tip - dir * arrow_len - perp * arrow_width;

        painter.add(egui::Shape::convex_polygon(
            vec![tip, left, right],
            color,
            Stroke::new(width * 0.5, color),
        ));
    }

    pub(crate) fn draw_resize_handles(&self, painter: &egui::Painter, screen_rect: Rect) {
        let handle_half = 5.0;
        let handles = Self::resize_handle_positions(screen_rect);
        for (_handle, pos) in &handles {
            let r = Rect::from_center_size(*pos, Vec2::splat(handle_half * 2.0));
            painter.rect_filled(r, CornerRadius::ZERO, SELECTION_COLOR);
            painter.rect_stroke(
                r,
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::WHITE),
                StrokeKind::Outside,
            );
        }
    }
}
