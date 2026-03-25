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
            // Capture ghost data before removal for shrink-fade animation
            let now = ctx.input(|i| i.time);
            for id in &node_ids {
                if let Some(n) = self.document.find_node(id) {
                    let c = n.rect().center();
                    self.deletion_ghosts.push(([c.x, c.y], n.size, n.style.fill_color, now));
                }
            }
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

        // Cmd+B = toggle bold for selected nodes
        if ctx.input(|i| i.key_pressed(Key::B) && i.modifiers.matches_exact(cmd)) && !self.selection.node_ids.is_empty() {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            let all_bold = ids.iter().all(|id| self.document.find_node(id).map_or(false, |n| n.style.bold));
            for id in &ids {
                if let Some(node) = self.document.find_node_mut(id) {
                    node.style.bold = !all_bold;
                }
            }
            self.history.push(&self.document);
        }

        // B (no modifier) = toggle border dashed for selected nodes
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::B) && i.modifiers.is_none()) && !self.selection.node_ids.is_empty() {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            let all_dashed = ids.iter().all(|id| self.document.find_node(id).map_or(false, |n| n.style.border_dashed));
            for id in &ids {
                if let Some(node) = self.document.find_node_mut(id) {
                    node.style.border_dashed = !all_dashed;
                }
            }
            self.history.push(&self.document);
            let label = if !all_dashed { "Dashed border" } else { "Solid border" };
            self.status_message = Some((label.to_string(), std::time::Instant::now()));
        }

        // Cmd+I = toggle italic for selected nodes
        if ctx.input(|i| i.key_pressed(Key::I) && i.modifiers.matches_exact(cmd)) && !self.selection.node_ids.is_empty() {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            let all_italic = ids.iter().all(|id| self.document.find_node(id).map_or(false, |n| n.style.italic));
            for id in &ids {
                if let Some(node) = self.document.find_node_mut(id) {
                    node.style.italic = !all_italic;
                }
            }
            self.history.push(&self.document);
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

        // Cmd+V with empty node clipboard + text in system clipboard → create node from text
        let paste_text: Option<String> = ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Paste(text) = event {
                    if !text.trim().is_empty() {
                        return Some(text.clone());
                    }
                }
            }
            None
        });
        if let Some(pasted_text) = paste_text {
            if self.clipboard.is_empty() && !any_text_focused {
                let center = self.canvas_rect.center();
                let world_center = self.viewport.screen_to_canvas(center);
                let section = self.section_at_canvas_pos(world_center);

                // Multi-line paste (2+ non-empty lines) → one node per line
                let lines: Vec<&str> = pasted_text.lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .collect();

                if lines.len() >= 2 {
                    let row_h = 55.0_f32;
                    let total_h = row_h * lines.len() as f32;
                    let start_y = world_center.y - total_h / 2.0;
                    self.selection.clear();
                    for (i, line) in lines.iter().enumerate() {
                        let pos = egui::Pos2::new(world_center.x - 80.0, start_y + i as f32 * row_h);
                        let mut node = Node::new(crate::model::NodeShape::RoundedRect, pos);
                        node.size = [160.0, 44.0];
                        if let crate::model::NodeKind::Shape { ref mut label, .. } = node.kind {
                            *label = (*line).to_string();
                        }
                        if let Some(ref sec) = section {
                            node.section_name = sec.clone();
                        }
                        let id = node.id;
                        self.node_birth_times.insert(id, ctx.input(|i| i.time));
                        self.document.nodes.push(node);
                        self.selection.select_node(id);
                    }
                    self.history.push(&self.document);
                    self.status_message = Some((format!("Pasted {} items as nodes", lines.len()), std::time::Instant::now()));
                } else {
                    // Single line → sticky note (existing behavior)
                    let mut node = Node::new_sticky(
                        crate::model::StickyColor::Yellow,
                        egui::Pos2::new(world_center.x - 90.0, world_center.y - 60.0),
                    );
                    if let crate::model::NodeKind::StickyNote { ref mut text, .. } = node.kind {
                        *text = pasted_text.chars().take(300).collect();
                    }
                    if let Some(sec) = section {
                        node.section_name = sec;
                    }
                    let id = node.id;
                    let label_copy = pasted_text.chars().take(300).collect::<String>();
                    self.document.nodes.push(node);
                    self.selection.select_node(id);
                    self.inline_node_edit = Some((id, label_copy));
                    self.history.push(&self.document);
                    self.status_message = Some(("Text pasted as sticky note — editing".to_string(), std::time::Instant::now()));
                }
            }
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
                    comment: String::new(),
                };
                self.document.edges.push(edge);
                self.selection.select_node(new_id);
                self.focus_label_edit = true;
                self.history.push(&self.document);
                self.status_message = Some(("Connected node added".to_string(), std::time::Instant::now()));
            }
        }

        // R key (no modifier) + selection = resolve ticket (Done + last section); without selection = Rectangle
        if !any_text_focused && !self.selection.node_ids.is_empty()
            && ctx.input(|i| i.key_pressed(Key::R) && i.modifiers.is_none())
        {
            // Find last section in doc order
            let mut seen_sections: Vec<String> = Vec::new();
            for n in &self.document.nodes {
                if !n.section_name.is_empty() && !seen_sections.contains(&n.section_name) {
                    seen_sections.push(n.section_name.clone());
                }
            }
            let resolve_sec: Option<String> = seen_sections.last().cloned();
            let ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
            let count = ids.len();
            for id in &ids {
                if let Some(n) = self.document.find_node_mut(id) {
                    n.tag = Some(crate::model::NodeTag::Ok);
                    n.progress = 1.0;
                    if let Some(ref sec) = resolve_sec {
                        n.section_name = sec.clone();
                    }
                }
            }
            self.history.push(&self.document);
            // Animated re-layout
            let mut doc_clone = self.document.clone();
            for n in doc_clone.nodes.iter_mut() { if !n.pinned { n.position = [0.0, 0.0]; } }
            crate::specgraph::layout::auto_layout(&mut doc_clone);
            self.layout_targets.clear();
            for n in &doc_clone.nodes { self.layout_targets.insert(n.id, n.position); }
            let msg = if let Some(ref sec) = resolve_sec {
                if count == 1 { format!("✓ Resolved → {}", sec) }
                else { format!("✓ Resolved {} tickets → {}", count, sec) }
            } else { "✓ Resolved".to_string() };
            self.status_message = Some((msg, std::time::Instant::now()));
        }

        // Quick shape creation shortcuts (skip when editing text)
        let shape_to_create: Option<crate::model::NodeShape> = if !any_text_focused {
            if ctx.input(|i| i.key_pressed(Key::R) && i.modifiers.is_none()) && self.selection.node_ids.is_empty() {
                Some(crate::model::NodeShape::Rectangle)
            } else if ctx.input(|i| i.key_pressed(Key::C) && i.modifiers.is_none()) && self.selection.node_ids.is_empty() {
                Some(crate::model::NodeShape::Circle)
            } else if ctx.input(|i| i.key_pressed(Key::D) && i.modifiers.is_none()) && self.selection.node_ids.is_empty() {
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
            // Auto-assign to kanban section if cursor is within a column band
            if let Some(sec) = self.section_for_canvas_x(canvas_center.x) {
                node.section_name = sec;
            }
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

        // Design thinking quick-create shortcuts (skip when editing text)
        // (shape, fill, label, status_msg)
        let dt_presets: &[(crate::model::NodeShape, [u8;4], &str, &str)] = &[
            (crate::model::NodeShape::Diamond,       [250, 179, 135, 255], "Hypothesis",  "H"),
            (crate::model::NodeShape::Parallelogram, [137, 180, 250, 255], "Assumption",  "Y"),
            (crate::model::NodeShape::Rectangle,     [166, 227, 161, 255], "Evidence",    "W"),
        ];
        // H = Hypothesis, Y = Assumption (A conflicts with select all), W = evidence (E conflicts with Connect)
        let dt_keys = [Key::H, Key::Y, Key::W];
        for ((&key, preset)) in dt_keys.iter().zip(dt_presets.iter()) {
            if !any_text_focused && self.selection.node_ids.is_empty() && ctx.input(|i| i.key_pressed(key) && i.modifiers.is_none()) {
                let (shape, fill, label, status) = preset;
                let canvas_center = {
                    let c = self.canvas_rect.center();
                    self.viewport.screen_to_canvas(c)
                };
                let mut node = crate::model::Node::new(*shape, canvas_center);
                let w = node.size[0]; let h = node.size[1];
                node.set_pos(egui::Pos2::new(canvas_center.x - w / 2.0, canvas_center.y - h / 2.0));
                node.style.fill_color = *fill;
                node.style.text_color = crate::app::theme::auto_contrast_text(*fill);
                if let crate::model::NodeKind::Shape { label: ref mut l, .. } = node.kind {
                    *l = label.to_string();
                }
                let id = node.id;
                self.document.nodes.push(node);
                self.selection.select_node(id);
                self.focus_label_edit = true;
                self.history.push(&self.document);
                self.status_message = Some((format!("{status}: {label} created"), std::time::Instant::now()));
            }
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

        // Cmd+Shift+> = increase font size for selected nodes
        // Cmd+Shift+< = decrease font size for selected nodes
        let cmd_shift = Modifiers { shift: true, ..cmd };
        let font_up   = ctx.input(|i| i.key_pressed(Key::Period) && i.modifiers.matches_exact(cmd_shift));
        let font_down = ctx.input(|i| i.key_pressed(Key::Comma)  && i.modifiers.matches_exact(cmd_shift));
        if (font_up || font_down) && !self.selection.node_ids.is_empty() {
            let delta: f32 = if font_up { 1.0 } else { -1.0 };
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            for id in &ids {
                if let Some(node) = self.document.find_node_mut(id) {
                    node.style.font_size = (node.style.font_size + delta).clamp(6.0, 72.0);
                }
            }
            self.history.push(&self.document);
            let new_size = self.document.find_node(ids.first().unwrap())
                .map(|n| n.style.font_size as i32).unwrap_or(13);
            self.status_message = Some((format!("Font size: {new_size}pt"), std::time::Instant::now()));
        }

        // F = fit selection (or all if nothing selected)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.is_none()) {
            if self.view_mode == super::ViewMode::ThreeD {
                self.fit_3d_to_content(ctx);
            } else if !self.selection.is_empty() {
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
            // Compute max numeric hrf_id suffix before the loop so we can increment per-duplicate
            let mut max_t_dup: u32 = self.document.nodes.iter()
                .filter_map(|n| n.hrf_id.strip_prefix('t').and_then(|s| s.parse::<u32>().ok()))
                .max().unwrap_or(0);
            for template in originals {
                let mut node = template.clone();
                node.id = NodeId::new();
                // Generate a new hrf_id for the duplicate (avoid collisions)
                if !node.hrf_id.is_empty() {
                    max_t_dup += 1;
                    node.hrf_id = format!("t{}", max_t_dup);
                }
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

        // F2 = rename: focus the label editor in the properties panel
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::F2)) {
            if !self.selection.node_ids.is_empty() || !self.selection.edge_ids.is_empty() {
                self.focus_label_edit = true;
            }
        }

        // Shift+R = toggle coordinate rulers
        let shift_only = egui::Modifiers { shift: true, ..Default::default() };
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::R) && i.modifiers.matches_exact(shift_only)) {
            self.show_rulers = !self.show_rulers;
        }

        // Cmd+F = search (also clears persistent filter)
        if ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.matches_exact(cmd)) {
            if self.persist_search_filter {
                // Second Cmd+F clears the pinned filter
                self.persist_search_filter = false;
                self.search_query.clear();
                self.show_search = false;
                self.status_message = Some(("Filter cleared".to_string(), std::time::Instant::now()));
            } else {
                self.show_search = !self.show_search;
                if self.show_search { self.search_query.clear(); }
            }
        }

        // Cmd+N = open template gallery
        if ctx.input(|i| i.key_pressed(Key::N) && i.modifiers.matches_exact(cmd)) {
            self.show_template_gallery = !self.show_template_gallery;
        }

        // Cmd+E = live spec editor panel (populate with current HRF on open)
        if ctx.input(|i| i.key_pressed(Key::E) && i.modifiers.matches_exact(cmd)) {
            self.show_spec_editor = !self.show_spec_editor;
            if self.show_spec_editor {
                let title = self.document.title.clone();
                let is_3d = matches!(self.view_mode, super::ViewMode::ThreeD);
                let bg_str = match self.bg_pattern {
                    super::BgPattern::Dots       => "dots",
                    super::BgPattern::Lines      => "lines",
                    super::BgPattern::Crosshatch => "crosshatch",
                    super::BgPattern::None       => "none",
                };
                let vp = crate::specgraph::hrf::ViewportExportConfig {
                    bg_pattern: bg_str,
                    snap: self.snap_to_grid,
                    grid_size: self.grid_size,
                    zoom: self.viewport.zoom,
                    view_3d: is_3d,
                    camera_yaw:   if is_3d { Some(self.camera3d.yaw) }   else { None },
                    camera_pitch: if is_3d { Some(self.camera3d.pitch) } else { None },
                };
                self.spec_editor_text = crate::specgraph::hrf::export_hrf_ex(&self.document, &title, Some(&vp));
                self.spec_editor_error = None;
            }
        }

        // Cmd+Shift+E = insert Quick Experiment Card (Hypothesis → Test → Result → Learning)
        {
            let cmd_shift_e = egui::Modifiers { command: true, shift: true, ..Default::default() };
            if !any_text_focused && ctx.input(|i| i.key_pressed(Key::E) && i.modifiers.matches_exact(cmd_shift_e)) {
                let center = ctx.input(|i| i.pointer.hover_pos())
                    .map(|s| self.viewport.screen_to_canvas(s))
                    .unwrap_or_else(|| {
                        let c = self.canvas_rect.center();
                        self.viewport.screen_to_canvas(c)
                    });
                // Create 4 nodes spaced 160px apart, left-aligned around center
                let spacing = 160.0_f32;
                let total_w = spacing * 3.0;
                let start_x = center.x - total_w / 2.0;
                let y = center.y - 30.0;

                let labels = ["Hypothesis", "Test Method", "Result", "Learning"];
                let fills: [[u8; 4]; 4] = [
                    [137, 180, 250, 255],  // blue
                    [203, 166, 247, 255],  // purple
                    [166, 227, 161, 255],  // green
                    [249, 226, 175, 255],  // yellow
                ];
                let section = "Experiments";
                let mut ids = Vec::new();
                for (k, (&lbl, &fill)) in labels.iter().zip(fills.iter()).enumerate() {
                    let pos = egui::Pos2::new(start_x + k as f32 * spacing, y);
                    let mut node = crate::model::Node::new(crate::model::NodeShape::RoundedRect, pos);
                    node.size = [130.0, 50.0];
                    if let crate::model::NodeKind::Shape { ref mut label, .. } = node.kind {
                        *label = lbl.to_string();
                    }
                    node.style.fill_color = fill;
                    node.style.text_color = crate::app::theme::auto_contrast_text(fill);
                    node.section_name = section.to_string();
                    ids.push(node.id);
                    self.document.nodes.push(node);
                    self.node_birth_times.insert(*ids.last().unwrap(), ctx.input(|i| i.time));
                }
                // Connect the 4 nodes in a chain
                for w in ids.windows(2) {
                    let edge = crate::model::Edge::new(
                        crate::model::Port { node_id: w[0], side: crate::model::PortSide::Right },
                        crate::model::Port { node_id: w[1], side: crate::model::PortSide::Left  },
                    );
                    self.document.edges.push(edge);
                }
                // Select and start editing the first node
                self.selection.clear();
                self.selection.select_node(ids[0]);
                self.inline_node_edit = Some((ids[0], "Hypothesis".to_string()));
                self.history.push(&self.document);
                self.status_message = Some(("Experiment card created — edit label to begin".to_string(), std::time::Instant::now()));
            }
        }

        // Cmd+Shift+I = new intake ticket: creates a single ticket in the first section, opens for editing
        {
            let cmd_shift_i = egui::Modifiers { command: true, shift: true, ..Default::default() };
            if !any_text_focused && ctx.input(|i| i.key_pressed(Key::I) && i.modifiers.matches_exact(cmd_shift_i)) {
                // Find first section in document order
                let first_section: String = self.document.nodes.iter()
                    .find(|n| !n.section_name.is_empty())
                    .map(|n| n.section_name.clone())
                    .unwrap_or_else(|| "Intake".to_string());
                let today = super::render::today_iso();
                let canvas_center = {
                    let c = self.canvas_rect.center();
                    self.viewport.screen_to_canvas(c)
                };
                // Auto-generate hrf_id: find max numeric suffix of existing ids prefixed with 't'
                let max_t_id: u32 = self.document.nodes.iter()
                    .filter_map(|n| {
                        n.hrf_id.strip_prefix('t')
                            .and_then(|s| s.parse::<u32>().ok())
                    })
                    .max()
                    .unwrap_or(0);
                let new_hrf_id = format!("t{}", max_t_id + 1);
                // Default due = created + SLA days for P3 (from document config, default 7)
                let p3_sla = self.document.sla_days[2].max(1) as i64;
                let due_7d = {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
                    let days7 = ((secs / 86400) + p3_sla) as i32;
                    let z = days7 + 719468;
                    let era = if z >= 0 { z } else { z - 146096 } / 146097;
                    let doe = (z - era * 146097) as u32;
                    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
                    let y = yoe as i32 + era * 400;
                    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
                    let mp = (5 * doy + 2) / 153;
                    let d = doy - (153 * mp + 2) / 5 + 1;
                    let m = if mp < 10 { mp + 3 } else { mp - 9 };
                    format!("{:04}-{:02}-{:02}", y, m, d)
                };
                let mut node = crate::model::Node::new(crate::model::NodeShape::RoundedRect, canvas_center);
                node.size = [160.0, 58.0];
                if let crate::model::NodeKind::Shape { ref mut label, .. } = node.kind {
                    *label = "New Ticket".to_string();
                }
                // Default P3 (medium priority)
                node.tag = Some(crate::model::NodeTag::Info);
                node.priority = 3;
                node.style.fill_color = [137, 180, 250, 255];
                node.style.text_color = crate::app::theme::auto_contrast_text([137, 180, 250, 255]);
                node.section_name = first_section.clone();
                node.sublabel = format!("👤 \n📅 {}", due_7d);
                node.created_date = today;
                node.hrf_id = new_hrf_id.clone();
                let new_id = node.id;
                self.document.nodes.push(node);
                self.node_birth_times.insert(new_id, ctx.input(|i| i.time));
                self.selection.select_node(new_id);
                // Open inline editor for the label
                self.inline_node_edit = Some((new_id, "New Ticket".to_string()));
                self.history.push(&self.document);
                // Animated layout
                let mut doc_clone = self.document.clone();
                for n in doc_clone.nodes.iter_mut() { if !n.pinned { n.position = [0.0, 0.0]; } }
                crate::specgraph::layout::auto_layout(&mut doc_clone);
                self.layout_targets.clear();
                for n in &doc_clone.nodes { self.layout_targets.insert(n.id, n.position); }
                self.status_message = Some((format!("New ticket {} → {}", new_hrf_id, first_section), std::time::Instant::now()));
            }
        }

        // Cmd+Shift+T = insert Quick Support Ticket card (Intake → Triage → In Progress → Resolved)
        {
            let cmd_shift_t = egui::Modifiers { command: true, shift: true, ..Default::default() };
            if !any_text_focused && ctx.input(|i| i.key_pressed(Key::T) && i.modifiers.matches_exact(cmd_shift_t)) {
                let center = ctx.input(|i| i.pointer.hover_pos())
                    .map(|s| self.viewport.screen_to_canvas(s))
                    .unwrap_or_else(|| {
                        let c = self.canvas_rect.center();
                        self.viewport.screen_to_canvas(c)
                    });
                let spacing = 170.0_f32;
                let total_w = spacing * 3.0;
                let start_x = center.x - total_w / 2.0;
                let y = center.y - 30.0;
                // Intake=blue, Triage=purple, In Progress=yellow, Resolved=green
                let labels = ["Intake", "Triage", "In Progress", "Resolved"];
                let fills: [[u8; 4]; 4] = [
                    [137, 180, 250, 255],  // blue — Intake
                    [203, 166, 247, 255],  // purple — Triage
                    [249, 226, 175, 255],  // yellow — In Progress
                    [166, 227, 161, 255],  // green — Resolved
                ];
                let tags = [
                    Some(crate::model::NodeTag::Info),
                    Some(crate::model::NodeTag::Warning),
                    Some(crate::model::NodeTag::Info),
                    Some(crate::model::NodeTag::Ok),
                ];
                let section = "Support";
                let mut ids = Vec::new();
                for (k, ((&lbl, &fill), tag)) in labels.iter().zip(fills.iter()).zip(tags.iter()).enumerate() {
                    let pos = egui::Pos2::new(start_x + k as f32 * spacing, y);
                    let mut node = crate::model::Node::new(crate::model::NodeShape::RoundedRect, pos);
                    node.size = [140.0, 54.0];
                    if let crate::model::NodeKind::Shape { ref mut label, .. } = node.kind {
                        *label = lbl.to_string();
                    }
                    node.style.fill_color = fill;
                    node.style.text_color = crate::app::theme::auto_contrast_text(fill);
                    node.section_name = section.to_string();
                    node.tag = *tag;
                    ids.push(node.id);
                    self.document.nodes.push(node);
                    self.node_birth_times.insert(*ids.last().unwrap(), ctx.input(|i| i.time));
                }
                for w in ids.windows(2) {
                    let edge = crate::model::Edge::new(
                        crate::model::Port { node_id: w[0], side: crate::model::PortSide::Right },
                        crate::model::Port { node_id: w[1], side: crate::model::PortSide::Left  },
                    );
                    self.document.edges.push(edge);
                }
                self.selection.clear();
                self.selection.select_node(ids[0]);
                self.history.push(&self.document);
                self.status_message = Some(("Support ticket card created — edit labels to begin".to_string(), std::time::Instant::now()));
            }
        }

        // Cmd+Shift+F = zoom to fit selected nodes (or all if nothing selected)
        let cmd_shift = egui::Modifiers { command: true, shift: true, ..Default::default() };
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.matches_exact(cmd_shift)) {
            if self.selection.node_ids.is_empty() {
                self.fit_to_content();
            } else {
                self.zoom_to_selection();
            }
        }

        // Cmd+H = find & replace
        if ctx.input(|i| i.key_pressed(Key::H) && i.modifiers.matches_exact(cmd)) {
            self.show_find_replace = !self.show_find_replace;
            if self.show_find_replace { self.find_query.clear(); self.replace_query.clear(); }
        }

        // Cmd+S = save to current path (or open Save As if no path yet)
        // matches_exact: ensures Cmd+Shift+S (shift also held) does NOT trigger this path
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.matches_exact(cmd)) {
            if let Some(path) = self.current_file_path.clone() {
                self.save_to_path(path);
            } else if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Spec / YAML", &["spec", "yaml"])
                    .set_file_name("diagram.spec")
                    .save_file()
            {
                self.save_to_path(path);
            }
        }

        // Cmd+Shift+S = always open Save As dialog
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.matches_exact(cmd_shift)) {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Spec / YAML", &["spec", "yaml"])
                .set_file_name("diagram.spec")
                .save_file()
            {
                self.save_to_path(path);
            }
        }

        // Cmd+Shift+Y = copy current diagram as HRF spec to system clipboard (moved from Cmd+Shift+S)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::Y) && i.modifiers.matches_exact(cmd_shift)) {
            let is_3d = matches!(self.view_mode, super::ViewMode::ThreeD);
            let bg_str = match self.bg_pattern {
                super::BgPattern::Dots      => "dots",
                super::BgPattern::Lines     => "lines",
                super::BgPattern::Crosshatch => "crosshatch",
                super::BgPattern::None      => "none",
            };
            let vp = crate::specgraph::hrf::ViewportExportConfig {
                bg_pattern: bg_str,
                snap: self.snap_to_grid,
                grid_size: self.grid_size,
                zoom: self.viewport.zoom,
                view_3d: is_3d,
                camera_yaw:   if is_3d { Some(self.camera3d.yaw) }   else { None },
                camera_pitch: if is_3d { Some(self.camera3d.pitch) } else { None },
            };
            let hrf = crate::specgraph::hrf::export_hrf_ex(&self.document, "Untitled Diagram", Some(&vp));
            ctx.copy_text(hrf);
            self.status_message = Some((
                "Spec copied to clipboard".to_string(),
                std::time::Instant::now(),
            ));
        }

        // Cmd+Shift+R = copy support status report as Markdown to clipboard
        let cmd_shift_r = Modifiers { shift: true, ..cmd };
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::R) && i.modifiers.matches_exact(cmd_shift_r)) {
            let nodes: Vec<_> = if !self.selection.node_ids.is_empty() {
                self.document.nodes.iter()
                    .filter(|n| self.selection.node_ids.contains(&n.id))
                    .collect()
            } else {
                self.document.nodes.iter().collect()
            };

            // Group by section
            let mut sections: std::collections::BTreeMap<String, Vec<_>> = std::collections::BTreeMap::new();
            for n in &nodes {
                sections.entry(n.section_name.clone()).or_default().push(n);
            }

            let mut md = String::new();
            let title = if let Some(t) = &self.document.import_hints.project_title {
                if !t.is_empty() { t.clone() } else { "Support Report".to_string() }
            } else { "Support Report".to_string() };
            md.push_str(&format!("# {}\n\n", title));

            let today_rpt = super::render::today_iso();
            for (sec, items) in &sections {
                let header = if sec.is_empty() { "General".to_string() } else { sec.clone() };
                md.push_str(&format!("## {}\n\n", header));
                for n in items {
                    if n.is_frame { continue; }
                    let status = if n.priority > 0 {
                        match n.priority { 1 => "🔴", 2 => "🟡", 3 => "🔵", _ => "🟢" }
                    } else {
                        match n.tag {
                            Some(crate::model::NodeTag::Critical) => "🔴",
                            Some(crate::model::NodeTag::Warning)  => "🟡",
                            Some(crate::model::NodeTag::Ok)        => "🟢",
                            Some(crate::model::NodeTag::Info)      => "🔵",
                            None => "⚪",
                        }
                    };
                    let label = n.display_label();
                    let id_part = if !n.hrf_id.is_empty() { format!("[`#{}`] ", n.hrf_id) } else { String::new() };
                    let prio_part = if n.priority > 0 { format!(" `P{}`", n.priority) } else { String::new() };
                    let desc = match &n.kind {
                        crate::model::NodeKind::Shape { description, .. } if !description.is_empty() => {
                            format!(" — {}", description)
                        }
                        _ => String::new(),
                    };
                    let assignee = n.sublabel.lines()
                        .find(|l| l.starts_with("👤 "))
                        .map(|l| format!("  `{}`", l.trim()))
                        .unwrap_or_default();
                    let due = n.sublabel.lines()
                        .find(|l| l.starts_with("📅 "))
                        .map(|l| {
                            let d = l.strip_prefix("📅 ").unwrap_or("").trim();
                            let days = super::render::iso_days_remaining_pub(d, &today_rpt);
                            if days < 0 { format!("  `📅 {} ({}d overdue)`", d, -days) }
                            else if days == 0 { format!("  `📅 {} (TODAY)`", d) }
                            else { format!("  `📅 {} (+{}d)`", d, days) }
                        })
                        .unwrap_or_default();
                    let comment_part = if !n.comment.is_empty() {
                        format!("\n  > 💬 {}", n.comment.chars().take(120).collect::<String>())
                    } else { String::new() };
                    md.push_str(&format!("- {} {}**{}**{}{}{}{}{}\n", status, id_part, label, prio_part, desc, assignee, due, comment_part));
                }
                md.push('\n');
            }

            ctx.copy_text(md);
            let count = nodes.len();
            self.status_message = Some((
                format!("Support report copied ({} nodes)", count),
                std::time::Instant::now(),
            ));
        }

        // Cmd+Shift+X = export nodes as CSV to clipboard (selection or all)
        {
            let cmd_shift_x = Modifiers { shift: true, ..cmd };
            if !any_text_focused && ctx.input(|i| i.key_pressed(Key::X) && i.modifiers.matches_exact(cmd_shift_x)) {
                let nodes: Vec<&crate::model::Node> = if !self.selection.node_ids.is_empty() {
                    self.document.nodes.iter()
                        .filter(|n| self.selection.node_ids.contains(&n.id))
                        .collect()
                } else {
                    self.document.nodes.iter().collect()
                };

                let today_csv = super::render::today_iso();
                let mut csv = String::from("Ticket ID,Label,Section,Priority,Status,Assignee,Due Date,Age (days),URL,Comment\n");
                for n in &nodes {
                    if n.is_frame { continue; }
                    let ticket_id = n.hrf_id.replace('"', "\"\"");
                    let label = n.display_label().replace('"', "\"\"");
                    let section = n.section_name.replace('"', "\"\"");
                    // Use numeric priority field if set, otherwise derive from tag
                    let priority = if n.priority > 0 {
                        match n.priority { 1 => "P1", 2 => "P2", 3 => "P3", _ => "P4" }
                    } else {
                        match n.tag {
                            Some(crate::model::NodeTag::Critical) => "P1",
                            Some(crate::model::NodeTag::Warning)  => "P2",
                            Some(crate::model::NodeTag::Info)      => "P3",
                            Some(crate::model::NodeTag::Ok)        => "P4",
                            None => "",
                        }
                    };
                    let status = match (n.tag, n.progress) {
                        (Some(crate::model::NodeTag::Ok), p) if p >= 0.99 => "Done",
                        (Some(crate::model::NodeTag::Info), _) => "WIP",
                        (Some(crate::model::NodeTag::Warning), p) if p >= 0.75 => "Review",
                        (Some(crate::model::NodeTag::Warning), _) => "Todo",
                        (Some(crate::model::NodeTag::Critical), _) => "Blocked",
                        _ => "",
                    };
                    let assignee = n.sublabel.lines()
                        .find(|l| l.starts_with("👤 "))
                        .and_then(|l| l.strip_prefix("👤 "))
                        .unwrap_or("")
                        .replace('"', "\"\"");
                    let due = n.sublabel.lines()
                        .find(|l| l.starts_with("📅 "))
                        .and_then(|l| l.strip_prefix("📅 "))
                        .unwrap_or("")
                        .replace('"', "\"\"");
                    let age = if !n.created_date.is_empty() {
                        let d = -super::render::iso_days_remaining_pub(&n.created_date, &today_csv);
                        if d >= 0 { d.to_string() } else { String::new() }
                    } else { String::new() };
                    let url = n.url.replace('"', "\"\"");
                    let comment = n.comment.replace('"', "\"\"").replace('\n', " ");
                    csv.push_str(&format!(
                        "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                        ticket_id, label, section, priority, status, assignee, due, age, url, comment
                    ));
                }

                ctx.copy_text(csv);
                let count = nodes.len();
                self.status_message = Some((
                    format!("CSV copied — {} rows", count),
                    std::time::Instant::now(),
                ));
            }
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

        // Bookmark shortcuts: Cmd+Shift+1..5 = save, Shift+1..5 = jump
        let cmd_shift = Modifiers { shift: true, ..cmd };
        let shift_only = Modifiers { shift: true, ..Modifiers::NONE };
        let bookmark_keys = [
            (Key::Num1, 0), (Key::Num2, 1), (Key::Num3, 2),
            (Key::Num4, 3), (Key::Num5, 4),
        ];
        for &(key, slot) in &bookmark_keys {
            if ctx.input(|i| i.key_pressed(key) && i.modifiers.matches_exact(cmd_shift)) {
                self.bookmarks[slot] = Some(self.viewport.clone());
                self.status_message = Some((
                    format!("Bookmark {} saved", slot + 1),
                    std::time::Instant::now(),
                ));
            } else if !any_text_focused && self.selection.node_ids.is_empty() && ctx.input(|i| i.key_pressed(key) && i.modifiers.matches_exact(shift_only)) {
                if let Some(bv) = &self.bookmarks[slot].clone() {
                    self.zoom_target = bv.zoom;
                    self.pan_target = Some(bv.offset);
                    self.status_message = Some((
                        format!("Jumped to bookmark {}", slot + 1),
                        std::time::Instant::now(),
                    ));
                } else {
                    self.status_message = Some((
                        format!("No bookmark {} set — use ⌘⇧{} to save", slot + 1, slot + 1),
                        std::time::Instant::now(),
                    ));
                }
            }
        }

        // Cmd+Shift+L = toggle canvas lock (prevent node moves)
        let cmd_shift_l = Modifiers { shift: true, ..cmd };
        if ctx.input(|i| i.key_pressed(Key::L) && i.modifiers.matches_exact(cmd_shift_l)) {
            self.canvas_locked = !self.canvas_locked;
            let msg = if self.canvas_locked { "🔒 Canvas locked" } else { "🔓 Canvas unlocked" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // 3D camera preset shortcuts (only in 3D mode, no modifiers)
        if !any_text_focused && matches!(self.view_mode, super::ViewMode::ThreeD) {
            // 1=Iso, 2=Top, 3=Front, 4=Side
            let cam_presets: &[(Key, f32, f32, &str)] = &[
                (Key::Num1, -0.6,  0.5,  "Camera: Isometric"),
                (Key::Num2,  0.0,  1.55, "Camera: Top"),
                (Key::Num3,  0.0,  0.05, "Camera: Front"),
                (Key::Num4,  1.57, 0.05, "Camera: Side"),
            ];
            for &(key, yaw, pitch, msg) in cam_presets {
                if ctx.input(|i| i.key_pressed(key) && i.modifiers.is_none()) {
                    self.camera3d.yaw = yaw;
                    self.camera3d.pitch = pitch;
                    self.status_message = Some((msg.to_string(), std::time::Instant::now()));
                }
            }
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

        // Cmd+R = auto-layout / Rearrange (hierarchical, animated)
        if ctx.input(|i| i.key_pressed(Key::R) && i.modifiers.matches_exact(cmd)) {
            // Compute hierarchical layout on a document clone, then animate toward results
            let mut doc_clone = self.document.clone();
            for node in doc_clone.nodes.iter_mut() {
                if !node.pinned { node.position = [0.0, 0.0]; }
            }
            crate::specgraph::layout::auto_layout(&mut doc_clone);
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
                let current_id = self.selection.node_ids.iter().next().copied();
                let current_section = current_id
                    .and_then(|id| self.document.find_node(&id))
                    .map(|n| n.section_name.clone())
                    .unwrap_or_default();

                // Build spatially sorted candidate list (same section if in a section, otherwise all)
                let mut candidates: Vec<(crate::model::NodeId, [f32; 2])> = self.document.nodes.iter()
                    .filter(|node| {
                        if !current_section.is_empty() {
                            node.section_name == current_section
                        } else {
                            true // all nodes when no section
                        }
                    })
                    .map(|node| {
                        let c = node.rect().center();
                        (node.id, [c.y, c.x]) // sort: top-to-bottom, then left-to-right
                    })
                    .collect();
                candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                if !candidates.is_empty() {
                    let nc = candidates.len();
                    let current_pos = current_id
                        .and_then(|id| candidates.iter().position(|(cid, _)| *cid == id));
                    let next_idx = match current_pos {
                        None => 0,
                        Some(i) if shift => (i + nc - 1) % nc,
                        Some(i) => (i + 1) % nc,
                    };
                    let next_id = candidates[next_idx].0;
                    self.selection.select_node(next_id);
                    // Smoothly pan to the selected node
                    if let Some(node) = self.document.find_node(&next_id) {
                        let node_center = node.rect().center();
                        let c = self.canvas_rect.center();
                        self.pan_target = Some([
                            c.x - node_center.x * self.viewport.zoom,
                            c.y - node_center.y * self.viewport.zoom,
                        ]);
                    }
                }
            }
        }

        // P = cycle background pattern
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::P) && i.modifiers.is_none()) {
            if !self.selection.node_ids.is_empty() {
                // P + nodes selected → cycle support priority: None→P1→P2→P3→P4→None
                let node_ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
                // Determine next priority from first selected node
                let first = self.document.find_node(node_ids.first().unwrap());
                let current_prio = first.map(|n| n.priority).unwrap_or(0);
                let current_tag = first.and_then(|n| n.tag);
                let (new_tag, new_prio, new_fill, label) = match (current_prio, current_tag) {
                    (0, None)   | (0, Some(_)) => (Some(crate::model::NodeTag::Critical), 1u8, Some([243u8, 139, 168, 255]), "P1 — Critical"),
                    (1, _) | (0, Some(crate::model::NodeTag::Critical)) => (Some(crate::model::NodeTag::Warning), 2u8, Some([250u8, 179, 135, 255]), "P2 — High"),
                    (2, _) | (0, Some(crate::model::NodeTag::Warning))  => (Some(crate::model::NodeTag::Info),    3u8, Some([137u8, 180, 250, 255]), "P3 — Medium"),
                    (3, _) | (0, Some(crate::model::NodeTag::Info))     => (None,                                 4u8, Some([166u8, 227, 161, 255]), "P4 — Low"),
                    _                                                    => (None,                                 0u8, None,                         "No priority"),
                };
                for id in &node_ids {
                    if let Some(node) = self.document.find_node_mut(id) {
                        node.tag = new_tag;
                        node.priority = new_prio;
                        if let Some(fc) = new_fill {
                            node.style.fill_color = fc;
                        } else {
                            // Reset to default fill
                            node.style.fill_color = crate::model::NodeStyle::default().fill_color;
                        }
                    }
                }
                self.history.push(&self.document);
                self.status_message = Some((format!("Priority: {}", label), std::time::Instant::now()));
            } else {
                // No selection: cycle background pattern
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
        }

        // Shift+T = toggle dark/light mode (full theme switch)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::T) && i.modifiers.shift && !i.modifiers.command) {
            self.toggle_dark_mode(ctx);
        }

        // Cmd+Shift+W = toggle workload summary panel
        {
            let cmd_shift_w = Modifiers { shift: true, ..cmd };
            if !any_text_focused && ctx.input(|i| i.key_pressed(Key::W) && i.modifiers.matches_exact(cmd_shift_w)) {
                self.show_workload_panel = !self.show_workload_panel;
                let msg = if self.show_workload_panel { "Workload Panel On" } else { "Workload Panel Off" };
                self.status_message = Some((msg.to_string(), std::time::Instant::now()));
            }
        }

        // Shift+F = toggle focus mode (dim non-selected; W freed for design-thinking Evidence)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::F) && i.modifiers.shift && !i.modifiers.command && !i.modifiers.alt) {
            self.focus_mode = !self.focus_mode;
            let msg = if self.focus_mode { "Focus Mode On" } else { "Focus Mode Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // F / F5 = toggle presentation mode with slide navigation
        if !any_text_focused && ctx.input(|i| (i.key_pressed(Key::F) || i.key_pressed(Key::F5)) && i.modifiers.is_none()) {
            if self.presentation_mode {
                self.exit_presentation_mode();
            } else {
                self.enter_presentation_mode();
            }
        }

        // [ = toggle left toolbar   ] = toggle right properties panel
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::OpenBracket) && i.modifiers.is_none()) {
            self.toolbar_collapsed = !self.toolbar_collapsed;
        }
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::CloseBracket) && i.modifiers.is_none()) {
            self.properties_collapsed = !self.properties_collapsed;
        }

        // Shift+P = toggle quick-notes panel
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::P) && i.modifiers.shift && !i.modifiers.command) {
            self.show_quick_notes = !self.show_quick_notes;
            let msg = if self.show_quick_notes { "Quick Notes On" } else { "Quick Notes Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // K = toggle connectivity heatmap (H is reserved for design-thinking Hypothesis)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::K) && i.modifiers.is_none()) {
            self.show_heatmap = !self.show_heatmap;
            let msg = if self.show_heatmap { "Heatmap On" } else { "Heatmap Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // M (no modifier) = toggle minimap overlay
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::M) && i.modifiers.is_none()) {
            self.show_minimap = !self.show_minimap;
            let msg = if self.show_minimap { "Minimap On" } else { "Minimap Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // Shift+H = distribute selected nodes horizontally (equal spacing on X)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::H) && i.modifiers.shift_only())
            && self.selection.node_ids.len() >= 3
        {
            self.distribute_nodes_h();
            self.history.push(&self.document);
            self.status_message = Some(("Distributed H".to_string(), std::time::Instant::now()));
        }

        // Shift+V = distribute selected nodes vertically (equal spacing on Y)
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::V) && i.modifiers.shift_only())
            && self.selection.node_ids.len() >= 3
        {
            self.distribute_nodes_v();
            self.history.push(&self.document);
            self.status_message = Some(("Distributed V".to_string(), std::time::Instant::now()));
        }

        // Shift+A = toggle data-flow animation
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::A) && i.modifiers.shift_only()) {
            self.show_flow_animation = !self.show_flow_animation;
            let msg = if self.show_flow_animation { "Flow Animation On" } else { "Flow Animation Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // Shift+G = go to canvas position overlay
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::G) && i.modifiers.shift && !i.modifiers.command) {
            self.show_goto = !self.show_goto;
            if self.show_goto { self.goto_query.clear(); }
        }

        // G = toggle grid
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::G) && i.modifiers.is_none()) {
            self.show_grid = !self.show_grid;
            let msg = if self.show_grid { "Grid On" } else { "Grid Off" };
            self.status_message = Some((msg.to_string(), std::time::Instant::now()));
        }

        // Cmd+G = wrap selected nodes in a frame
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::G) && i.modifiers.matches_exact(cmd))
            && self.selection.node_ids.len() >= 2
        {
            let mut bb_min = egui::pos2(f32::MAX, f32::MAX);
            let mut bb_max = egui::pos2(f32::MIN, f32::MIN);
            for id in &self.selection.node_ids {
                if let Some(n) = self.document.find_node(id) {
                    let r = n.rect();
                    bb_min.x = bb_min.x.min(r.min.x);
                    bb_min.y = bb_min.y.min(r.min.y);
                    bb_max.x = bb_max.x.max(r.max.x);
                    bb_max.y = bb_max.y.max(r.max.y);
                }
            }
            if bb_min.x < f32::MAX {
                let pad = 20.0_f32;
                let frame_pos = egui::pos2(bb_min.x - pad, bb_min.y - pad);
                let mut frame = crate::model::Node::new_frame(frame_pos);
                frame.size = [bb_max.x - bb_min.x + pad * 2.0, bb_max.y - bb_min.y + pad * 2.0];
                let fid = frame.id;
                // Insert frame at beginning (so it's behind all nodes)
                self.document.nodes.insert(0, frame);
                self.selection.select_node(fid);
                self.history.push(&self.document);
                self.status_message = Some(("Group frame created".to_string(), std::time::Instant::now()));
            }
        }

        // S = toggle snap to grid
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.is_none()) {
            if !self.selection.edge_ids.is_empty() {
                // Cycle edge style: solid → dashed → animated → thick → back to solid
                let edge_ids: Vec<EdgeId> = self.selection.edge_ids.iter().copied().collect();
                let mut style_label = "Solid";
                for id in &edge_ids {
                    if let Some(edge) = self.document.find_edge_mut(id) {
                        if !edge.style.dashed && !edge.style.animated && edge.style.width <= 2.5 {
                            // solid → dashed
                            edge.style.dashed = true;
                            edge.style.animated = false;
                            style_label = "Dashed";
                        } else if edge.style.dashed && !edge.style.animated {
                            // dashed → animated
                            edge.style.dashed = false;
                            edge.style.animated = true;
                            style_label = "Animated";
                        } else if edge.style.animated {
                            // animated → thick
                            edge.style.animated = false;
                            edge.style.dashed = false;
                            edge.style.width = 3.5;
                            style_label = "Thick";
                        } else {
                            // thick → solid default
                            edge.style.dashed = false;
                            edge.style.animated = false;
                            edge.style.width = 1.5;
                            style_label = "Solid";
                        }
                    }
                }
                self.status_message = Some((format!("Edge style: {}", style_label), std::time::Instant::now()));
                self.history.push(&self.document);
            } else if !self.selection.node_ids.is_empty() {
                // S + nodes selected → cycle node status: None→Todo→WIP→Review→Done→Blocked→None
                let node_ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
                let mut label = "None";
                for id in &node_ids {
                    if let Some(node) = self.document.find_node_mut(id) {
                        let (new_tag, new_progress, lbl) = match node.tag {
                            None => (Some(crate::model::NodeTag::Warning), 0.0, "Todo"),
                            Some(crate::model::NodeTag::Warning) if node.progress < 0.5 => {
                                (Some(crate::model::NodeTag::Info), 0.5, "WIP")
                            }
                            Some(crate::model::NodeTag::Info) => {
                                (Some(crate::model::NodeTag::Warning), 0.75, "Review")
                            }
                            Some(crate::model::NodeTag::Warning) => {
                                (Some(crate::model::NodeTag::Ok), 1.0, "Done")
                            }
                            Some(crate::model::NodeTag::Ok) => {
                                (Some(crate::model::NodeTag::Critical), 0.0, "Blocked")
                            }
                            Some(crate::model::NodeTag::Critical) => (None, 0.0, "None"),
                        };
                        node.tag = new_tag;
                        node.progress = new_progress;
                        label = lbl;
                    }
                }
                // Check if all nodes in the affected section are now Done → celebrate
                let section_complete_msg: Option<String> = {
                    let affected_sections: std::collections::HashSet<String> = node_ids.iter()
                        .filter_map(|id| self.document.find_node(id))
                        .filter(|n| !n.section_name.is_empty())
                        .map(|n| n.section_name.clone())
                        .collect();
                    let mut msg = None;
                    for sec in &affected_sections {
                        let nodes_in_sec: Vec<_> = self.document.nodes.iter()
                            .filter(|n| &n.section_name == sec)
                            .collect();
                        let all_done = !nodes_in_sec.is_empty()
                            && nodes_in_sec.iter().all(|n| matches!(n.tag, Some(crate::model::NodeTag::Ok)));
                        if all_done {
                            msg = Some(format!("🎉 All done in \"{sec}\"!"));
                            break;
                        }
                    }
                    msg
                };
                let status_text = if let Some(cel) = section_complete_msg {
                    cel
                } else {
                    format!("Status: {label}")
                };
                self.status_message = Some((status_text, std::time::Instant::now()));
                self.history.push(&self.document);
            } else {
                self.snap_to_grid = !self.snap_to_grid;
                let msg = if self.snap_to_grid { "Snap On" } else { "Snap Off" };
                self.status_message = Some((msg.to_string(), std::time::Instant::now()));
            }
        }

        // Alt+1..9 = apply color preset to selected nodes
        // Palette: blue, green, red, yellow, purple, teal, orange, pink, white
        if !any_text_focused && !self.selection.node_ids.is_empty() {
            let alt_only = egui::Modifiers { alt: true, ..egui::Modifiers::NONE };
            let color_keys: [(Key, [u8; 4], &str); 9] = [
                (Key::Num1, [137, 180, 250, 255], "Blue"),      // hypothesis/info
                (Key::Num2, [166, 227, 161, 255], "Green"),     // evidence/done
                (Key::Num3, [243, 139, 168, 255], "Red"),       // blocked/risk
                (Key::Num4, [249, 226, 175, 255], "Yellow"),    // assumption/todo
                (Key::Num5, [203, 166, 247, 255], "Purple"),    // conclusion
                (Key::Num6, [148, 226, 213, 255], "Teal"),      // context
                (Key::Num7, [250, 179, 135, 255], "Orange"),    // hypothesis alt
                (Key::Num8, [245, 194, 231, 255], "Pink"),      // observation
                (Key::Num9, [230, 230, 240, 255], "White"),     // default/clear
            ];
            for (key, color, name) in &color_keys {
                if ctx.input(|i| i.key_pressed(*key) && i.modifiers.matches_exact(alt_only)) {
                    let node_ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
                    for id in &node_ids {
                        if let Some(node) = self.document.find_node_mut(id) {
                            node.style.fill_color = *color;
                            node.style.text_color = crate::app::theme::auto_contrast_text(*color);
                        }
                    }
                    self.history.push(&self.document);
                    self.status_message = Some((format!("Color: {name}"), std::time::Instant::now()));
                    break;
                }
            }
        }

        // Shift+1..5 = directly set status on all selected nodes (faster than cycling with S)
        // 1=Todo  2=WIP  3=Review  4=Done  5=Blocked  0=Clear
        if !any_text_focused && !self.selection.node_ids.is_empty() {
            let shift_only = egui::Modifiers { shift: true, ..egui::Modifiers::NONE };
            let status_keys: [(Key, Option<crate::model::NodeTag>, f32, &str); 6] = [
                (Key::Num0, None,                                    0.0,  "Cleared"),
                (Key::Num1, Some(crate::model::NodeTag::Warning),    0.0,  "Todo"),
                (Key::Num2, Some(crate::model::NodeTag::Info),       0.5,  "WIP"),
                (Key::Num3, Some(crate::model::NodeTag::Warning),    0.75, "Review"),
                (Key::Num4, Some(crate::model::NodeTag::Ok),         1.0,  "Done"),
                (Key::Num5, Some(crate::model::NodeTag::Critical),   0.0,  "Blocked"),
            ];
            for (key, new_tag, new_progress, status_name) in &status_keys {
                if ctx.input(|i| i.key_pressed(*key) && i.modifiers.matches_exact(shift_only)) {
                    let node_ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
                    let count = node_ids.len();
                    for id in &node_ids {
                        if let Some(node) = self.document.find_node_mut(id) {
                            node.tag = *new_tag;
                            node.progress = *new_progress;
                        }
                    }
                    self.history.push(&self.document);
                    let msg = if count == 1 {
                        format!("Status: {status_name}")
                    } else {
                        format!("Status: {status_name} ({count} nodes)")
                    };
                    self.status_message = Some((msg, std::time::Instant::now()));
                    break;
                }
            }
        }

        // Shift+D = set due date to TODAY for all selected nodes (preserves assignee)
        // Shift+W = set due date to one week from today for all selected nodes
        if !any_text_focused && !self.selection.node_ids.is_empty() {
            let shift_only = egui::Modifiers { shift: true, ..egui::Modifiers::NONE };
            let shift_d = ctx.input(|i| i.key_pressed(Key::D) && i.modifiers.matches_exact(shift_only));
            let shift_w = ctx.input(|i| i.key_pressed(Key::W) && i.modifiers.matches_exact(shift_only));
            if shift_d || shift_w {
                let today = super::render::today_iso();
                // Compute target date: today or +7 days
                let target_date = if shift_d {
                    today.clone()
                } else {
                    // Add 7 days to today using the same UNIX epoch trick
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let secs = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;
                    let days7 = ((secs / 86400) + 7) as i32;
                    // civil_from_days for days7
                    let z = days7 + 719468;
                    let era = if z >= 0 { z } else { z - 146096 } / 146097;
                    let doe = (z - era * 146097) as u32;
                    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
                    let y = yoe as i32 + era * 400;
                    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
                    let mp = (5 * doy + 2) / 153;
                    let d = doy - (153 * mp + 2) / 5 + 1;
                    let m = if mp < 10 { mp + 3 } else { mp - 9 };
                    let y = y + if m <= 2 { 1 } else { 0 };
                    format!("{:04}-{:02}-{:02}", y, m, d)
                };
                let due_part = format!("📅 {}", target_date);
                let node_ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
                let count = node_ids.len();
                for id in &node_ids {
                    if let Some(node) = self.document.find_node_mut(id) {
                        // Compose: preserve existing 👤 assignee line, replace/add 📅 line
                        let new_sublabel = {
                            let lines: Vec<&str> = node.sublabel.lines().collect();
                            let assignee_line = lines.iter().find(|l| l.starts_with("👤")).copied();
                            match assignee_line {
                                Some(a) => format!("{}\n{}", a, due_part),
                                None => due_part.clone(),
                            }
                        };
                        node.sublabel = new_sublabel;
                    }
                }
                self.history.push(&self.document);
                let label = if shift_d { "today" } else { "next week" };
                self.status_message = Some((
                    if count == 1 { format!("Due: {}", label) }
                    else { format!("Due: {} ({} nodes)", label, count) },
                    std::time::Instant::now(),
                ));
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
                                self.pan_target = Some([
                                    c.x - np.x * self.viewport.zoom,
                                    c.y - np.y * self.viewport.zoom,
                                ]);
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

        // Presentation mode: arrow keys + Space navigate slides, ESC exits
        if self.presentation_mode {
            if !any_text_focused && ctx.input(|i| i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::Space)) {
                self.presentation_next_slide();
            }
            if !any_text_focused && ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
                self.presentation_prev_slide();
            }
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.exit_presentation_mode();
            }
            return;
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

        // ] / [ = promote / demote selected nodes to next / previous kanban section
        if !any_text_focused && !self.selection.node_ids.is_empty() {
            let fwd = ctx.input(|i| i.key_pressed(Key::CloseBracket) && i.modifiers.is_none());
            let bwd = ctx.input(|i| i.key_pressed(Key::OpenBracket)  && i.modifiers.is_none());
            if fwd || bwd {
                // Build ordered section list by first occurrence in document
                let mut seen_sections: Vec<String> = Vec::new();
                for n in &self.document.nodes {
                    if !n.section_name.is_empty() && !seen_sections.contains(&n.section_name) {
                        seen_sections.push(n.section_name.clone());
                    }
                }
                if seen_sections.len() >= 2 {
                    // Find the most-common section among selected nodes
                    let sel_ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
                    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
                    for id in &sel_ids {
                        if let Some(n) = self.document.find_node(id) {
                            if !n.section_name.is_empty() {
                                *counts.entry(n.section_name.clone()).or_default() += 1;
                            }
                        }
                    }
                    // Pick the dominant section (most nodes belong to), fallback to first in order
                    let current = seen_sections.iter()
                        .max_by_key(|s| counts.get(*s).copied().unwrap_or(0))
                        .cloned()
                        .unwrap_or_else(|| seen_sections[0].clone());
                    let idx = seen_sections.iter().position(|s| s == &current).unwrap_or(0);
                    let new_idx = if fwd {
                        (idx + 1).min(seen_sections.len() - 1)
                    } else {
                        idx.saturating_sub(1)
                    };
                    if new_idx != idx {
                        let new_section = seen_sections[new_idx].clone();
                        for id in &sel_ids {
                            if let Some(n) = self.document.find_node_mut(id) {
                                n.section_name = new_section.clone();
                            }
                        }
                        self.history.push(&self.document);
                        let count = sel_ids.len();
                        self.status_message = Some((
                            if count == 1 { format!("→ {}", new_section) }
                            else { format!("→ {} ({} nodes)", new_section, count) },
                            std::time::Instant::now(),
                        ));
                        // Trigger animated re-layout (same as Cmd+L)
                        let mut doc_clone = self.document.clone();
                        for node in doc_clone.nodes.iter_mut() {
                            if !node.pinned { node.position = [0.0, 0.0]; }
                        }
                        crate::specgraph::layout::auto_layout(&mut doc_clone);
                        self.layout_targets.clear();
                        for node in &doc_clone.nodes {
                            self.layout_targets.insert(node.id, node.position);
                        }
                        self.pending_fit = true;
                    }
                }
            }
        }

        // C (no modifier) + nodes selected = open quick-comment popup
        if !any_text_focused && !self.selection.node_ids.is_empty()
            && ctx.input(|i| i.key_pressed(Key::C) && i.modifiers.is_none())
        {
            let prefill = self.selection.node_ids.iter().next()
                .and_then(|id| self.document.find_node(id))
                .map(|n| n.comment.clone())
                .unwrap_or_default();
            self.quick_comment_buf = Some(prefill);
        }

        // A (no modifier) + nodes selected = open quick-assign popup
        if !any_text_focused && !self.selection.node_ids.is_empty()
            && ctx.input(|i| i.key_pressed(Key::A) && i.modifiers.is_none())
        {
            // Pre-fill with the first selected node's current assignee (if any)
            let prefill = self.selection.node_ids.iter().next()
                .and_then(|id| self.document.find_node(id))
                .and_then(|n| n.sublabel.lines().find(|l| l.starts_with("👤 ")).map(|l| l.trim_start_matches("👤 ").to_string()))
                .unwrap_or_default();
            self.quick_assign_buf = Some(prefill);
        }

        // Z (no modifier) + nodes selected = snooze: push due date +1 day
        if !any_text_focused && !self.selection.node_ids.is_empty()
            && ctx.input(|i| i.key_pressed(Key::Z) && i.modifiers.is_none())
        {
            let add_days_str = |date_iso: &str, offset: i64| -> Option<String> {
                let parts: Vec<i64> = date_iso.splitn(3, '-').filter_map(|p| p.parse().ok()).collect();
                if parts.len() < 3 { return None; }
                let (y, m, d) = (parts[0], parts[1], parts[2]);
                // Convert to epoch days (days since 1970-01-01)
                let days_in_month = [0i64, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
                let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
                let mut day_n = (y - 1970) * 365 + (y - 1969) / 4 - (y - 1901) / 100 + (y - 1601) / 400;
                for mo in 1..m {
                    day_n += days_in_month[mo as usize];
                    if mo == 2 && leap { day_n += 1; }
                }
                day_n += d - 1 + offset;
                // Convert back
                let z = day_n as i32 + 719468;
                let era = if z >= 0 { z } else { z - 146096 } / 146097;
                let doe = (z - era * 146097) as u32;
                let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
                let yr = yoe as i32 + era * 400;
                let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
                let mp = (5 * doy + 2) / 153;
                let dd = doy - (153 * mp + 2) / 5 + 1;
                let mo = if mp < 10 { mp + 3 } else { mp - 9 };
                let yr = if mo <= 2 { yr + 1 } else { yr };
                Some(format!("{:04}-{:02}-{:02}", yr, mo, dd))
            };
            let ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
            let count = ids.len();
            let mut snoozed = 0u32;
            for id in &ids {
                if let Some(n) = self.document.find_node_mut(id) {
                    let due_line = n.sublabel.lines().find(|l| l.starts_with("📅 ")).map(|l| l.to_string());
                    if let Some(ref dl) = due_line {
                        let date_str = dl.trim_start_matches("📅 ").trim();
                        if let Some(new_date) = add_days_str(date_str, 1) {
                            let other: Vec<String> = n.sublabel.lines()
                                .filter(|l| !l.starts_with("📅 ")).map(|l| l.to_string()).collect();
                            let mut parts: Vec<String> = n.sublabel.lines()
                                .filter(|l| l.starts_with("👤 ")).map(|l| l.to_string()).collect();
                            parts.push(format!("📅 {}", new_date));
                            parts.extend(other.into_iter().filter(|l| !l.starts_with("👤 ")));
                            parts.retain(|l| !l.is_empty());
                            n.sublabel = parts.join("\n");
                            snoozed += 1;
                        }
                    }
                }
            }
            if snoozed > 0 {
                self.history.push(&self.document);
                self.status_message = Some((
                    if snoozed == 1 { "💤 Snoozed +1 day".to_string() }
                    else { format!("💤 Snoozed {} tickets +1 day", snoozed) },
                    std::time::Instant::now(),
                ));
            }
        }

        // Shift+E = Escalate: set Critical priority on selected tickets, move to Triage (section 2)
        if !any_text_focused && !self.selection.node_ids.is_empty()
            && ctx.input(|i| i.key_pressed(Key::E) && i.modifiers.matches_exact(egui::Modifiers { shift: true, ..egui::Modifiers::NONE }))
        {
            // Find the 2nd section (Triage) in doc order
            let mut seen_sections: Vec<String> = Vec::new();
            for n in &self.document.nodes {
                if !n.section_name.is_empty() && !seen_sections.contains(&n.section_name) {
                    seen_sections.push(n.section_name.clone());
                }
            }
            let triage_sec: Option<String> = seen_sections.get(1).cloned();
            let ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
            let count = ids.len();
            for id in &ids {
                if let Some(n) = self.document.find_node_mut(id) {
                    n.tag = Some(crate::model::NodeTag::Critical);
                    n.priority = 1;
                    n.style.fill_color = [243, 139, 168, 255];
                    if let Some(ref sec) = triage_sec {
                        n.section_name = sec.clone();
                    }
                }
            }
            self.history.push(&self.document);
            // Animated re-layout
            let mut doc_clone = self.document.clone();
            for n in doc_clone.nodes.iter_mut() { if !n.pinned { n.position = [0.0, 0.0]; } }
            crate::specgraph::layout::auto_layout(&mut doc_clone);
            self.layout_targets.clear();
            for n in &doc_clone.nodes { self.layout_targets.insert(n.id, n.position); }
            let msg = if let Some(ref sec) = triage_sec {
                if count == 1 { format!("🚨 Escalated → {}", sec) }
                else { format!("🚨 Escalated {} tickets → {}", count, sec) }
            } else {
                "🚨 Escalated: P1 Critical".to_string()
            };
            self.status_message = Some((msg, std::time::Instant::now()));
        }

        // Cmd+Enter = mark all selected nodes as Done (tag=Ok, progress=1.0)
        // Great for "close ticket" in a support kanban workflow
        if !any_text_focused && !self.selection.node_ids.is_empty()
            && ctx.input(|i| i.key_pressed(Key::Enter) && i.modifiers.matches_exact(cmd))
        {
            let ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
            let count = ids.len();
            for id in &ids {
                if let Some(n) = self.document.find_node_mut(id) {
                    n.tag = Some(crate::model::NodeTag::Ok);
                    n.progress = 1.0;
                }
            }
            self.history.push(&self.document);
            self.status_message = Some((
                if count == 1 { "✓ Done".to_string() }
                else { format!("✓ Done ({} tickets)", count) },
                std::time::Instant::now(),
            ));
        }

        // Ctrl+1..4 = jump selected tickets to section N (direct column assignment in kanban)
        // Uses actual Control key (not Cmd) so it's free on Mac
        if !any_text_focused && !self.selection.node_ids.is_empty() {
            let ctrl_only = egui::Modifiers { ctrl: true, ..egui::Modifiers::NONE };
            let sec_keys = [Key::Num1, Key::Num2, Key::Num3, Key::Num4];
            let pressed_idx = sec_keys.iter().position(|&k| {
                ctx.input(|i| i.key_pressed(k) && i.modifiers.matches_exact(ctrl_only))
            });
            if let Some(idx) = pressed_idx {
                // Build section list in first-occurrence order
                let mut seen: Vec<String> = Vec::new();
                for n in &self.document.nodes {
                    if !n.section_name.is_empty() && !seen.contains(&n.section_name) {
                        seen.push(n.section_name.clone());
                    }
                }
                if let Some(target_sec) = seen.get(idx) {
                    let target_sec = target_sec.clone();
                    let sel_ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
                    let count = sel_ids.len();
                    for id in &sel_ids {
                        if let Some(n) = self.document.find_node_mut(id) {
                            n.section_name = target_sec.clone();
                        }
                    }
                    self.history.push(&self.document);
                    // Trigger animated re-layout
                    let mut doc_clone = self.document.clone();
                    for n in doc_clone.nodes.iter_mut() { if !n.pinned { n.position = [0.0, 0.0]; } }
                    crate::specgraph::layout::auto_layout(&mut doc_clone);
                    self.layout_targets.clear();
                    for n in &doc_clone.nodes { self.layout_targets.insert(n.id, n.position); }
                    self.status_message = Some((
                        if count == 1 { format!("→ {}", target_sec) }
                        else { format!("→ {} ({} tickets)", target_sec, count) },
                        std::time::Instant::now(),
                    ));
                }
            }
        }

        // Alt+P = sort nodes within each section by priority (P1→P2→P3→P4→Done)
        {
            let alt_only = egui::Modifiers { alt: true, ..egui::Modifiers::NONE };
            if !any_text_focused && ctx.input(|i| i.key_pressed(Key::P) && i.modifiers.matches_exact(alt_only)) {
                // Priority rank: Critical=0, Warning=1, Info=2, None=3, Ok=4 (done last)
                let priority_rank = |n: &crate::model::Node| -> i32 {
                    if n.progress >= 1.0 { return 100; }
                    match n.tag {
                        Some(crate::model::NodeTag::Critical) => 0,
                        Some(crate::model::NodeTag::Warning)  => 1,
                        Some(crate::model::NodeTag::Info)     => 2,
                        None                                  => 3,
                        Some(crate::model::NodeTag::Ok)       => 4,
                    }
                };
                // Group nodes by section, sort each group by priority, reorder Y positions
                let mut sections: std::collections::HashMap<String, Vec<(crate::model::NodeId, i32)>> = std::collections::HashMap::new();
                for n in &self.document.nodes {
                    if n.section_name.is_empty() || n.is_frame || n.pinned { continue; }
                    let rank = priority_rank(n);
                    sections.entry(n.section_name.clone()).or_default().push((n.id, rank));
                }
                let mut changed = false;
                for (_, group) in &sections {
                    // Get current Y positions
                    let mut positioned: Vec<(crate::model::NodeId, i32, f32)> = group.iter().filter_map(|&(id, rank)| {
                        self.document.find_node(&id).map(|n| (id, rank, n.position[1]))
                    }).collect();
                    if positioned.len() < 2 { continue; }
                    let ys: Vec<f32> = { let mut v: Vec<f32> = positioned.iter().map(|t| t.2).collect(); v.sort_by(|a,b| a.partial_cmp(b).unwrap()); v };
                    // Sort by priority rank
                    positioned.sort_by_key(|t| t.1);
                    // Assign sorted positions
                    for (i, &(id, _, _)) in positioned.iter().enumerate() {
                        if let Some(n) = self.document.find_node_mut(&id) {
                            n.position[1] = ys[i];
                            changed = true;
                        }
                    }
                }
                if changed {
                    self.history.push(&self.document);
                    self.status_message = Some(("⬆ Sorted by priority".to_string(), std::time::Instant::now()));
                }
            }
        }

        // Alt+D = sort nodes within each section by due date (soonest first, no-due-date last)
        {
            let alt_only = egui::Modifiers { alt: true, ..egui::Modifiers::NONE };
            if !any_text_focused && ctx.input(|i| i.key_pressed(Key::D) && i.modifiers.matches_exact(alt_only)) {
                let today = super::render::today_iso();
                // due_rank: days remaining from today; None/missing = very large (sort last)
                let due_rank = |n: &crate::model::Node| -> i32 {
                    n.sublabel.split('\n').find_map(|l| l.strip_prefix("📅 "))
                        .map(|d| super::render::iso_days_remaining_pub(d.trim(), &today))
                        .unwrap_or(i32::MAX)
                };
                let mut sections: std::collections::HashMap<String, Vec<(crate::model::NodeId, i32)>> = std::collections::HashMap::new();
                for n in &self.document.nodes {
                    if n.section_name.is_empty() || n.is_frame || n.pinned { continue; }
                    sections.entry(n.section_name.clone()).or_default().push((n.id, due_rank(n)));
                }
                let mut changed = false;
                for (_, group) in &sections {
                    let mut positioned: Vec<(crate::model::NodeId, i32, f32)> = group.iter().filter_map(|&(id, rank)| {
                        self.document.find_node(&id).map(|n| (id, rank, n.position[1]))
                    }).collect();
                    if positioned.len() < 2 { continue; }
                    let ys: Vec<f32> = { let mut v: Vec<f32> = positioned.iter().map(|t| t.2).collect(); v.sort_by(|a,b| a.partial_cmp(b).unwrap()); v };
                    positioned.sort_by_key(|t| t.1);
                    for (i, &(id, _, _)) in positioned.iter().enumerate() {
                        if let Some(n) = self.document.find_node_mut(&id) {
                            n.position[1] = ys[i];
                            changed = true;
                        }
                    }
                }
                if changed {
                    self.history.push(&self.document);
                    self.status_message = Some(("📅 Sorted by due date".to_string(), std::time::Instant::now()));
                }
            }
        }

        // Enter = chain: create a new node connected in the layout direction
        // Shift+Enter = chain in the orthogonal direction
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::Enter)) {
            let shift = ctx.input(|i| i.modifiers.shift);
            // Default direction follows the document's layout_dir (TB/BT = downward; LR/RL = rightward)
            let is_tb = matches!(self.document.layout_dir.as_str(), "TB" | "BT");
            let downward = if shift { !is_tb } else { is_tb };
            self.chain_create_node(downward);
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
        // Inherit parent's style and section
        if let Some(parent) = self.document.find_node(&sel_id) {
            new_node.style = parent.style.clone();
            new_node.section_name = parent.section_name.clone();
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

    /// Zoom the 3D camera to fit all nodes (adjusts distance only, resets to iso view).
    pub(crate) fn fit_3d_to_content(&mut self, ctx: &egui::Context) {
        if self.document.nodes.is_empty() { return; }
        // Find bounding box of all nodes in world XY
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for node in &self.document.nodes {
            min_x = min_x.min(node.position[0]);
            min_y = min_y.min(node.position[1]);
            max_x = max_x.max(node.position[0] + node.size[0]);
            max_y = max_y.max(node.position[1] + node.size[1]);
        }
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        let span = ((max_x - min_x).max(max_y - min_y)) * 1.4;
        self.camera3d.target = [cx, cy, 0.0];
        let now = ctx.input(|i| i.time);
        self.camera3d.animate_to(-0.4, 0.6, now, 0.4);
        self.camera3d.distance = span.max(400.0).min(8000.0);
        self.cam3d_zoom_vel = 0.0; // cancel any in-progress zoom inertia
        ctx.request_repaint();
        self.status_message = Some(("3D Fit".to_string(), std::time::Instant::now()));
    }
}
