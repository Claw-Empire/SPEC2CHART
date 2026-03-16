//! Right-click context menus for nodes, edges, multi-selection, and empty canvas.

use egui::{Pos2, Vec2};
use crate::model::*;
use super::FlowchartApp;
use super::theme::{BULK_COLORS, NODE_COLORS, EDGE_COLORS, to_color32, auto_contrast_text};

impl FlowchartApp {
    /// Dispatch context menu based on what was right-clicked.
    pub(crate) fn draw_context_menu(&mut self, ui: &mut egui::Ui, pointer_pos: Option<Pos2>) {
        let Some(mouse) = pointer_pos else { return };
        let canvas_pos = self.viewport.screen_to_canvas(mouse);

        // Multi-selection: ≥ 2 nodes selected and clicking within any of them
        let multi_click = if self.selection.node_ids.len() > 1 {
            self.document.node_at_pos(canvas_pos)
                .map(|id| self.selection.node_ids.contains(&id))
                .unwrap_or(false)
        } else {
            false
        };

        if multi_click {
            self.context_menu_multi(ui);
        } else if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
            self.context_menu_node(ui, node_id);
        } else if let Some(edge_id) = self.hit_test_edge(canvas_pos) {
            self.context_menu_edge(ui, edge_id);
        } else {
            self.context_menu_canvas(ui, canvas_pos);
        }
    }

    // ── Multi-selection ──────────────────────────────────────────────────

    fn context_menu_multi(&mut self, ui: &mut egui::Ui) {
        let n_sel = self.selection.node_ids.len();
        ui.label(egui::RichText::new(format!("{} nodes", n_sel))
            .size(11.0).color(self.theme.text_dim).strong());
        ui.separator();

        // Bulk color row
        let mut bulk_color_pick: Option<[u8; 4]> = None;
        ui.horizontal_wrapped(|ui| {
            for (color, name) in BULK_COLORS {
                let c = to_color32(*color);
                if ui.add(egui::Button::new("  ").fill(c).min_size(egui::Vec2::new(22.0, 22.0)))
                    .on_hover_text(*name).clicked()
                {
                    bulk_color_pick = Some(*color);
                }
            }
        });
        if let Some(col) = bulk_color_pick {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            for id in ids {
                if let Some(n) = self.document.find_node_mut(&id) {
                    n.style.fill_color = col;
                }
            }
            self.history.push(&self.document);
            ui.close_menu();
        }

        // Bulk tag
        self.tag_submenu(ui, "🏷 Tag all…", None);

        // Bulk priority (support use case)
        ui.menu_button("🔢 Set Priority…", |ui| {
            let priorities: &[(&str, Option<NodeTag>, [u8; 4], &str)] = &[
                ("P1 — Critical", Some(NodeTag::Critical), [243, 139, 168, 255], "System down / blocking"),
                ("P2 — High",     Some(NodeTag::Warning),  [250, 179, 135, 255], "Major feature broken"),
                ("P3 — Medium",   Some(NodeTag::Info),     [137, 180, 250, 255], "Degraded experience"),
                ("P4 — Low",      None,                    [166, 227, 161, 255], "Cosmetic / question"),
            ];
            for (label, tag, fill, tip) in priorities {
                if ui.button(*label).on_hover_text(*tip).clicked() {
                    let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                    for id in &ids {
                        if let Some(n) = self.document.find_node_mut(id) {
                            n.tag = *tag;
                            n.style.fill_color = *fill;
                            n.style.text_color = auto_contrast_text(*fill);
                        }
                    }
                    self.history.push(&self.document);
                    ui.close_menu();
                }
            }
        });

        // Bulk highlight toggle
        {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            let all_highlighted = ids.iter().all(|id| {
                self.document.find_node(id).map_or(false, |n| n.highlight)
            });
            let hl_label = if all_highlighted { "⭐ Remove Highlight All" } else { "⭐ Highlight All" };
            if ui.button(hl_label).clicked() {
                let new_val = !all_highlighted;
                for id in &ids {
                    if let Some(node) = self.document.find_node_mut(id) {
                        node.highlight = new_val;
                    }
                }
                self.history.push(&self.document);
                ui.close_menu();
            }
        }

        ui.separator();

        // Align submenu
        ui.menu_button("⟺ Align…", |ui| {
            if ui.button("← Left edges").clicked()   { self.align_nodes_left();     ui.close_menu(); }
            if ui.button("→ Right edges").clicked()   { self.align_nodes_right();    ui.close_menu(); }
            if ui.button("↕ Center H").clicked()      { self.align_nodes_center_h(); ui.close_menu(); }
            if ui.button("↑ Top edges").clicked()      { self.align_nodes_top();      ui.close_menu(); }
            if ui.button("↓ Bottom edges").clicked()   { self.align_nodes_bottom();   ui.close_menu(); }
            if ui.button("⟺ Center V").clicked()      { self.align_nodes_center_v(); ui.close_menu(); }
            ui.separator();
            if ui.button("⟺ Distribute H").clicked() { self.distribute_nodes_h();   ui.close_menu(); }
            if ui.button("↕ Distribute V").clicked()  { self.distribute_nodes_v();   ui.close_menu(); }
        });

        if ui.button("⎘ Duplicate all").clicked() {
            let to_dup: Vec<_> = self.selection.node_ids.iter()
                .filter_map(|id| self.document.find_node(id).cloned())
                .collect();
            self.selection.clear();
            for mut node in to_dup {
                node.id = NodeId::new();
                node.set_pos(node.pos() + Vec2::new(24.0, 24.0));
                let new_id = node.id;
                self.document.nodes.push(node);
                self.selection.node_ids.insert(new_id);
            }
            self.history.push(&self.document);
            ui.close_menu();
        }

        ui.separator();

        if ui.button("🗑 Delete all").clicked() {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            for id in ids { self.document.remove_node(&id); }
            self.selection.clear();
            self.history.push(&self.document);
            ui.close_menu();
        }
    }

    // ── Single node ──────────────────────────────────────────────────────

    fn context_menu_node(&mut self, ui: &mut egui::Ui, node_id: NodeId) {
        self.selection.select_node(node_id);

        // Quick-color row
        ui.label(egui::RichText::new("Fill").size(9.5).color(self.theme.text_dim));
        let mut color_pick: Option<[u8; 4]> = None;
        ui.horizontal_wrapped(|ui| {
            for (color, name) in NODE_COLORS {
                let c = to_color32(*color);
                let is_current = self.document.find_node(&node_id)
                    .map(|n| n.style.fill_color == *color)
                    .unwrap_or(false);
                let btn = egui::Button::new(if is_current { "✓" } else { "  " })
                    .fill(c)
                    .min_size(egui::Vec2::new(22.0, 22.0));
                if ui.add(btn).on_hover_text(*name).clicked() {
                    color_pick = Some(*color);
                }
            }
        });
        if let Some(col) = color_pick {
            if let Some(n) = self.document.find_node_mut(&node_id) {
                n.style.fill_color = col;
                n.style.text_color = auto_contrast_text(col);
            }
            self.history.push(&self.document);
            ui.close_menu();
        }
        ui.separator();

        if ui.button("✏ Edit label").clicked() {
            self.focus_label_edit = true;
            ui.close_menu();
        }
        // Copy node label to clipboard
        if let Some(label) = self.document.find_node(&node_id).and_then(|n| match &n.kind {
            NodeKind::Shape { label, .. } => Some(label.clone()),
            NodeKind::StickyNote { text, .. } => Some(text.clone()),
            _ => None,
        }) {
            if ui.button("📋 Copy label").clicked() {
                ui.ctx().copy_text(label);
                self.status_message = Some(("Label copied to clipboard".to_string(), std::time::Instant::now()));
                ui.close_menu();
            }
        }
        // Spawn child node connected from this node
        if ui.button("→ Spawn child node").clicked() {
            let is_tb = matches!(self.document.layout_dir.as_str(), "TB" | "BT");
            if let Some(src) = self.document.find_node(&node_id).cloned() {
                let (dx, dy) = if is_tb { (0.0_f32, src.size[1] + 80.0) } else { (src.size[0] + 100.0, 0.0_f32) };
                let new_pos = egui::Pos2::new(src.position[0] + dx, src.position[1] + dy);
                let shape = match &src.kind { NodeKind::Shape { shape, .. } => *shape, _ => NodeShape::RoundedRect };
                let mut child = Node::new(shape, new_pos);
                child.size = src.size;
                child.style = src.style.clone();
                child.section_name = src.section_name.clone();
                child.z_offset = src.z_offset;
                child.tag = None; child.progress = 0.0;
                let child_id = child.id;
                self.document.nodes.push(child);
                let (ss, ts) = if is_tb { (PortSide::Bottom, PortSide::Top) } else { (PortSide::Right, PortSide::Left) };
                self.document.edges.push(Edge::new(Port { node_id, side: ss }, Port { node_id: child_id, side: ts }));
                self.selection.clear();
                self.selection.select_node(child_id);
                self.focus_label_edit = true;
                self.history.push(&self.document);
            }
            ui.close_menu();
        }
        if ui.button("⎘ Duplicate").clicked() {
            if let Some(node) = self.document.find_node(&node_id).cloned() {
                let mut copy = node;
                copy.id = NodeId::new();
                copy.set_pos(copy.pos() + Vec2::new(24.0, 24.0));
                let cid = copy.id;
                self.document.nodes.push(copy);
                self.selection.select_node(cid);
                self.history.push(&self.document);
            }
            ui.close_menu();
        }
        if ui.button("⬆ Bring to Front").clicked() {
            if let Some(i) = self.document.nodes.iter().position(|n| n.id == node_id) {
                let n = self.document.nodes.remove(i);
                self.document.nodes.push(n);
                self.history.push(&self.document);
            }
            ui.close_menu();
        }
        if ui.button("⬇ Send to Back").clicked() {
            if let Some(i) = self.document.nodes.iter().position(|n| n.id == node_id) {
                let n = self.document.nodes.remove(i);
                self.document.nodes.insert(0, n);
                self.history.push(&self.document);
            }
            ui.close_menu();
        }
        ui.separator();

        // Pin / Lock toggles
        {
            let is_pinned = self.document.find_node(&node_id).map(|n| n.pinned).unwrap_or(false);
            if ui.button(if is_pinned { "📌 Unpin" } else { "📌 Pin" }).clicked() {
                if let Some(n) = self.document.find_node_mut(&node_id) { n.pinned = !n.pinned; }
                self.history.push(&self.document);
                ui.close_menu();
            }
        }
        {
            let is_locked = self.document.find_node(&node_id).map(|n| n.locked).unwrap_or(false);
            if ui.button(if is_locked { "🔓 Unlock" } else { "🔒 Lock" }).clicked() {
                if let Some(n) = self.document.find_node_mut(&node_id) { n.locked = !n.locked; }
                self.history.push(&self.document);
                ui.close_menu();
            }
        }

        if ui.button("🔗 Select Connected").clicked() {
            let connected: Vec<NodeId> = self.document.edges.iter()
                .filter_map(|e| {
                    if e.source.node_id == node_id { Some(e.target.node_id) }
                    else if e.target.node_id == node_id { Some(e.source.node_id) }
                    else { None }
                })
                .collect();
            for nid in connected { self.selection.node_ids.insert(nid); }
            ui.close_menu();
        }

        self.tag_submenu(ui, "🏷 Tag…", Some(node_id));

        // Section assignment submenu
        ui.menu_button("📂 Move to Section…", |ui| {
            let current_section = self.document.find_node(&node_id)
                .map(|n| n.section_name.clone())
                .unwrap_or_default();

            // Collect unique sections currently used in the document (sorted, excluding blank)
            let mut doc_sections: Vec<String> = {
                let mut seen = std::collections::BTreeSet::new();
                for n in &self.document.nodes {
                    if !n.section_name.is_empty() { seen.insert(n.section_name.clone()); }
                }
                seen.into_iter().collect()
            };

            if !doc_sections.is_empty() {
                ui.label(egui::RichText::new("In this document").size(10.0).weak());
                for section in &doc_sections {
                    let is_current = current_section.as_str() == section.as_str();
                    let label = if is_current { format!("✓ {section}") } else { section.clone() };
                    if ui.button(label).clicked() {
                        if let Some(n) = self.document.find_node_mut(&node_id) {
                            n.section_name = if is_current { String::new() } else { section.clone() };
                        }
                        self.history.push(&self.document);
                        ui.close_menu();
                    }
                }
                ui.separator();
                ui.label(egui::RichText::new("Common sections").size(10.0).weak());
            }

            // Design thinking presets
            let presets_design = [
                "Hypotheses", "Assumptions", "Evidence", "Conclusions",
                "Questions", "Experiments", "Options", "Risks",
                "Roses", "Buds", "Thorns", "Actions",
                "Strengths", "Weaknesses", "Opportunities", "Threats",
            ];
            // Support ops presets
            let presets_support = [
                "Intake", "Triage", "In Progress", "Resolved",
                "Escalated", "Closed", "Backlog", "On Hold",
                "P1 — Critical", "P2 — High", "P3 — Medium", "P4 — Low",
            ];
            for presets_group in [&presets_design[..], &presets_support[..]] {
                for &section in presets_group {
                    if doc_sections.iter().any(|s| s.as_str() == section) { continue; } // skip dupes
                    let is_current = current_section.as_str() == section;
                    let label = if is_current { format!("✓ {section}") } else { section.to_string() };
                    if ui.button(label).clicked() {
                        if let Some(n) = self.document.find_node_mut(&node_id) {
                            n.section_name = if is_current { String::new() } else { section.to_string() };
                        }
                        self.history.push(&self.document);
                        ui.close_menu();
                    }
                }
            }
            ui.separator();
            if ui.button("✕ Clear section").clicked() {
                if let Some(n) = self.document.find_node_mut(&node_id) {
                    n.section_name.clear();
                }
                self.history.push(&self.document);
                ui.close_menu();
            }
        });

        // Assign to submenu
        ui.menu_button("👤 Assign to…", |ui| {
            // Collect assignees already used in this document
            let existing_assignees: Vec<String> = {
                let mut seen = std::collections::BTreeSet::new();
                for n in &self.document.nodes {
                    if let Some(name) = n.sublabel.strip_prefix("👤 ") {
                        seen.insert(name.trim().to_string());
                    }
                }
                seen.into_iter().collect()
            };
            if !existing_assignees.is_empty() {
                ui.label(egui::RichText::new("In this document").size(10.0).weak());
                for name in &existing_assignees {
                    if ui.button(format!("👤 {name}")).clicked() {
                        let ids: Vec<_> = if self.selection.node_ids.contains(&node_id) {
                            self.selection.node_ids.iter().copied().collect()
                        } else { vec![node_id] };
                        for id in &ids {
                            if let Some(n) = self.document.find_node_mut(id) {
                                n.sublabel = format!("👤 {}", name);
                            }
                        }
                        self.history.push(&self.document);
                        ui.close_menu();
                    }
                }
                ui.separator();
            }
            let common: &[&str] = &["Alice", "Bob", "Charlie", "Dana", "Erin", "Frank", "Team", "On-call"];
            for name in common {
                if existing_assignees.iter().any(|a| a.as_str() == *name) { continue; }
                if ui.button(format!("👤 {name}")).clicked() {
                    let ids: Vec<_> = if self.selection.node_ids.contains(&node_id) {
                        self.selection.node_ids.iter().copied().collect()
                    } else { vec![node_id] };
                    for id in &ids {
                        if let Some(n) = self.document.find_node_mut(id) {
                            n.sublabel = format!("👤 {}", name);
                        }
                    }
                    self.history.push(&self.document);
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui.button("✕ Clear assignee").clicked() {
                let ids: Vec<_> = if self.selection.node_ids.contains(&node_id) {
                    self.selection.node_ids.iter().copied().collect()
                } else { vec![node_id] };
                for id in &ids {
                    if let Some(n) = self.document.find_node_mut(id) {
                        if n.sublabel.starts_with("👤") { n.sublabel.clear(); }
                    }
                }
                self.history.push(&self.document);
                ui.close_menu();
            }
        });

        // Due date quick-set submenu
        ui.menu_button("📅 Set Due Date…", |ui| {
            let today_str = "2026-03-16";
            // Pre-compute common relative dates
            let date_opts: &[(&str, &str)] = &[
                ("Today (2026-03-16)",        "2026-03-16"),
                ("Tomorrow (2026-03-17)",     "2026-03-17"),
                ("This Friday (2026-03-20)",  "2026-03-20"),
                ("Next Monday (2026-03-23)",  "2026-03-23"),
                ("End of month (2026-03-31)", "2026-03-31"),
                ("End of Q2 (2026-06-30)",    "2026-06-30"),
            ];
            for (label, date) in date_opts {
                if ui.button(*label).clicked() {
                    let ids: Vec<_> = if self.selection.node_ids.contains(&node_id) {
                        self.selection.node_ids.iter().copied().collect()
                    } else { vec![node_id] };
                    for id in &ids {
                        if let Some(n) = self.document.find_node_mut(id) {
                            n.sublabel = format!("📅 {}", date);
                        }
                    }
                    self.history.push(&self.document);
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui.button("✕ Clear due date").clicked() {
                let ids: Vec<_> = if self.selection.node_ids.contains(&node_id) {
                    self.selection.node_ids.iter().copied().collect()
                } else { vec![node_id] };
                for id in &ids {
                    if let Some(n) = self.document.find_node_mut(id) {
                        if n.sublabel.starts_with("📅") { n.sublabel.clear(); }
                    }
                }
                self.history.push(&self.document);
                ui.close_menu();
            }
            let _ = today_str; // used for comment
        });

        // Design Thinking type picker
        ui.menu_button("💡 Design Type…", |ui| {
            // (shape, fill_color, label)
            let presets: &[(NodeShape, [u8; 4], &str)] = &[
                (NodeShape::Diamond,      [250, 179, 135, 255], "💡 Hypothesis"),
                (NodeShape::Parallelogram,[137, 180, 250, 255], "📐 Assumption"),
                (NodeShape::Rectangle,   [166, 227, 161, 255], "✅ Evidence"),
                (NodeShape::Hexagon,     [203, 166, 247, 255], "🏁 Conclusion"),
                (NodeShape::Circle,      [249, 226, 175, 255], "❓ Question"),
                (NodeShape::Hexagon,     [249, 226, 175, 255], "🧪 Experiment"),
                (NodeShape::Rectangle,   [148, 226, 213, 255], "📊 Metric"),
                (NodeShape::RoundedRect, [245, 194, 231, 255], "🤔 How-might-we"),
                (NodeShape::RoundedRect, [166, 227, 161, 255], "⭐ Strength"),
                (NodeShape::RoundedRect, [243, 139, 168, 255], "⚠️ Weakness"),
                (NodeShape::RoundedRect, [137, 180, 250, 255], "🌱 Opportunity"),
                (NodeShape::RoundedRect, [249, 226, 175, 255], "🚨 Threat"),
                (NodeShape::Callout,     [245, 194, 231, 255], "💬 Quote / Observation"),
                (NodeShape::Diamond,     [243, 139, 168, 255], "😣 Pain Point"),
                (NodeShape::RoundedRect, [166, 227, 161, 255], "🌟 Gain / Delight"),
            ];
            for (shape, fill, label) in presets {
                if ui.button(*label).clicked() {
                    if let Some(n) = self.document.find_node_mut(&node_id) {
                        if let NodeKind::Shape { shape: ref mut s, .. } = n.kind {
                            *s = *shape;
                        }
                        n.style.fill_color = *fill;
                        n.style.text_color = auto_contrast_text(*fill);
                    }
                    self.history.push(&self.document);
                    ui.close_menu();
                }
            }
        });

        // Highlight toggle
        if let Some(node) = self.document.find_node(&node_id) {
            let hl = node.highlight;
            let hl_label = if hl { "⭐ Remove Highlight" } else { "⭐ Highlight" };
            if ui.button(hl_label).clicked() {
                if let Some(node) = self.document.find_node_mut(&node_id) {
                    node.highlight = !hl;
                }
                self.history.push(&self.document);
                ui.close_menu();
            }
        }

        ui.separator();
        if ui.button("🗑 Delete").clicked() {
            self.document.remove_node(&node_id);
            self.selection.clear();
            self.history.push(&self.document);
            ui.close_menu();
        }
    }

    // ── Edge ─────────────────────────────────────────────────────────────

    fn context_menu_edge(&mut self, ui: &mut egui::Ui, edge_id: EdgeId) {
        self.selection.select_edge(edge_id);
        ui.label(egui::RichText::new("Edge").size(11.0).color(self.theme.text_dim));
        ui.separator();

        // Color presets
        ui.horizontal_wrapped(|ui| {
            for (color, name) in EDGE_COLORS {
                let c = to_color32(*color);
                if ui.add(egui::Button::new("  ").fill(c).min_size(egui::Vec2::new(22.0, 22.0)))
                    .on_hover_text(*name).clicked()
                {
                    if let Some(e) = self.document.find_edge_mut(&edge_id) { e.style.color = *color; }
                    self.history.push(&self.document);
                    ui.close_menu();
                }
            }
        });

        // Thickness presets
        ui.label(egui::RichText::new("Thickness").size(9.5).color(self.theme.text_dim));
        ui.horizontal(|ui| {
            for (w, label, tip) in [(1.0_f32, "─", "Thin"), (2.5, "━", "Normal"), (5.0, "▬", "Thick"), (9.0, "█", "Bold")] {
                let is_cur = self.document.find_edge(&edge_id)
                    .map(|e| (e.style.width - w).abs() < 0.5)
                    .unwrap_or(false);
                let btn = egui::Button::new(
                    egui::RichText::new(label).size(14.0).color(if is_cur { self.theme.accent } else { self.theme.text_secondary })
                ).fill(if is_cur { self.theme.surface1 } else { self.theme.surface0 })
                 .min_size(egui::Vec2::new(36.0, 28.0));
                if ui.add(btn).on_hover_text(tip).clicked() {
                    if let Some(e) = self.document.find_edge_mut(&edge_id) { e.style.width = w; }
                    self.history.push(&self.document);
                    ui.close_menu();
                }
            }
        });

        // Style toggles
        ui.horizontal(|ui| {
            let (is_dashed, is_animated, is_ortho, is_glow) = self.document.find_edge(&edge_id)
                .map(|e| (e.style.dashed, e.style.animated, e.style.orthogonal, e.style.glow))
                .unwrap_or_default();
            let tog = |ui: &mut egui::Ui, active: bool, label: &str, tip: &str| {
                ui.add(egui::Button::new(egui::RichText::new(label).size(11.0)
                    .color(if active { self.theme.accent } else { self.theme.text_dim }))
                    .fill(if active { self.theme.surface1 } else { self.theme.surface0 })
                    .min_size(egui::Vec2::new(44.0, 24.0)))
                    .on_hover_text(tip).clicked()
            };
            if tog(ui, is_dashed,   "- - -",  "Dashed")       { if let Some(e) = self.document.find_edge_mut(&edge_id) { e.style.dashed = !e.style.dashed; }       self.history.push(&self.document); ui.close_menu(); }
            if tog(ui, is_animated, "→→→",    "Animated flow") { if let Some(e) = self.document.find_edge_mut(&edge_id) { e.style.animated = !e.style.animated; }   self.history.push(&self.document); ui.close_menu(); }
            if tog(ui, is_ortho,    "┐",      "Orthogonal")    { if let Some(e) = self.document.find_edge_mut(&edge_id) { e.style.orthogonal = !e.style.orthogonal; } self.history.push(&self.document); ui.close_menu(); }
            if tog(ui, is_glow,     "✦",      "Glow")          { if let Some(e) = self.document.find_edge_mut(&edge_id) { e.style.glow = !e.style.glow; }           self.history.push(&self.document); ui.close_menu(); }
        });
        ui.separator();

        if ui.button("⇄ Reverse direction").clicked() {
            if let Some(e) = self.document.find_edge_mut(&edge_id) { std::mem::swap(&mut e.source, &mut e.target); }
            self.history.push(&self.document);
            ui.close_menu();
        }
        if ui.button("↺ Reset style").clicked() {
            if let Some(e) = self.document.find_edge_mut(&edge_id) { e.style = EdgeStyle::default(); }
            self.history.push(&self.document);
            ui.close_menu();
        }
        // Sync edge color to source node fill
        {
            let src_color = self.document.find_edge(&edge_id)
                .and_then(|e| self.document.find_node(&e.source.node_id))
                .map(|n| n.style.fill_color);
            if let Some(col) = src_color {
                if ui.button("🎨 Sync color to source node").clicked() {
                    if let Some(e) = self.document.find_edge_mut(&edge_id) { e.style.color = col; }
                    self.history.push(&self.document);
                    ui.close_menu();
                }
            }
        }
        if ui.button("🗑 Delete edge").clicked() {
            self.document.remove_edge(&edge_id);
            self.selection.clear();
            self.history.push(&self.document);
            ui.close_menu();
        }
    }

    // ── Empty canvas ─────────────────────────────────────────────────────

    fn context_menu_canvas(&mut self, ui: &mut egui::Ui, canvas_pos: Pos2) {
        ui.label(egui::RichText::new("Canvas").size(10.0).color(self.theme.text_dim));
        ui.separator();

        // Add node submenu
        ui.menu_button("➕ Add Node…", |ui| {
            for (shape, label) in [
                (NodeShape::Rectangle,   "□ Rectangle"),
                (NodeShape::RoundedRect, "▢ Rounded"),
                (NodeShape::Diamond,     "◇ Diamond"),
                (NodeShape::Circle,      "○ Circle"),
                (NodeShape::Hexagon,     "⬡ Hexagon"),
            ] {
                if ui.button(label).clicked() {
                    let w = 140.0_f32; let h = 60.0_f32;
                    let pos = Pos2::new(canvas_pos.x - w / 2.0, canvas_pos.y - h / 2.0);
                    let mut node = Node::new(shape, pos);
                    node.size = [w, h];
                    let id = node.id;
                    self.document.nodes.push(node);
                    self.selection.select_node(id);
                    self.focus_label_edit = true;
                    self.history.push(&self.document);
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui.button("📝 Sticky Note").clicked() {
                let n = Node::new_sticky(StickyColor::Yellow,
                    Pos2::new(canvas_pos.x - 75.0, canvas_pos.y - 75.0));
                self.selection.select_node(n.id);
                self.document.nodes.push(n);
                self.history.push(&self.document);
                ui.close_menu();
            }
            if ui.button("⬜ Frame").clicked() {
                let n = Node::new_frame(Pos2::new(canvas_pos.x - 150.0, canvas_pos.y - 110.0));
                self.selection.select_node(n.id);
                self.document.nodes.push(n);
                self.history.push(&self.document);
                ui.close_menu();
            }
        });

        if !self.clipboard.is_empty() {
            if ui.button(format!("📋 Paste ({} node(s))", self.clipboard.len())).clicked() {
                self.selection.clear();
                let n = self.clipboard.len() as f32;
                let centroid = self.clipboard.iter().fold(Vec2::ZERO, |a, nd| a + nd.pos().to_vec2()) / n;
                let shift = canvas_pos.to_vec2() - centroid;
                for tmpl in self.clipboard.clone() {
                    let mut nd = tmpl;
                    nd.id = NodeId::new();
                    nd.set_pos(nd.pos() + shift);
                    self.selection.node_ids.insert(nd.id);
                    self.document.nodes.push(nd);
                }
                self.history.push(&self.document);
                ui.close_menu();
            }
        }

        if ui.button("🔍 Select All").clicked() {
            for n in &self.document.nodes { self.selection.node_ids.insert(n.id); }
            ui.close_menu();
        }

        ui.separator();

        if ui.button("⊞ Fit to Content").clicked() {
            self.fit_to_content();
            ui.close_menu();
        }
        if ui.button("1:1 Reset Zoom").clicked() {
            self.viewport.zoom = 1.0;
            ui.close_menu();
        }

        ui.separator();

        if ui.button(if self.show_grid { "⊡ Hide Grid" } else { "⊞ Show Grid" }).clicked() {
            self.show_grid = !self.show_grid;
            ui.close_menu();
        }
        if ui.button(if self.snap_to_grid { "⊠ Snap Off" } else { "⊟ Snap to Grid" }).clicked() {
            self.snap_to_grid = !self.snap_to_grid;
            ui.close_menu();
        }

        if !self.document.edges.is_empty() {
            ui.separator();
            if ui.button("🎨 Sync all edge colors to source").clicked() {
                for i in 0..self.document.edges.len() {
                    let src_id = self.document.edges[i].source.node_id;
                    if let Some(col) = self.document.nodes.iter()
                        .find(|n| n.id == src_id)
                        .map(|n| n.style.fill_color)
                    {
                        self.document.edges[i].style.color = col;
                    }
                }
                self.history.push(&self.document);
                ui.close_menu();
            }
        }
    }

    // ── Shared: tag submenu ──────────────────────────────────────────────

    /// Draw a tag submenu. If `node_id` is Some, tag that node; otherwise tag all selected.
    fn tag_submenu(&mut self, ui: &mut egui::Ui, label: &str, node_id: Option<NodeId>) {
        ui.menu_button(label, |ui| {
            let tags = [
                (None,                             "None"),
                (Some(crate::model::NodeTag::Critical), "🔴 Critical"),
                (Some(crate::model::NodeTag::Warning),  "🟡 Warning"),
                (Some(crate::model::NodeTag::Ok),       "🟢 OK"),
                (Some(crate::model::NodeTag::Info),     "🔵 Info"),
            ];
            for (variant, tag_label) in tags {
                if ui.button(tag_label).clicked() {
                    if let Some(nid) = node_id {
                        if let Some(n) = self.document.find_node_mut(&nid) { n.tag = variant; }
                    } else {
                        let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                        for id in ids {
                            if let Some(n) = self.document.find_node_mut(&id) { n.tag = variant; }
                        }
                    }
                    self.history.push(&self.document);
                    ui.close_menu();
                }
            }
        });
    }
}
