use egui::{Key, Modifiers, Vec2};
use crate::model::*;
use super::FlowchartApp;

impl FlowchartApp {
    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            self.space_held = i.key_down(Key::Space);
        });

        let cmd = if cfg!(target_os = "macos") {
            Modifiers::MAC_CMD
        } else {
            Modifiers::CTRL
        };

        // Cmd+Z = undo
        if ctx.input(|i| i.key_pressed(Key::Z) && i.modifiers.matches_exact(cmd)) {
            if let Some(doc) = self.history.undo() {
                self.document = doc.clone();
                self.selection.clear();
            }
        }

        // Cmd+Shift+Z = redo
        let cmd_shift = Modifiers {
            shift: true,
            ..cmd
        };
        if ctx.input(|i| i.key_pressed(Key::Z) && i.modifiers.matches_exact(cmd_shift)) {
            if let Some(doc) = self.history.redo() {
                self.document = doc.clone();
                self.selection.clear();
            }
        }

        // Delete/Backspace = remove selected (skip when editing text)
        let any_text_focused = ctx.memory(|m| m.focused()).is_some()
            && ctx.wants_keyboard_input();
        if !any_text_focused
            && ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
            && !self.selection.is_empty()
        {
            let node_ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            let edge_ids: Vec<EdgeId> = self.selection.edge_ids.iter().copied().collect();
            for id in &node_ids {
                self.document.remove_node(id);
            }
            for id in &edge_ids {
                self.document.remove_edge(id);
            }
            self.selection.clear();
            self.history.push(&self.document);
        }

        // Cmd+C = copy selected nodes
        if ctx.input(|i| i.key_pressed(Key::C) && i.modifiers.matches_exact(cmd)) {
            self.clipboard.clear();
            for id in &self.selection.node_ids {
                if let Some(node) = self.document.find_node(id) {
                    self.clipboard.push(node.clone());
                }
            }
        }

        // Cmd+V = paste (centered on viewport)
        if ctx.input(|i| i.key_pressed(Key::V) && i.modifiers.matches_exact(cmd))
            && !self.clipboard.is_empty()
        {
            self.selection.clear();
            // Compute clipboard centroid in world space
            let n = self.clipboard.len() as f32;
            let centroid: Vec2 = self.clipboard.iter().fold(Vec2::ZERO, |acc, nd| acc + nd.pos().to_vec2()) / n;
            // Compute viewport center in world space
            let vp_center: Vec2 = (self.canvas_rect.center().to_vec2()
                - Vec2::new(self.viewport.offset[0], self.viewport.offset[1]))
                / self.viewport.zoom;
            let shift: Vec2 = vp_center - centroid;
            for template in self.clipboard.clone() {
                let mut node = template;
                node.id = NodeId::new();
                node.set_pos(node.pos() + shift);
                self.selection.node_ids.insert(node.id);
                self.document.nodes.push(node);
            }
            self.history.push(&self.document);
        }

        // V = select tool (skip when editing text)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::V) && i.modifiers.is_none()) {
            self.tool = super::Tool::Select;
        }
        // E = connect tool (skip when editing text)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::E) && i.modifiers.is_none()) {
            self.tool = super::Tool::Connect;
        }

        // Cmd+A = select all
        if ctx.input(|i| i.key_pressed(Key::A) && i.modifiers.matches_exact(cmd)) {
            self.selection.clear();
            for node in &self.document.nodes {
                self.selection.node_ids.insert(node.id);
            }
            for edge in &self.document.edges {
                self.selection.edge_ids.insert(edge.id);
            }
        }

        // 2 = switch to 2D view (no modifier, skip when editing text)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::Num2) && i.modifiers.is_none()) {
            if self.view_mode != super::ViewMode::TwoD {
                self.sync_viewport_to_camera();
                self.view_mode = super::ViewMode::TwoD;
                self.view_transition_target = 0.0;
                self.status_message = Some(("2D View".to_string(), std::time::Instant::now()));
            }
        }

        // 3 = switch to 3D view (no modifier, skip when editing text)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::Num3) && i.modifiers.is_none()) {
            if self.view_mode != super::ViewMode::ThreeD {
                self.view_mode = super::ViewMode::ThreeD;
                self.view_transition_target = 1.0;
                self.sync_camera_to_viewport();
                self.status_message = Some(("3D View".to_string(), std::time::Instant::now()));
            }
        }

        // Cmd+1 = fit to content
        if ctx.input(|i| i.key_pressed(Key::Num1) && i.modifiers.matches_exact(cmd)) {
            self.fit_to_content();
        }

        // Cmd+2 = zoom to selection
        if ctx.input(|i| i.key_pressed(Key::Num2) && i.modifiers.matches_exact(cmd)) {
            self.zoom_to_selection();
        }

        // Cmd+= zoom in
        if ctx.input(|i| {
            (i.key_pressed(Key::Equals) || i.key_pressed(Key::Plus))
                && i.modifiers.matches_exact(cmd)
        }) {
            self.step_zoom(1.25);
        }

        // Cmd+- zoom out
        if ctx.input(|i| i.key_pressed(Key::Minus) && i.modifiers.matches_exact(cmd)) {
            self.step_zoom(0.8);
        }

        // Cmd+0 = reset zoom
        if ctx.input(|i| i.key_pressed(Key::Num0) && i.modifiers.matches_exact(cmd)) {
            self.viewport.zoom = 1.0;
            self.viewport.offset = [0.0, 0.0];
        }

        // F = fit selection (or all if nothing selected)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.is_none()) {
            if !self.selection.is_empty() {
                self.zoom_to_selection();
            } else {
                self.fit_to_content();
            }
        }

        // Cmd+D = duplicate selected nodes
        if ctx.input(|i| i.key_pressed(Key::D) && i.modifiers.matches_exact(cmd))
            && !self.selection.node_ids.is_empty()
        {
            let offset = Vec2::new(24.0, 24.0);
            let originals: Vec<crate::model::Node> = self.selection.node_ids.iter()
                .filter_map(|id| self.document.find_node(id).cloned())
                .collect();
            self.selection.clear();
            for template in originals {
                let mut node = template;
                node.id = NodeId::new();
                node.set_pos(node.pos() + offset);
                self.selection.node_ids.insert(node.id);
                self.document.nodes.push(node);
            }
            self.history.push(&self.document);
            self.status_message = Some(("Duplicated".to_string(), std::time::Instant::now()));
        }

        // Escape = deselect
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.selection.clear();
        }

        // Arrow keys = nudge selected nodes (1px; 10px with Shift)
        if !any_text_focused && !self.selection.node_ids.is_empty() {
            let shift = ctx.input(|i| i.modifiers.shift);
            let step = if shift { 10.0_f32 } else { 1.0_f32 };
            let mut delta = egui::Vec2::ZERO;
            ctx.input(|i| {
                if i.key_pressed(Key::ArrowLeft)  { delta.x -= step; }
                if i.key_pressed(Key::ArrowRight) { delta.x += step; }
                if i.key_pressed(Key::ArrowUp)    { delta.y -= step; }
                if i.key_pressed(Key::ArrowDown)  { delta.y += step; }
            });
            if delta != egui::Vec2::ZERO {
                let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                for id in &ids {
                    if let Some(node) = self.document.nodes.iter_mut().find(|n| n.id == *id) {
                        let p = node.pos();
                        node.set_pos(p + delta);
                    }
                }
                self.history.push(&self.document);
            }
        }
    }
}
