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
            // Skip locked nodes
            let node_ids: Vec<NodeId> = self.selection.node_ids.iter()
                .filter(|id| !self.document.find_node(id).map_or(false, |n| n.locked))
                .copied().collect();
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

        // Cmd+] = bring forward (increase z_offset), Cmd+[ = send backward
        // Cmd+Shift+] = bring to front, Cmd+Shift+[ = send to back
        if !self.selection.node_ids.is_empty() {
            let fwd  = ctx.input(|i| i.key_pressed(Key::CloseBracket) && i.modifiers.matches_exact(cmd));
            let back = ctx.input(|i| i.key_pressed(Key::OpenBracket)  && i.modifiers.matches_exact(cmd));
            let front = ctx.input(|i| i.key_pressed(Key::CloseBracket) && i.modifiers.matches_exact(cmd_shift));
            let last  = ctx.input(|i| i.key_pressed(Key::OpenBracket)  && i.modifiers.matches_exact(cmd_shift));
            if fwd || back || front || last {
                let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                let max_z = self.document.nodes.iter().map(|n| n.z_offset).fold(f32::NEG_INFINITY, f32::max);
                let min_z = self.document.nodes.iter().map(|n| n.z_offset).fold(f32::INFINITY, f32::min);
                for id in &ids {
                    if let Some(node) = self.document.find_node_mut(id) {
                        if front       { node.z_offset = max_z + 1.0; }
                        else if last   { node.z_offset = min_z - 1.0; }
                        else if fwd    { node.z_offset += 1.0; }
                        else if back   { node.z_offset -= 1.0; }
                    }
                }
                self.history.push(&self.document);
            }
        }

        // Cmd+M = open comment editor for selected node
        if ctx.input(|i| i.key_pressed(Key::M) && i.modifiers.matches_exact(cmd)) {
            if let Some(id) = self.selection.node_ids.iter().next().copied() {
                self.comment_editing = Some(id);
            }
        }

        // Cmd+L = toggle lock on selected nodes
        if ctx.input(|i| i.key_pressed(Key::L) && i.modifiers.matches_exact(cmd)) && !self.selection.node_ids.is_empty() {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            let all_locked = ids.iter().all(|id| self.document.find_node(id).map_or(false, |n| n.locked));
            for id in &ids {
                if let Some(node) = self.document.find_node_mut(id) {
                    node.locked = !all_locked;
                }
            }
            let msg = if all_locked { "Unlocked" } else { "Locked" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
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

        // Cmd+C = copy selected nodes + edges between them (resets paste offset counter)
        if ctx.input(|i| i.key_pressed(Key::C) && i.modifiers.matches_exact(cmd)) {
            self.clipboard.clear();
            self.edge_clipboard.clear();
            self.paste_count = 0;
            for id in &self.selection.node_ids {
                if let Some(node) = self.document.find_node(id) {
                    self.clipboard.push(node.clone());
                }
            }
            // Copy edges where both endpoints are in the selection
            let sel_ids = &self.selection.node_ids;
            for edge in &self.document.edges {
                if sel_ids.contains(&edge.source.node_id) && sel_ids.contains(&edge.target.node_id) {
                    self.edge_clipboard.push(edge.clone());
                }
            }
            let n_edges = self.edge_clipboard.len();
            let n_nodes = self.clipboard.len();
            let msg = if n_edges > 0 {
                format!("Copied {} nodes, {} edges", n_nodes, n_edges)
            } else {
                format!("Copied {} node{}", n_nodes, if n_nodes == 1 { "" } else { "s" })
            };
            self.status_message = Some((msg, std::time::Instant::now()));
        }

        // Cmd+V = paste nodes + their internal edges (progressive offset)
        if ctx.input(|i| i.key_pressed(Key::V) && i.modifiers.matches_exact(cmd))
            && !self.clipboard.is_empty()
        {
            self.selection.clear();
            self.paste_count += 1;
            let step = 24.0_f32 * self.paste_count as f32;
            let paste_offset = Vec2::new(step, step);

            // Compute clipboard centroid in world space
            let n = self.clipboard.len() as f32;
            let centroid: Vec2 = self.clipboard.iter().fold(Vec2::ZERO, |acc, nd| acc + nd.pos().to_vec2()) / n;
            // Compute viewport center in world space + progressive offset
            let vp_center: Vec2 = (self.canvas_rect.center().to_vec2()
                - Vec2::new(self.viewport.offset[0], self.viewport.offset[1]))
                / self.viewport.zoom;
            let shift: Vec2 = vp_center - centroid + paste_offset / self.viewport.zoom;

            // Build old→new NodeId mapping
            let mut id_map: std::collections::HashMap<NodeId, NodeId> = std::collections::HashMap::new();
            for template in self.clipboard.clone() {
                let old_id = template.id;
                let mut node = template;
                node.id = NodeId::new();
                node.set_pos(node.pos() + shift);
                id_map.insert(old_id, node.id);
                self.selection.node_ids.insert(node.id);
                self.document.nodes.push(node);
            }

            // Recreate edges with remapped IDs
            let edge_templates = self.edge_clipboard.clone();
            let mut pasted_edges = 0usize;
            for mut edge in edge_templates {
                if let (Some(&new_src), Some(&new_tgt)) = (
                    id_map.get(&edge.source.node_id),
                    id_map.get(&edge.target.node_id),
                ) {
                    edge.id = EdgeId::new();
                    edge.source.node_id = new_src;
                    edge.target.node_id = new_tgt;
                    self.document.edges.push(edge);
                    pasted_edges += 1;
                }
            }

            self.history.push(&self.document);
            let n_pasted = self.selection.node_ids.len();
            let msg = if pasted_edges > 0 {
                format!("Pasted {} nodes + {} edges ×{}", n_pasted, pasted_edges, self.paste_count)
            } else {
                format!("Pasted ×{}", self.paste_count)
            };
            self.status_message = Some((msg, std::time::Instant::now()));
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

        // Shift+N = add connected node to the right of the selected node
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::N) && i.modifiers.shift && !i.modifiers.command) {
            if let Some(&src_id) = self.selection.node_ids.iter().next() {
                let (src_pos, src_size) = self.document.find_node(&src_id)
                    .map(|n| (n.pos(), n.size_vec()))
                    .unwrap_or((egui::Pos2::ZERO, egui::Vec2::new(140.0, 60.0)));
                let gap = 60.0;
                let new_pos = egui::Pos2::new(src_pos.x + src_size.x + gap, src_pos.y);
                let mut new_node = crate::model::Node::new(crate::model::NodeShape::Rectangle, new_pos);
                // Copy style from source
                if let Some(src_node) = self.document.find_node(&src_id) {
                    new_node.style = src_node.style.clone();
                }
                let new_id = new_node.id;
                self.document.nodes.push(new_node);
                // Create edge: src -> new (right to left)
                let edge = crate::model::Edge {
                    id: EdgeId::new(),
                    source: crate::model::Port { node_id: src_id, side: crate::model::PortSide::Right },
                    target: crate::model::Port { node_id: new_id, side: crate::model::PortSide::Left },
                    label: String::new(),
                    source_label: String::new(),
                    target_label: String::new(),
                    source_cardinality: crate::model::Cardinality::None,
                    target_cardinality: crate::model::Cardinality::None,
                    style: crate::model::EdgeStyle::default(),
                };
                self.document.edges.push(edge);
                self.selection.select_node(new_id);
                self.focus_label_edit = true;
                self.history.push(&self.document);
                self.status_message = Some(("Connected node added".to_string(), std::time::Instant::now()));
            }
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

        // Cmd+0 = reset zoom (smooth animated)
        if ctx.input(|i| i.key_pressed(Key::Num0) && i.modifiers.matches_exact(cmd)) {
            self.zoom_target = 1.0;
            self.pan_target = Some([0.0, 0.0]);
        }

        // F = fit selection (or all if nothing selected)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.is_none()) {
            if !self.selection.is_empty() {
                self.zoom_to_selection();
            } else {
                self.fit_to_content();
            }
        }

        // Cmd+D = duplicate selected nodes (smart offset avoids stacking)
        if ctx.input(|i| i.key_pressed(Key::D) && i.modifiers.matches_exact(cmd))
            && !self.selection.node_ids.is_empty()
        {
            let base_offset = Vec2::new(24.0, 24.0);
            let originals: Vec<crate::model::Node> = self.selection.node_ids.iter()
                .filter_map(|id| self.document.find_node(id).cloned())
                .collect();
            self.selection.clear();
            for template in originals {
                let mut node = template.clone();
                node.id = NodeId::new();
                // Find a non-overlapping position by nudging by multiples of base_offset
                let mut candidate = template.pos() + base_offset;
                let mut attempts = 0;
                while attempts < 8 {
                    let snap_r = egui::Rect::from_min_size(candidate, node.size_vec());
                    let overlaps = self.document.nodes.iter().any(|n| {
                        n.rect().expand(-4.0).intersects(snap_r)
                    });
                    if !overlaps { break; }
                    candidate = candidate + base_offset;
                    attempts += 1;
                }
                node.set_pos(candidate);
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

        // Cmd+Shift+H = collapse/expand selected nodes
        let cmd_shift_h = Modifiers { shift: true, ..cmd };
        if ctx.input(|i| i.key_pressed(Key::H) && i.modifiers.matches_exact(cmd_shift_h)) {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            if !ids.is_empty() {
                let all_collapsed = ids.iter().all(|id| {
                    self.document.find_node(id).map_or(false, |n| n.collapsed)
                });
                for id in &ids {
                    if let Some(node) = self.document.find_node_mut(id) {
                        if all_collapsed { node.toggle_collapsed(); } // expand
                        else if !node.collapsed { node.toggle_collapsed(); } // collapse only uncollapsed ones
                    }
                }
                self.history.push(&self.document);
                let msg = if all_collapsed { "Expanded" } else { "Collapsed" };
                self.status_message = Some((msg.to_string(), std::time::Instant::now()));
            }
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

        // Cmd+L = auto-layout (hierarchical, animated)
        if ctx.input(|i| i.key_pressed(Key::L) && i.modifiers.matches_exact(cmd)) {
            // Compute hierarchical layout on a document clone, then animate toward results
            let mut doc_clone = self.document.clone();
            for node in doc_clone.nodes.iter_mut() {
                if !node.pinned { node.position = [0.0, 0.0]; }
            }
            crate::specgraph::layout::hierarchical_layout(&mut doc_clone);
            // Store final positions as animation targets without touching the live document
            self.layout_targets.clear();
            for node in &doc_clone.nodes {
                self.layout_targets.insert(node.id, node.position);
            }
            self.pending_fit = true;
            self.status_message = Some(("Layout animating…".to_string(), std::time::Instant::now()));
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

        // Shift+P = toggle quick-notes panel
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::P) && i.modifiers.shift && !i.modifiers.command) {
            self.show_quick_notes = !self.show_quick_notes;
            let msg = if self.show_quick_notes { "Quick Notes On" } else { "Quick Notes Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // H = toggle connectivity heatmap
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::H) && i.modifiers.is_none()) {
            self.show_heatmap = !self.show_heatmap;
            let msg = if self.show_heatmap { "Heatmap On" } else { "Heatmap Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // Shift+A = toggle data-flow animation
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::A) && i.modifiers.shift_only()) {
            self.show_flow_animation = !self.show_flow_animation;
            let msg = if self.show_flow_animation { "Flow Animation On" } else { "Flow Animation Off" };
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

        // Arrow keys on selected node = navigate to adjacent node by spatial direction
        if !any_text_focused && self.selection.node_ids.len() == 1 && self.selection.edge_ids.is_empty() {
            let sel_id = *self.selection.node_ids.iter().next().unwrap();
            let shift = ctx.input(|i| i.modifiers.shift);
            if !shift { // only without shift (shift+arrow = nudge node position)
                let (left, right, up, down) = ctx.input(|i| (
                    i.key_pressed(Key::ArrowLeft),
                    i.key_pressed(Key::ArrowRight),
                    i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::ArrowDown),
                ));
                let nav_dir = if left { Some((-1.0_f32, 0.0_f32)) }
                    else if right { Some((1.0, 0.0)) }
                    else if up { Some((0.0, -1.0)) }
                    else if down { Some((0.0, 1.0)) }
                    else { None };

                if let Some((dx, dy)) = nav_dir {
                    if let Some(sel_node) = self.document.find_node(&sel_id) {
                        let sel_center = sel_node.rect().center();
                        // Find all neighbors connected by edges
                        let neighbors: Vec<NodeId> = self.document.edges.iter()
                            .filter_map(|e| {
                                if e.source.node_id == sel_id { Some(e.target.node_id) }
                                else if e.target.node_id == sel_id { Some(e.source.node_id) }
                                else { None }
                            })
                            .filter(|nid| *nid != sel_id)
                            .collect();

                        // Also include spatial neighbors (all nodes in roughly this direction)
                        let all_candidates: Vec<NodeId> = if neighbors.is_empty() {
                            self.document.nodes.iter().map(|n| n.id).filter(|id| *id != sel_id).collect()
                        } else {
                            neighbors
                        };

                        // Pick the neighbor closest in the requested direction
                        let best = all_candidates.iter().filter_map(|nid| {
                            self.document.find_node(nid).map(|n| {
                                let nc = n.rect().center();
                                let delta = nc - sel_center;
                                // Project onto direction vector — must be positive (in correct direction)
                                let proj = delta.x * dx + delta.y * dy;
                                let lateral = (delta.x * dy - delta.y * dx).abs(); // perpendicular component
                                (proj, lateral, *nid)
                            })
                        })
                        .filter(|(proj, _, _)| *proj > 10.0) // must be in that direction
                        .min_by(|a, b| {
                            // Sort by: best directional score = high proj, low lateral
                            let score_a = a.1 / (a.0 + 1.0);
                            let score_b = b.1 / (b.0 + 1.0);
                            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                        });

                        if let Some((_, _, target_id)) = best {
                            self.selection.select_node(target_id);
                            if let Some(n) = self.document.find_node(&target_id) {
                                let c = self.canvas_rect.center();
                                let np = n.rect().center();
                                self.viewport.offset[0] = c.x - np.x * self.viewport.zoom;
                                self.viewport.offset[1] = c.y - np.y * self.viewport.zoom;
                            }
                        }
                    }
                }
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

        // Shift+L = force-directed auto-layout (animated transition)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::L) && i.modifiers.shift && !i.modifiers.command) {
            self.run_animated_force_layout();
        }

        // Shift+H = distribute selected nodes horizontally with equal gaps
        // Shift+V = distribute selected nodes vertically with equal gaps
        if !any_text_focused && self.selection.node_ids.len() >= 3 {
            let distribute_h = ctx.input(|i| i.key_pressed(Key::H) && i.modifiers.shift && !i.modifiers.command);
            let distribute_v = ctx.input(|i| i.key_pressed(Key::V) && i.modifiers.shift && !i.modifiers.command);

            if distribute_h || distribute_v {
                let mut selected: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                if distribute_h {
                    // Sort by X position
                    selected.sort_by(|a, b| {
                        let xa = self.document.find_node(a).map_or(0.0, |n| n.position[0]);
                        let xb = self.document.find_node(b).map_or(0.0, |n| n.position[0]);
                        xa.partial_cmp(&xb).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    // Compute total width of nodes and available span
                    let first = self.document.find_node(selected.first().unwrap()).map(|n| n.pos().x).unwrap_or(0.0);
                    let last_node = self.document.find_node(selected.last().unwrap());
                    let last_right = last_node.map_or(0.0, |n| n.pos().x + n.size[0]);
                    let total_node_w: f32 = selected.iter()
                        .filter_map(|id| self.document.find_node(id))
                        .map(|n| n.size[0])
                        .sum();
                    let span = last_right - first;
                    let gap = (span - total_node_w) / (selected.len() - 1) as f32;
                    let mut x_cursor = first;
                    for id in &selected {
                        if let Some(node) = self.document.find_node_mut(id) {
                            node.position[0] = x_cursor;
                            x_cursor += node.size[0] + gap;
                        }
                    }
                } else {
                    // Sort by Y position
                    selected.sort_by(|a, b| {
                        let ya = self.document.find_node(a).map_or(0.0, |n| n.position[1]);
                        let yb = self.document.find_node(b).map_or(0.0, |n| n.position[1]);
                        ya.partial_cmp(&yb).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let first = self.document.find_node(selected.first().unwrap()).map(|n| n.pos().y).unwrap_or(0.0);
                    let last_node = self.document.find_node(selected.last().unwrap());
                    let last_bot = last_node.map_or(0.0, |n| n.pos().y + n.size[1]);
                    let total_node_h: f32 = selected.iter()
                        .filter_map(|id| self.document.find_node(id))
                        .map(|n| n.size[1])
                        .sum();
                    let span = last_bot - first;
                    let gap = (span - total_node_h) / (selected.len() - 1) as f32;
                    let mut y_cursor = first;
                    for id in &selected {
                        if let Some(node) = self.document.find_node_mut(id) {
                            node.position[1] = y_cursor;
                            y_cursor += node.size[1] + gap;
                        }
                    }
                }
                self.history.push(&self.document);
                self.status_message = Some(("Distributed evenly".to_string(), std::time::Instant::now()));
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

        // Enter = chain: create a new node connected to the right of the selected node
        // Shift+Enter = chain downward
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::Enter)) {
            let shift = ctx.input(|i| i.modifiers.shift);
            self.chain_create_node(shift);
        }
    }

    /// Compute force-directed layout targets and kick off animated transition.
    pub(crate) fn run_animated_force_layout(&mut self) {
        let target_ids: Vec<NodeId> = if self.selection.node_ids.len() >= 2 {
            self.selection.node_ids.iter().copied().collect()
        } else {
            self.document.nodes.iter().filter(|n| !n.locked && !n.is_frame).map(|n| n.id).collect()
        };

        // Work on a scratch position map so we don't mutate the document yet
        let mut positions: std::collections::HashMap<NodeId, egui::Pos2> = target_ids.iter()
            .filter_map(|id| self.document.find_node(id).map(|n| (*id, n.pos())))
            .collect();

        let sizes: std::collections::HashMap<NodeId, egui::Vec2> = target_ids.iter()
            .filter_map(|id| self.document.find_node(id).map(|n| (*id, n.size_vec())))
            .collect();

        // 40 iterations of repulsion + edge attraction on scratch positions
        for _ in 0..40 {
            let mut forces: std::collections::HashMap<NodeId, egui::Vec2> =
                target_ids.iter().map(|id| (*id, egui::Vec2::ZERO)).collect();

            for i in 0..target_ids.len() {
                for j in (i + 1)..target_ids.len() {
                    let ai = target_ids[i];
                    let aj = target_ids[j];
                    let pi = positions[&ai];
                    let pj = positions[&aj];
                    let si = sizes.get(&ai).copied().unwrap_or(egui::Vec2::new(100.0, 60.0));
                    let sj = sizes.get(&aj).copied().unwrap_or(egui::Vec2::new(100.0, 60.0));
                    let ri = egui::Rect::from_min_size(pi, si).expand(20.0);
                    let rj = egui::Rect::from_min_size(pj, sj).expand(20.0);
                    let diff = ri.center() - rj.center();
                    let dist = diff.length().max(0.01);
                    let ideal = (ri.width() + rj.width()) * 0.55 + (ri.height() + rj.height()) * 0.25;
                    if dist < ideal {
                        let force = diff.normalized() * (ideal - dist) * 0.5;
                        *forces.entry(ai).or_default() += force;
                        *forces.entry(aj).or_default() -= force;
                    }
                }
            }

            for edge in &self.document.edges {
                let (sid, tid) = (edge.source.node_id, edge.target.node_id);
                if !forces.contains_key(&sid) || !forces.contains_key(&tid) { continue; }
                let ps = positions[&sid];
                let pt = positions[&tid];
                let ss = sizes.get(&sid).copied().unwrap_or(egui::Vec2::new(100.0, 60.0));
                let st = sizes.get(&tid).copied().unwrap_or(egui::Vec2::new(100.0, 60.0));
                let ideal_edge = (ss.x + st.x) * 0.5 + 80.0;
                let diff = pt - ps;
                let dist = diff.length().max(0.01);
                let attract = diff.normalized() * (dist - ideal_edge) * 0.1;
                *forces.entry(sid).or_default() += attract;
                *forces.entry(tid).or_default() -= attract;
            }

            for id in &target_ids {
                if let Some(p) = positions.get_mut(id) {
                    if let Some(f) = forces.get(id) {
                        let clamped = egui::Vec2::new(f.x.clamp(-40.0, 40.0), f.y.clamp(-40.0, 40.0));
                        *p += clamped;
                    }
                }
            }
        }

        // Store final positions as animation targets
        self.layout_targets.clear();
        for (id, pos) in positions {
            self.layout_targets.insert(id, [pos.x, pos.y]);
        }
        self.status_message = Some(("Layout animating…".to_string(), std::time::Instant::now()));
    }

    /// Advance the animated layout one frame; call each frame while layout_targets is non-empty.
    pub(crate) fn step_layout_animation(&mut self, dt: f32, ctx: &egui::Context) {
        if self.layout_targets.is_empty() { return; }

        let lerp = 1.0 - 0.75_f32.powf(dt * 60.0); // ~60% per frame at 60fps
        let mut settled = true;

        let ids: Vec<NodeId> = self.layout_targets.keys().copied().collect();
        for id in &ids {
            let target = match self.layout_targets.get(id) { Some(t) => *t, None => continue };
            if let Some(node) = self.document.find_node_mut(id) {
                if node.pinned || node.locked { continue; }
                let dx = target[0] - node.position[0];
                let dy = target[1] - node.position[1];
                if dx.abs() > 0.5 || dy.abs() > 0.5 {
                    node.position[0] += dx * lerp;
                    node.position[1] += dy * lerp;
                    settled = false;
                } else {
                    node.position[0] = target[0];
                    node.position[1] = target[1];
                }
            }
        }

        if settled {
            self.layout_targets.clear();
            self.history.push(&self.document);
            self.status_message = Some(("Layout complete".to_string(), std::time::Instant::now()));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    /// Create a new node connected to the currently selected node.
    /// If `downward` is false, place it to the right; if true, place it below.
    fn chain_create_node(&mut self, downward: bool) {
        // Only act when exactly one node is selected
        let sel_id = match self.selection.node_ids.iter().next().copied() {
            Some(id) if self.selection.node_ids.len() == 1 => id,
            _ => return,
        };
        let (new_pos, new_size, src_side, tgt_side) = {
            let node = match self.document.find_node(&sel_id) {
                Some(n) => n,
                None => return,
            };
            let gap = 60.0_f32;
            if downward {
                let pos = egui::Pos2::new(
                    node.position[0],
                    node.position[1] + node.size[1] + gap,
                );
                (pos, node.size, PortSide::Bottom, PortSide::Top)
            } else {
                let pos = egui::Pos2::new(
                    node.position[0] + node.size[0] + gap,
                    node.position[1],
                );
                (pos, node.size, PortSide::Right, PortSide::Left)
            }
        };

        // Clone the shape kind from parent, but clear the label
        let shape = match self.document.find_node(&sel_id) {
            Some(n) => match &n.kind {
                NodeKind::Shape { shape, .. } => *shape,
                _ => NodeShape::Rectangle,
            },
            None => NodeShape::Rectangle,
        };

        let mut new_node = Node::new(shape, new_pos);
        new_node.size = new_size;
        // Inherit parent's style
        if let Some(parent) = self.document.find_node(&sel_id) {
            new_node.style = parent.style.clone();
        }
        let new_id = new_node.id;
        self.document.nodes.push(new_node);

        // Connect with an edge from parent → new
        let edge = Edge::new(
            Port { node_id: sel_id, side: src_side },
            Port { node_id: new_id,  side: tgt_side },
        );
        self.document.edges.push(edge);

        // Select new node and focus its label for immediate rename
        self.selection.clear();
        self.selection.select_node(new_id);
        self.focus_label_edit = true;
        self.history.push(&self.document);

        let dir = if downward { "below" } else { "right" };
        self.status_message = Some((
            format!("New node → chained {dir} (Enter to continue)"),
            std::time::Instant::now(),
        ));
    }
}
