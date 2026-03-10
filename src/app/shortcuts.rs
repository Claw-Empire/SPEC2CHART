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

        // Cmd+V = paste
        if ctx.input(|i| i.key_pressed(Key::V) && i.modifiers.matches_exact(cmd))
            && !self.clipboard.is_empty()
        {
            self.selection.clear();
            let offset = Vec2::new(30.0, 30.0);
            for template in self.clipboard.clone() {
                let mut node = template;
                node.id = NodeId::new();
                let pos = node.pos() + offset;
                node.set_pos(pos);
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
    }
}
