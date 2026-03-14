//! Command palette — Cmd+K searchable action list.

use egui::{Align2, Color32, Frame, Key, Margin, RichText, Stroke, Vec2};

use super::{BgPattern, DiagramMode, FlowchartApp};
use crate::model::NodeId;

struct PaletteEntry {
    icon:     &'static str,
    label:    &'static str,
    category: &'static str,
    action:   PaletteAction,
}

#[derive(Clone, Copy)]
enum PaletteAction {
    FitAll,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    SelectAll,
    Deselect,
    DeleteSelected,
    Undo,
    Redo,
    Duplicate,
    CopyStyle,
    PasteStyle,
    AutoLayout,
    ToggleGrid,
    ToggleSnap,
    ToggleFocusMode,
    TogglePresentation,
    ToggleFlowAnimation,
    ToggleDarkMode,
    SwitchToFlowchart,
    SwitchToER,
    SwitchToFigJam,
    ToggleToolbarCollapse,
    TogglePropertiesCollapse,
    SetBgDots,
    SetBgLines,
    SetBgCrosshatch,
    SetBgNone,
    OpenFindReplace,
    OpenSearch,
    ExportMermaid,
    ToggleTimelineMode,
    LoadHypothesisTemplate,
    LoadSwotTemplate,
    LoadRoadmapTemplate,
    LoadForceFieldTemplate,
    LoadLeanCanvasTemplate,
    LoadOkrTreeTemplate,
    LoadFiveWhysTemplate,
    LoadImpactEffortTemplate,
    LoadCustomerJourneyTemplate,
}

impl FlowchartApp {
    pub(crate) fn draw_command_palette(&mut self, ctx: &egui::Context) {
        // Open/close on Cmd+K
        let open_shortcut = ctx.input(|i| {
            i.key_pressed(Key::K) && i.modifiers.command && !i.modifiers.shift && !i.modifiers.alt
        });
        if open_shortcut {
            self.show_command_palette = !self.show_command_palette;
            if self.show_command_palette {
                self.command_palette_query.clear();
                self.command_palette_cursor = 0;
            }
        }
        if !self.show_command_palette { return; }

        // Close on Escape
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.show_command_palette = false;
            return;
        }

        let entries = build_entries();
        let q = self.command_palette_query.to_lowercase();
        let matches: Vec<&PaletteEntry> = if q.is_empty() {
            entries.iter().collect()
        } else {
            entries.iter().filter(|e| {
                e.label.to_lowercase().contains(&q) || e.category.to_lowercase().contains(&q)
            }).collect()
        };

        // Clamp cursor
        let max_idx = matches.len().saturating_sub(1);
        if self.command_palette_cursor > max_idx { self.command_palette_cursor = max_idx; }

        // Arrow key navigation (no text-edit conflict; palette has focus)
        let up   = ctx.input(|i| i.key_pressed(Key::ArrowUp));
        let down = ctx.input(|i| i.key_pressed(Key::ArrowDown));
        if down && self.command_palette_cursor < max_idx { self.command_palette_cursor += 1; }
        if up   && self.command_palette_cursor > 0       { self.command_palette_cursor -= 1; }

        let enter = ctx.input(|i| i.key_pressed(Key::Enter));
        let mut execute_action: Option<PaletteAction> = None;
        if enter {
            if let Some(entry) = matches.get(self.command_palette_cursor) {
                execute_action = Some(entry.action);
            }
            self.show_command_palette = false;
        }

        let mantle = self.theme.mantle;
        let surface0 = self.theme.surface0;
        let surface1 = self.theme.surface1;
        let text_dim = self.theme.text_dim;
        let text_primary = self.theme.text_primary;
        let text_secondary = self.theme.text_secondary;
        let accent = self.theme.accent;

        egui::Window::new("##cmd_palette")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_TOP, [0.0, 64.0])
            .fixed_size([480.0, 360.0])
            .frame(
                Frame::NONE
                    .fill(mantle)
                    .stroke(Stroke::new(1.0, surface1))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(Margin::same(0)),
            )
            .show(ctx, |ui| {
                // ── Search input ─────────────────────────────────────────
                Frame::NONE
                    .fill(surface0)
                    .inner_margin(Margin { left: 14, right: 14, top: 10, bottom: 10 })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⌘").size(13.0).color(text_dim));
                            ui.add_space(6.0);
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.command_palette_query)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Type an action…")
                                    .frame(false)
                                    .text_color(text_primary)
                                    .font(egui::FontId::proportional(14.0)),
                            );
                            // Auto-focus input
                            if ui.memory(|m| m.focused() != Some(resp.id)) {
                                resp.request_focus();
                            }
                        });
                    });

                ui.add(egui::Separator::default().spacing(0.0));

                // ── Results ───────────────────────────────────────────────
                egui::ScrollArea::vertical().max_height(295.0).show(ui, |ui| {
                    ui.add_space(4.0);
                    let mut last_cat = "";
                    for (i, entry) in matches.iter().enumerate() {
                        if entry.category != last_cat {
                            if !last_cat.is_empty() { ui.add_space(2.0); }
                            ui.add_space(2.0);
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(entry.category)
                                    .size(9.5)
                                    .color(text_dim)
                                    .strong(),
                            );
                            last_cat = entry.category;
                        }

                        let is_sel = i == self.command_palette_cursor;

                        let row = Frame::NONE
                            .fill(if is_sel { surface0 } else { Color32::TRANSPARENT })
                            .corner_radius(egui::CornerRadius::same(5))
                            .inner_margin(Margin { left: 10, right: 10, top: 4, bottom: 4 })
                            .show(ui, |ui| {
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.label(
                                            RichText::new(entry.icon)
                                                .size(13.0)
                                                .color(if is_sel { accent } else { text_dim }),
                                        );
                                        ui.add_space(6.0);
                                        ui.label(
                                            RichText::new(entry.label)
                                                .size(13.0)
                                                .color(if is_sel { text_primary } else { text_secondary }),
                                        );
                                    },
                                );
                            });

                        if row.response.interact(egui::Sense::click()).clicked() {
                            execute_action = Some(entry.action);
                        }
                        if row.response.hovered() {
                            self.command_palette_cursor = i;
                        }
                    }

                    if matches.is_empty() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("No matching actions").size(13.0).color(text_dim));
                        });
                    }
                    ui.add_space(4.0);
                });
            });

        if execute_action.is_some() {
            self.show_command_palette = false;
        }

        if let Some(action) = execute_action {
            self.run_palette_action(action, ctx);
            ctx.request_repaint();
        }
    }

    fn run_palette_action(&mut self, action: PaletteAction, ctx: &egui::Context) {
        match action {
            PaletteAction::FitAll => { self.pending_fit = true; }
            PaletteAction::ZoomIn  => { self.zoom_target *= 1.25; }
            PaletteAction::ZoomOut => { self.zoom_target /= 1.25; }
            PaletteAction::ZoomReset => { self.zoom_target = 1.0; }

            PaletteAction::SelectAll => {
                for n in &self.document.nodes { self.selection.node_ids.insert(n.id); }
            }
            PaletteAction::Deselect => { self.selection.clear(); }

            PaletteAction::DeleteSelected => {
                let node_ids: Vec<crate::model::NodeId> = self.selection.node_ids.iter()
                    .filter(|id| !self.document.find_node(id).map_or(false, |n| n.locked))
                    .copied().collect();
                let edge_ids: Vec<crate::model::EdgeId> = self.selection.edge_ids.iter().copied().collect();
                for id in &node_ids { self.document.remove_node(id); }
                for id in &edge_ids { self.document.remove_edge(id); }
                if !node_ids.is_empty() || !edge_ids.is_empty() {
                    self.selection.clear();
                    self.history.push(&self.document);
                    self.status_message = Some(("Deleted".to_string(), std::time::Instant::now()));
                }
            }

            PaletteAction::Undo => {
                if let Some(doc) = self.history.undo() {
                    self.document = doc.clone();
                    self.selection.clear();
                }
            }
            PaletteAction::Redo => {
                if let Some(doc) = self.history.redo() {
                    self.document = doc.clone();
                    self.selection.clear();
                }
            }

            PaletteAction::Duplicate => {
                if !self.selection.node_ids.is_empty() {
                    let base_offset = Vec2::new(24.0, 24.0);
                    let originals: Vec<crate::model::Node> = self.selection.node_ids.iter()
                        .filter_map(|id| self.document.find_node(id).cloned())
                        .collect();
                    self.selection.clear();
                    for template in originals {
                        let mut node = template.clone();
                        node.id = NodeId::new();
                        let mut candidate = template.pos() + base_offset;
                        for _ in 0..8 {
                            let snap_r = egui::Rect::from_min_size(candidate, node.size_vec());
                            if !self.document.nodes.iter().any(|n| n.rect().expand(-4.0).intersects(snap_r)) { break; }
                            candidate = candidate + base_offset;
                        }
                        node.set_pos(candidate);
                        self.selection.node_ids.insert(node.id);
                        self.document.nodes.push(node);
                    }
                    self.history.push(&self.document);
                    self.status_message = Some(("Duplicated".to_string(), std::time::Instant::now()));
                }
            }

            PaletteAction::CopyStyle => {
                if let Some(id) = self.selection.node_ids.iter().next().copied() {
                    if let Some(n) = self.document.find_node(&id) {
                        self.style_clipboard = Some(n.style.clone());
                        self.status_message = Some(("Style copied".to_string(), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::PasteStyle => {
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

            PaletteAction::AutoLayout => {
                let mut doc_clone = self.document.clone();
                for node in doc_clone.nodes.iter_mut() { if !node.pinned { node.position = [0.0, 0.0]; } }
                crate::specgraph::layout::auto_layout(&mut doc_clone);
                self.layout_targets.clear();
                for node in &doc_clone.nodes { self.layout_targets.insert(node.id, node.position); }
                self.pending_fit = true;
                self.status_message = Some(("Layout animating…".to_string(), std::time::Instant::now()));
            }

            PaletteAction::ToggleGrid             => { self.show_grid = !self.show_grid; }
            PaletteAction::ToggleSnap             => { self.snap_to_grid = !self.snap_to_grid; }
            PaletteAction::ToggleFocusMode        => { self.focus_mode = !self.focus_mode; }
            PaletteAction::TogglePresentation     => { self.presentation_mode = !self.presentation_mode; }
            PaletteAction::ToggleFlowAnimation    => { self.show_flow_animation = !self.show_flow_animation; }
            PaletteAction::ToggleDarkMode         => { self.toggle_dark_mode(ctx); }
            PaletteAction::SwitchToFlowchart      => { self.diagram_mode = DiagramMode::Flowchart; }
            PaletteAction::SwitchToER             => { self.diagram_mode = DiagramMode::ER; }
            PaletteAction::SwitchToFigJam         => { self.diagram_mode = DiagramMode::FigJam; }
            PaletteAction::ToggleToolbarCollapse  => { self.toolbar_collapsed = !self.toolbar_collapsed; }
            PaletteAction::TogglePropertiesCollapse => { self.properties_collapsed = !self.properties_collapsed; }
            PaletteAction::SetBgDots              => { self.bg_pattern = BgPattern::Dots; }
            PaletteAction::SetBgLines             => { self.bg_pattern = BgPattern::Lines; }
            PaletteAction::SetBgCrosshatch        => { self.bg_pattern = BgPattern::Crosshatch; }
            PaletteAction::SetBgNone              => { self.bg_pattern = BgPattern::None; }
            PaletteAction::OpenFindReplace        => { self.show_find_replace = true; self.find_query.clear(); }
            PaletteAction::OpenSearch             => { self.show_search = true; self.search_query.clear(); }
            PaletteAction::ExportMermaid          => {
                let mermaid = crate::app::export_mermaid::to_mermaid(&self.document);
                ctx.copy_text(mermaid);
                self.status_message = Some(("Mermaid copied to clipboard".to_string(), std::time::Instant::now()));
            }
            PaletteAction::ToggleTimelineMode => {
                self.document.timeline_mode = !self.document.timeline_mode;
                if self.document.timeline_mode {
                    crate::specgraph::layout::auto_layout(&mut self.document);
                    self.pending_fit = true;
                    self.status_message = Some(("Timeline mode on".to_string(), std::time::Instant::now()));
                } else {
                    self.status_message = Some(("Timeline mode off".to_string(), std::time::Instant::now()));
                }
                self.history.push(&self.document);
            }
            PaletteAction::LoadHypothesisTemplate => {
                let spec = include_str!("../../assets/examples/hypothesis_map.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Hypothesis map loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadSwotTemplate => {
                let spec = include_str!("../../assets/examples/swot_analysis.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("SWOT analysis loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadRoadmapTemplate => {
                let spec = include_str!("../../assets/examples/timeline_roadmap.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Roadmap template loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadForceFieldTemplate => {
                let spec = include_str!("../../assets/examples/force_field.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Force field loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadLeanCanvasTemplate => {
                let spec = include_str!("../../assets/examples/lean_canvas.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Lean canvas loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadOkrTreeTemplate => {
                let spec = include_str!("../../assets/examples/okr_tree.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("OKR tree loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadFiveWhysTemplate => {
                let spec = include_str!("../../assets/examples/five_whys.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("5 Whys loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadImpactEffortTemplate => {
                let spec = include_str!("../../assets/examples/impact_effort.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Impact/Effort matrix loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadCustomerJourneyTemplate => {
                let spec = include_str!("../../assets/examples/customer_journey.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Customer journey loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
        }
    }
}

fn build_entries() -> Vec<PaletteEntry> {
    vec![
        // View
        PaletteEntry { icon: "⊙", label: "Fit all to view",           category: "View",    action: PaletteAction::FitAll },
        PaletteEntry { icon: "⊕", label: "Zoom in",                   category: "View",    action: PaletteAction::ZoomIn },
        PaletteEntry { icon: "⊖", label: "Zoom out",                  category: "View",    action: PaletteAction::ZoomOut },
        PaletteEntry { icon: "⊟", label: "Zoom to 100%",              category: "View",    action: PaletteAction::ZoomReset },
        PaletteEntry { icon: "⊞", label: "Toggle grid",               category: "View",    action: PaletteAction::ToggleGrid },
        PaletteEntry { icon: "⊠", label: "Toggle snap to grid",       category: "View",    action: PaletteAction::ToggleSnap },
        PaletteEntry { icon: "◎", label: "Focus mode",                category: "View",    action: PaletteAction::ToggleFocusMode },
        PaletteEntry { icon: "▣", label: "Presentation mode",         category: "View",    action: PaletteAction::TogglePresentation },
        PaletteEntry { icon: "≋", label: "Flow animation",            category: "View",    action: PaletteAction::ToggleFlowAnimation },
        PaletteEntry { icon: "☀", label: "Toggle dark/light mode",   category: "View",    action: PaletteAction::ToggleDarkMode },
        PaletteEntry { icon: "·", label: "Background: dots",          category: "View",    action: PaletteAction::SetBgDots },
        PaletteEntry { icon: "—", label: "Background: lines",         category: "View",    action: PaletteAction::SetBgLines },
        PaletteEntry { icon: "#", label: "Background: crosshatch",    category: "View",    action: PaletteAction::SetBgCrosshatch },
        PaletteEntry { icon: " ", label: "Background: none",          category: "View",    action: PaletteAction::SetBgNone },
        // Panels
        PaletteEntry { icon: "◀", label: "Toggle left toolbar",       category: "Panels",  action: PaletteAction::ToggleToolbarCollapse },
        PaletteEntry { icon: "▶", label: "Toggle right properties",   category: "Panels",  action: PaletteAction::TogglePropertiesCollapse },
        // Edit
        PaletteEntry { icon: "↩", label: "Undo",                      category: "Edit",    action: PaletteAction::Undo },
        PaletteEntry { icon: "↪", label: "Redo",                      category: "Edit",    action: PaletteAction::Redo },
        PaletteEntry { icon: "⧉", label: "Duplicate selected",        category: "Edit",    action: PaletteAction::Duplicate },
        PaletteEntry { icon: "✕", label: "Delete selected",           category: "Edit",    action: PaletteAction::DeleteSelected },
        PaletteEntry { icon: "⊙", label: "Auto-layout (hierarchical)",category: "Edit",    action: PaletteAction::AutoLayout },
        PaletteEntry { icon: "≡", label: "Copy node style",           category: "Edit",    action: PaletteAction::CopyStyle },
        PaletteEntry { icon: "≣", label: "Paste node style",          category: "Edit",    action: PaletteAction::PasteStyle },
        // Selection
        PaletteEntry { icon: "⬚", label: "Select all",                category: "Select",  action: PaletteAction::SelectAll },
        PaletteEntry { icon: "⊘", label: "Deselect all",              category: "Select",  action: PaletteAction::Deselect },
        // Diagram
        PaletteEntry { icon: "⬡", label: "Switch to Flowchart mode",  category: "Diagram", action: PaletteAction::SwitchToFlowchart },
        PaletteEntry { icon: "◫", label: "Switch to ER mode",         category: "Diagram", action: PaletteAction::SwitchToER },
        PaletteEntry { icon: "★", label: "Switch to FigJam mode",     category: "Diagram", action: PaletteAction::SwitchToFigJam },
        PaletteEntry { icon: "⊟", label: "Toggle timeline mode",      category: "Diagram", action: PaletteAction::ToggleTimelineMode },
        PaletteEntry { icon: "💡", label: "Load hypothesis map template",    category: "Templates", action: PaletteAction::LoadHypothesisTemplate },
        PaletteEntry { icon: "⊞", label: "Load SWOT analysis template",     category: "Templates", action: PaletteAction::LoadSwotTemplate },
        PaletteEntry { icon: "📅", label: "Load roadmap timeline template",  category: "Templates", action: PaletteAction::LoadRoadmapTemplate },
        PaletteEntry { icon: "⇄",  label: "Load force field analysis",       category: "Templates", action: PaletteAction::LoadForceFieldTemplate },
        PaletteEntry { icon: "◫",  label: "Load lean canvas template",       category: "Templates", action: PaletteAction::LoadLeanCanvasTemplate },
        PaletteEntry { icon: "⊙",  label: "Load OKR tree template",          category: "Templates", action: PaletteAction::LoadOkrTreeTemplate },
        PaletteEntry { icon: "❓",  label: "Load 5 Whys root cause template", category: "Templates", action: PaletteAction::LoadFiveWhysTemplate },
        PaletteEntry { icon: "⊞",  label: "Load Impact/Effort matrix",        category: "Templates", action: PaletteAction::LoadImpactEffortTemplate },
        PaletteEntry { icon: "🗺",  label: "Load customer journey map",        category: "Templates", action: PaletteAction::LoadCustomerJourneyTemplate },
        // Search
        PaletteEntry { icon: "🔍", label: "Search nodes",              category: "Search",  action: PaletteAction::OpenSearch },
        PaletteEntry { icon: "⇄",  label: "Find & Replace",            category: "Search",  action: PaletteAction::OpenFindReplace },
        // Export
        PaletteEntry { icon: "⎘",  label: "Copy as Mermaid to clipboard", category: "Export", action: PaletteAction::ExportMermaid },
    ]
}
