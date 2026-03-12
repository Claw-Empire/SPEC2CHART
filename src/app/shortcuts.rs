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

        // Cmd+Shift+C = copy style of selected node
        let cmd_shift = Modifiers { shift: true, ..cmd };
        if ctx.input(|i| i.key_pressed(Key::C) && i.modifiers.matches_exact(cmd_shift)) {
            if let Some(id) = self.selection.node_ids.iter().next() {
                if let Some(node) = self.document.find_node(id) {
                    self.style_clipboard = Some(node.style.clone());
                    self.status_message = Some(("Style copied".to_string(), std::time::Instant::now()));
                }
            }
        }

        // Cmd+Shift+V = paste style to selected nodes
        if ctx.input(|i| i.key_pressed(Key::V) && i.modifiers.matches_exact(cmd_shift)) {
            if let Some(style) = self.style_clipboard.clone() {
                let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                for id in &ids {
                    if let Some(node) = self.document.find_node_mut(id) {
                        node.style = style.clone();
                    }
                }
                if !ids.is_empty() {
                    self.history.push(&self.document);
                    self.status_message = Some(("Style pasted".to_string(), std::time::Instant::now()));
                }
            }
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
            self.shape_picker = None;
        }
        // E = connect tool (skip when editing text)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::E) && i.modifiers.is_none()) {
            self.tool = super::Tool::Connect;
            self.shape_picker = None;
        }
        // N = open shape picker at pointer (skip when editing text)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::N) && i.modifiers.is_none()) {
            let pos = ctx.input(|i| i.pointer.hover_pos()).unwrap_or(ctx.screen_rect().center());
            self.shape_picker = Some(pos);
        }

        // Quick shape creation shortcuts (skip when editing text)
        let shape_to_create: Option<crate::model::NodeShape> = if !any_text_focused {
            if ctx.input(|i| i.key_pressed(Key::R) && i.modifiers.is_none()) {
                Some(crate::model::NodeShape::Rectangle)
            } else if ctx.input(|i| i.key_pressed(Key::C) && i.modifiers.is_none()) {
                Some(crate::model::NodeShape::Circle)
            } else if ctx.input(|i| i.key_pressed(Key::D) && i.modifiers.is_none()) {
                Some(crate::model::NodeShape::Diamond)
            } else {
                None
            }
        } else { None };

        if let Some(shape) = shape_to_create {
            let canvas_center = {
                let c = self.canvas_rect.center();
                self.viewport.screen_to_canvas(c)
            };
            let mut node = crate::model::Node::new(shape, canvas_center);
            let w = node.size[0]; let h = node.size[1];
            node.set_pos(egui::Pos2::new(canvas_center.x - w / 2.0, canvas_center.y - h / 2.0));
            let id = node.id;
            self.document.nodes.push(node);
            self.selection.select_node(id);
            self.focus_label_edit = true;
            self.history.push(&self.document);
            let name = match shape {
                crate::model::NodeShape::Rectangle => "Rectangle",
                crate::model::NodeShape::Circle => "Circle",
                crate::model::NodeShape::Diamond => "Diamond",
                _ => "Shape",
            };
            self.status_message = Some((format!("{name} created"), std::time::Instant::now()));
        }
        // Escape also closes shape picker
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.shape_picker = None;
        }

        // Cmd+Shift+A = select connected nodes
        let cmd_shift = Modifiers { shift: true, ..cmd };
        if ctx.input(|i| i.key_pressed(Key::A) && i.modifiers.matches_exact(cmd_shift)) {
            let selected: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            for node_id in &selected {
                for edge in &self.document.edges {
                    if edge.source.node_id == *node_id {
                        self.selection.node_ids.insert(edge.target.node_id);
                    } else if edge.target.node_id == *node_id {
                        self.selection.node_ids.insert(edge.source.node_id);
                    }
                }
            }
            let cnt = self.selection.node_ids.len();
            self.status_message = Some((format!("{cnt} nodes"), std::time::Instant::now()));
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

        // Cmd+Shift+K = select connected component of selected nodes (flood-fill)
        let cmd_shift_k = Modifiers { shift: true, ..cmd };
        if ctx.input(|i| i.key_pressed(Key::K) && i.modifiers.matches_exact(cmd_shift_k)) {
            let seed_ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            if !seed_ids.is_empty() {
                let mut visited: std::collections::HashSet<NodeId> = seed_ids.iter().copied().collect();
                let mut queue = seed_ids;
                while let Some(nid) = queue.pop() {
                    for edge in &self.document.edges {
                        let neighbor = if edge.source.node_id == nid { Some(edge.target.node_id) }
                            else if edge.target.node_id == nid { Some(edge.source.node_id) }
                            else { None };
                        if let Some(nbr) = neighbor {
                            if visited.insert(nbr) { queue.push(nbr); }
                        }
                    }
                }
                // Select all visited nodes and their connecting edges
                for &id in &visited { self.selection.node_ids.insert(id); }
                for edge in &self.document.edges {
                    if visited.contains(&edge.source.node_id) && visited.contains(&edge.target.node_id) {
                        self.selection.edge_ids.insert(edge.id);
                    }
                }
                let n = visited.len();
                self.status_message = Some((format!("Connected: {} nodes", n), std::time::Instant::now()));
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

        // Cmd+F = search
        if ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.matches_exact(cmd)) {
            self.show_search = !self.show_search;
            if self.show_search { self.search_query.clear(); }
        }

        // Cmd+H = find & replace
        if ctx.input(|i| i.key_pressed(Key::H) && i.modifiers.matches_exact(cmd)) {
            self.show_find_replace = !self.show_find_replace;
            if self.show_find_replace { self.find_query.clear(); self.replace_query.clear(); }
        }

        // ? = toggle shortcuts panel
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::F1) || (i.key_pressed(Key::Slash) && i.modifiers.shift)) {
            self.show_shortcuts_panel = !self.show_shortcuts_panel;
        }

        // Cmd+Shift+L = toggle canvas lock (prevent node moves)
        let cmd_shift_l = Modifiers { shift: true, ..cmd };
        if ctx.input(|i| i.key_pressed(Key::L) && i.modifiers.matches_exact(cmd_shift_l)) {
            self.canvas_locked = !self.canvas_locked;
            let msg = if self.canvas_locked { "🔒 Canvas locked" } else { "🔓 Canvas unlocked" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // O = toggle overview (bird's eye) mode
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::O) && i.modifiers.is_none()) {
            if let Some(saved) = self.saved_viewport.take() {
                // Restore
                self.viewport = saved;
                self.status_message = Some(("Overview Off".to_string(), std::time::Instant::now()));
            } else {
                // Save and zoom out
                self.saved_viewport = Some(self.viewport.clone());
                self.fit_to_content();
                // Zoom out further for "bird's eye" feel
                let extra = 0.6;
                let cx = self.canvas_rect.center();
                self.viewport.offset[0] = cx.x - (cx.x - self.viewport.offset[0]) / extra;
                self.viewport.offset[1] = cx.y - (cx.y - self.viewport.offset[1]) / extra;
                self.viewport.zoom *= extra;
                self.viewport.zoom = self.viewport.zoom.clamp(0.05, 10.0);
                self.status_message = Some(("Overview Mode — press O to return".to_string(), std::time::Instant::now()));
            }
        }

        // Cmd+L = auto-layout (hierarchical)
        if ctx.input(|i| i.key_pressed(Key::L) && i.modifiers.matches_exact(cmd)) {
            // Reset all non-pinned node positions so the layout runs on all of them
            for node in self.document.nodes.iter_mut() {
                if !node.pinned {
                    node.position = [0.0, 0.0];
                }
            }
            crate::specgraph::layout::hierarchical_layout(&mut self.document);
            self.history.push(&self.document);
            self.pending_fit = true;
            self.status_message = Some(("Auto-layout applied".to_string(), std::time::Instant::now()));
        }

        // Tab / Shift+Tab = cycle through nodes
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::Tab)) {
            let n = self.document.nodes.len();
            if n > 0 {
                let shift = ctx.input(|i| i.modifiers.shift);
                let current = self.selection.node_ids.iter().next().copied()
                    .and_then(|id| self.document.nodes.iter().position(|n| n.id == id));
                let next_idx = match current {
                    None => 0,
                    Some(i) if shift => (i + n - 1) % n,
                    Some(i) => (i + 1) % n,
                };
                let next_id = self.document.nodes[next_idx].id;
                self.selection.select_node(next_id);
                // Pan to show selected node
                let node = &self.document.nodes[next_idx];
                let c = self.canvas_rect.center();
                let p = node.pos();
                self.viewport.offset[0] = c.x - p.x * self.viewport.zoom;
                self.viewport.offset[1] = c.y - p.y * self.viewport.zoom;
            }
        }

        // P = cycle background pattern
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::P) && i.modifiers.is_none()) {
            self.bg_pattern = match self.bg_pattern {
                super::BgPattern::Dots       => super::BgPattern::Lines,
                super::BgPattern::Lines      => super::BgPattern::Crosshatch,
                super::BgPattern::Crosshatch => super::BgPattern::None,
                super::BgPattern::None       => super::BgPattern::Dots,
            };
            let name = match self.bg_pattern {
                super::BgPattern::Dots       => "Dots",
                super::BgPattern::Lines      => "Lines",
                super::BgPattern::Crosshatch => "Crosshatch",
                super::BgPattern::None       => "No pattern",
            };
            self.status_message = Some((name.to_string(), std::time::Instant::now()));
        }

        // W = toggle focus mode (dim non-selected)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::W) && i.modifiers.is_none()) {
            self.focus_mode = !self.focus_mode;
            let msg = if self.focus_mode { "Focus Mode On" } else { "Focus Mode Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // F = toggle presentation mode (hide all panels for clean view)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.is_none()) {
            self.presentation_mode = !self.presentation_mode;
            let msg = if self.presentation_mode { "Presentation Mode On" } else { "Presentation Mode Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // G = toggle grid
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::G) && i.modifiers.is_none()) {
            self.show_grid = !self.show_grid;
            let msg = if self.show_grid { "Grid On" } else { "Grid Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // S = toggle snap to grid
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.is_none()) {
            self.snap_to_grid = !self.snap_to_grid;
            let msg = if self.snap_to_grid { "Snap On" } else { "Snap Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // Tab / Shift+Tab = cycle selection through nodes
        let tab_pressed = ctx.input(|i| i.key_pressed(Key::Tab));
        if !any_text_focused && tab_pressed {
            let shift_held = ctx.input(|i| i.modifiers.shift);
            let n = self.document.nodes.len();
            if n > 0 {
                let current_idx = self.selection.node_ids.iter().next()
                    .and_then(|id| self.document.nodes.iter().position(|n| n.id == *id));
                let next_idx = match current_idx {
                    None => 0,
                    Some(i) if shift_held => (i + n - 1) % n,
                    Some(i) => (i + 1) % n,
                };
                let next_id = self.document.nodes[next_idx].id;
                self.selection.select_node(next_id);
                // Pan viewport to show the selected node
                let node_pos = self.document.nodes[next_idx].pos();
                let screen_center = self.canvas_rect.center();
                self.viewport.offset[0] = screen_center.x - node_pos.x * self.viewport.zoom;
                self.viewport.offset[1] = screen_center.y - node_pos.y * self.viewport.zoom;
            }
        }

        // Arrow keys on selected edge = adjust curve bend (±5; ±20 with Shift)
        if !any_text_focused && self.selection.node_ids.is_empty() && self.selection.edge_ids.len() == 1 {
            let shift = ctx.input(|i| i.modifiers.shift);
            let step = if shift { 20.0_f32 } else { 5.0_f32 };
            let mut bend_delta = 0.0_f32;
            ctx.input(|i| {
                if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowUp)   { bend_delta -= step; }
                if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowDown) { bend_delta += step; }
            });
            if bend_delta != 0.0 {
                let ids: Vec<EdgeId> = self.selection.edge_ids.iter().copied().collect();
                for id in &ids {
                    if let Some(edge) = self.document.find_edge_mut(id) {
                        edge.style.curve_bend = (edge.style.curve_bend + bend_delta).clamp(-500.0, 500.0);
                    }
                }
                self.history.push(&self.document);
            }
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
