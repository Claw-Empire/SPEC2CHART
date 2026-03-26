//! Command palette — Cmd+K searchable action list.

use egui::{Align2, Color32, Frame, Key, Margin, RichText, Stroke, Vec2};
use std::borrow::Cow;

use super::{BgPattern, DiagramMode, FlowchartApp};
use crate::model::NodeId;

struct PaletteEntry {
    icon:     &'static str,
    label:    Cow<'static, str>,
    category: &'static str,
    action:   PaletteAction,
}

#[derive(Clone)]
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
    LoadDecisionRecordTemplate,
    LoadEmpathyMapTemplate,
    LoadValuePropositionTemplate,
    LoadFishboneTemplate,
    LoadPestleTemplate,
    LoadMindMapTemplate,
    LoadPremorttemTemplate,
    LoadRoseBudThornTemplate,
    LoadDoubleDiamondTemplate,
    LoadAssumptionMapTemplate,
    LoadBusinessModelCanvasTemplate,
    LoadHypothesisCanvasTemplate,
    LoadStoryMapTemplate,
    LoadIceScoringTemplate,
    LoadJobsToBeDoneTemplate,
    LoadCausalLoopTemplate,
    LoadExperimentBoardTemplate,
    LoadTheoryOfChangeTemplate,
    LoadCompetitiveAnalysisTemplate,
    LoadWhatSoWhatTemplate,
    LoadTwoByTwoMatrixTemplate,
    LoadDesignSprintTemplate,
    LoadProblemSolutionFitTemplate,
    // Support Ops templates
    LoadSupportTicketFlowTemplate,
    LoadIncidentResponseTemplate,
    LoadSupportEscalationMatrixTemplate,
    LoadBugTriageTemplate,
    LoadKnowledgeBaseStructureTemplate,
    LoadVoiceOfCustomerTemplate,
    LoadCustomerOnboardingTemplate,
    LoadSupportHealthDashboardTemplate,
    LoadPostmortemTemplate,
    LoadSupportSLAMatrixTemplate,
    LoadSupportQueueTemplate,
    CollapseAll,
    ExpandAll,
    InvertSelection,
    SelectSimilarShape,
    SelectSimilarTag,
    SelectSimilarSection,
    QuickStats,
    SelectOrphans,
    RandomizeColors,
    ResetView,
    SelectAllEdges,
    SelectEdgesOfSelection,
    CopyLabelsToClipboard,
    OpenRecentFile(usize),
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

        let recent_entries: Vec<PaletteEntry> = self.recent_files
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let filename = path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                PaletteEntry {
                    icon:     "📄",
                    label:    filename.into(),
                    category: "Recent",
                    action:   PaletteAction::OpenRecentFile(i),
                }
            })
            .collect();
        let entries = build_entries();
        let q = self.command_palette_query.to_lowercase();
        let recent_limit = if q.is_empty() { 5 } else { recent_entries.len() };
        let recent_matches: Vec<&PaletteEntry> = recent_entries.iter()
            .filter(|e| q.is_empty() || e.label.to_lowercase().contains(&q))
            .take(recent_limit)
            .collect();
        let builtin_matches: Vec<&PaletteEntry> = entries.iter()
            .filter(|e| q.is_empty() || e.label.to_lowercase().contains(&q) || e.category.to_lowercase().contains(&q))
            .collect();
        let matches: Vec<&PaletteEntry> = recent_matches.into_iter()
            .chain(builtin_matches)
            .collect();

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
                execute_action = Some(entry.action.clone());
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

                    // Getting started tips when query is empty
                    if q.is_empty() && self.document.nodes.is_empty() {
                        Frame::NONE
                            .fill(surface0)
                            .corner_radius(egui::CornerRadius::same(6))
                            .inner_margin(Margin { left: 12, right: 12, top: 8, bottom: 8 })
                            .show(ui, |ui| {
                                ui.label(RichText::new("Getting Started").size(10.0).strong().color(accent));
                                ui.add_space(4.0);
                                let tips = [
                                    "Try \"template\" to load a pre-built diagram",
                                    "Double-click the canvas to create your first node",
                                    "Press E to switch to Connect mode and draw edges",
                                    "Try \"dark\" or \"light\" to switch themes",
                                ];
                                for tip in tips {
                                    ui.label(RichText::new(format!("  {}", tip)).size(11.0).color(text_dim));
                                }
                            });
                        ui.add_space(6.0);
                    }
                    // Power tips for active diagrams
                    if q.is_empty() && !self.document.nodes.is_empty() {
                        let power_tips = [
                            "Tab / Shift+Tab — navigate connected nodes by keyboard",
                            "Cmd+Shift+D — duplicate node and auto-connect (chain workflow)",
                            "Cmd+/ — toggle done/undone on selected nodes",
                            "Focus mode — dim unrelated nodes to concentrate on a subgraph",
                            "Cmd+Shift+F — pin search filter to keep dimming while editing",
                            "Select Similar — find all nodes with same shape, tag, or section",
                            "Select Orphans — find unconnected nodes in your diagram",
                        ];
                        // Rotate tips: show 2 at a time based on seconds
                        let idx = (std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() / 8) as usize;
                        let tip1 = power_tips[idx % power_tips.len()];
                        let tip2 = power_tips[(idx + 1) % power_tips.len()];
                        Frame::NONE
                            .fill(surface0)
                            .corner_radius(egui::CornerRadius::same(6))
                            .inner_margin(Margin { left: 12, right: 12, top: 6, bottom: 6 })
                            .show(ui, |ui| {
                                ui.label(RichText::new("Tip").size(9.5).strong().color(accent.gamma_multiply(0.7)));
                                ui.label(RichText::new(format!("  {}", tip1)).size(10.5).color(text_dim));
                                ui.label(RichText::new(format!("  {}", tip2)).size(10.5).color(text_dim));
                            });
                        ui.add_space(4.0);
                    }

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
                                            RichText::new(entry.label.as_ref())
                                                .size(13.0)
                                                .color(if is_sel { text_primary } else { text_secondary }),
                                        );
                                    },
                                );
                            });

                        if row.response.interact(egui::Sense::click()).clicked() {
                            execute_action = Some(entry.action.clone());
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
                    let mut parts = Vec::new();
                    if !node_ids.is_empty() { parts.push(format!("{} node{}", node_ids.len(), if node_ids.len() == 1 { "" } else { "s" })); }
                    if !edge_ids.is_empty() { parts.push(format!("{} edge{}", edge_ids.len(), if edge_ids.len() == 1 { "" } else { "s" })); }
                    self.selection.clear();
                    self.history.push(&self.document);
                    self.status_message = Some((format!("Deleted {} — Cmd+Z to undo", parts.join(", ")), std::time::Instant::now()));
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
                    let n = self.selection.node_ids.len();
                    self.history.push(&self.document);
                    self.status_message = Some((format!("Duplicated {} node{}", n, if n == 1 { "" } else { "s" }), std::time::Instant::now()));
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
            PaletteAction::ToggleFocusMode        => {
                self.focus_mode = !self.focus_mode;
                if self.focus_mode {
                    self.status_message = Some(("Focus mode on — unrelated nodes dimmed".to_string(), std::time::Instant::now()));
                } else {
                    self.status_message = Some(("Focus mode off".to_string(), std::time::Instant::now()));
                }
            }
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
            PaletteAction::LoadDecisionRecordTemplate => {
                let spec = include_str!("../../assets/examples/decision_record.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Decision record loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadEmpathyMapTemplate => {
                let spec = include_str!("../../assets/examples/empathy_map.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Empathy map loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadValuePropositionTemplate => {
                let spec = include_str!("../../assets/examples/value_proposition.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Value proposition canvas loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadFishboneTemplate => {
                let spec = include_str!("../../assets/examples/fishbone.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Fishbone diagram loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadPestleTemplate => {
                let spec = include_str!("../../assets/examples/pestle.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("PESTLE analysis loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadMindMapTemplate => {
                let spec = include_str!("../../assets/examples/mind_map.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Mind map loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadPremorttemTemplate => {
                let spec = include_str!("../../assets/examples/premortem.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Premortem analysis loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadRoseBudThornTemplate => {
                let spec = include_str!("../../assets/examples/rose_bud_thorn.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Rose-Bud-Thorn retro loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadDoubleDiamondTemplate => {
                let spec = include_str!("../../assets/examples/double_diamond.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Double Diamond loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadAssumptionMapTemplate => {
                let spec = include_str!("../../assets/examples/assumption_map.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Assumption Map loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadBusinessModelCanvasTemplate => {
                let spec = include_str!("../../assets/examples/business_model_canvas.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Business Model Canvas loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadHypothesisCanvasTemplate => {
                let spec = include_str!("../../assets/examples/hypothesis_canvas.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Hypothesis Validation Canvas loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadStoryMapTemplate => {
                let spec = include_str!("../../assets/examples/story_map.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("User Story Map loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadIceScoringTemplate => {
                let spec = include_str!("../../assets/examples/ice_scoring.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("ICE Scoring Matrix loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadJobsToBeDoneTemplate => {
                let spec = include_str!("../../assets/examples/jobs_to_be_done.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Jobs To Be Done Canvas loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadCausalLoopTemplate => {
                let spec = include_str!("../../assets/examples/causal_loop.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Causal Loop Diagram loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadExperimentBoardTemplate => {
                let spec = include_str!("../../assets/examples/experiment_board.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Experiment Board loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadTheoryOfChangeTemplate => {
                let spec = include_str!("../../assets/examples/theory_of_change.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Theory of Change loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadCompetitiveAnalysisTemplate => {
                let spec = include_str!("../../assets/examples/competitive_analysis.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Competitive Analysis loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadWhatSoWhatTemplate => {
                let spec = include_str!("../../assets/examples/what_so_what.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("What? So What? Now What? loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadTwoByTwoMatrixTemplate => {
                let spec = include_str!("../../assets/examples/two_by_two_matrix.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("2×2 Prioritization Matrix loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadDesignSprintTemplate => {
                let spec = include_str!("../../assets/examples/design_sprint.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Design Sprint (5-Day) loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadProblemSolutionFitTemplate => {
                let spec = include_str!("../../assets/examples/problem_solution_fit.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => {
                        self.document = doc;
                        self.selection.clear();
                        self.history.push(&self.document);
                        self.pending_fit = true;
                        self.status_message = Some(("Problem/Solution Fit loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::LoadSupportTicketFlowTemplate => {
                let spec = include_str!("../../assets/examples/support_ticket_flow.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Support Ticket Flow loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadIncidentResponseTemplate => {
                let spec = include_str!("../../assets/examples/incident_response.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Incident Response Runbook loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadSupportEscalationMatrixTemplate => {
                let spec = include_str!("../../assets/examples/support_escalation_matrix.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Support Escalation Matrix loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadBugTriageTemplate => {
                let spec = include_str!("../../assets/examples/bug_triage.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Bug Triage Process loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadKnowledgeBaseStructureTemplate => {
                let spec = include_str!("../../assets/examples/knowledge_base_structure.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Knowledge Base Structure loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadVoiceOfCustomerTemplate => {
                let spec = include_str!("../../assets/examples/voice_of_customer.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Voice of Customer loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadCustomerOnboardingTemplate => {
                let spec = include_str!("../../assets/examples/customer_onboarding.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Customer Onboarding Journey loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadSupportHealthDashboardTemplate => {
                let spec = include_str!("../../assets/examples/support_health_dashboard.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Support Health Dashboard loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadPostmortemTemplate => {
                let spec = include_str!("../../assets/examples/postmortem.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Postmortem template loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadSupportSLAMatrixTemplate => {
                let spec = include_str!("../../assets/examples/support_sla_matrix.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Support SLA Matrix loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::LoadSupportQueueTemplate => {
                let spec = include_str!("../../assets/examples/support_queue.spec");
                match crate::specgraph::hrf::parse_hrf(spec) {
                    Ok(doc) => { self.document = doc; self.selection.clear(); self.history.push(&self.document); self.pending_fit = true; self.status_message = Some(("Support Queue kanban loaded".to_string(), std::time::Instant::now())); }
                    Err(e) => { self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now())); }
                }
            }
            PaletteAction::InvertSelection => {
                let all_ids: std::collections::HashSet<crate::model::NodeId> = self.document.nodes.iter().map(|n| n.id).collect();
                let currently_selected = self.selection.node_ids.clone();
                self.selection.node_ids = all_ids.difference(&currently_selected).copied().collect();
                let n = self.selection.node_ids.len();
                self.status_message = Some((format!("Selection inverted — {} node{} selected", n, if n == 1 { "" } else { "s" }), std::time::Instant::now()));
            }
            PaletteAction::SelectSimilarShape => {
                if let Some(&ref_id) = self.selection.node_ids.iter().next() {
                    if let Some(ref_node) = self.document.find_node(&ref_id) {
                        let ref_shape = match &ref_node.kind {
                            crate::model::NodeKind::Shape { shape, .. } => Some(*shape),
                            _ => None,
                        };
                        if let Some(shape) = ref_shape {
                            let shape_name = shape.default_label();
                            self.selection.node_ids.clear();
                            for node in &self.document.nodes {
                                if let crate::model::NodeKind::Shape { shape: s, .. } = &node.kind {
                                    if *s == shape { self.selection.node_ids.insert(node.id); }
                                }
                            }
                            let n = self.selection.node_ids.len();
                            self.status_message = Some((format!("Selected {} {} node{}", n, shape_name, if n == 1 { "" } else { "s" }), std::time::Instant::now()));
                        }
                    }
                }
            }
            PaletteAction::SelectSimilarTag => {
                if let Some(&ref_id) = self.selection.node_ids.iter().next() {
                    if let Some(ref_node) = self.document.find_node(&ref_id) {
                        let ref_tag = ref_node.tag;
                        self.selection.node_ids.clear();
                        for node in &self.document.nodes {
                            if node.tag == ref_tag { self.selection.node_ids.insert(node.id); }
                        }
                        let tag_label = ref_tag.map(|t| t.label()).unwrap_or("no tag");
                        let n = self.selection.node_ids.len();
                        self.status_message = Some((format!("Selected {} node{} with tag \"{}\"", n, if n == 1 { "" } else { "s" }, tag_label), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::SelectSimilarSection => {
                if let Some(&ref_id) = self.selection.node_ids.iter().next() {
                    if let Some(ref_node) = self.document.find_node(&ref_id) {
                        let ref_section = ref_node.section_name.clone();
                        self.selection.node_ids.clear();
                        for node in &self.document.nodes {
                            if node.section_name == ref_section { self.selection.node_ids.insert(node.id); }
                        }
                        let sec_label = if ref_section.is_empty() { "no section" } else { &ref_section };
                        let n = self.selection.node_ids.len();
                        self.status_message = Some((format!("Selected {} node{} in \"{}\"", n, if n == 1 { "" } else { "s" }, sec_label), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::QuickStats => {
                let total_nodes = self.document.nodes.len();
                let total_edges = self.document.edges.len();
                let frames = self.document.nodes.iter().filter(|n| n.is_frame).count();
                let orphans = self.document.nodes.iter().filter(|n| {
                    !n.is_frame && !self.document.edges.iter().any(|e| e.source.node_id == n.id || e.target.node_id == n.id)
                }).count();
                let sections: std::collections::HashSet<&str> = self.document.nodes.iter()
                    .filter(|n| !n.section_name.is_empty())
                    .map(|n| n.section_name.as_str())
                    .collect();
                let mut msg = format!("{} nodes, {} edges", total_nodes, total_edges);
                if frames > 0 { msg.push_str(&format!(", {} frames", frames)); }
                if !sections.is_empty() { msg.push_str(&format!(", {} sections", sections.len())); }
                if orphans > 0 { msg.push_str(&format!(", {} unconnected", orphans)); }
                self.status_message = Some((msg, std::time::Instant::now()));
            }
            PaletteAction::CopyLabelsToClipboard => {
                let labels: Vec<String> = if self.selection.node_ids.is_empty() {
                    self.document.nodes.iter().map(|n| n.display_label().to_string()).collect()
                } else {
                    self.selection.node_ids.iter()
                        .filter_map(|id| self.document.find_node(id))
                        .map(|n| n.display_label().to_string())
                        .collect()
                };
                if labels.is_empty() {
                    self.status_message = Some(("No nodes to copy labels from".to_string(), std::time::Instant::now()));
                } else {
                    let text = labels.join("\n");
                    ctx.copy_text(text);
                    let n = labels.len();
                    self.status_message = Some((format!("Copied {} label{} to clipboard", n, if n == 1 { "" } else { "s" }), std::time::Instant::now()));
                }
            }
            PaletteAction::SelectAllEdges => {
                self.selection.edge_ids.clear();
                for edge in &self.document.edges {
                    self.selection.edge_ids.insert(edge.id);
                }
                let n = self.selection.edge_ids.len();
                self.status_message = Some((format!("Selected all {} edge{}", n, if n == 1 { "" } else { "s" }), std::time::Instant::now()));
            }
            PaletteAction::SelectEdgesOfSelection => {
                let sel_nodes = &self.selection.node_ids;
                if sel_nodes.is_empty() {
                    self.status_message = Some(("Select nodes first, then use this to select their edges".to_string(), std::time::Instant::now()));
                } else {
                    self.selection.edge_ids.clear();
                    for edge in &self.document.edges {
                        if sel_nodes.contains(&edge.source.node_id) && sel_nodes.contains(&edge.target.node_id) {
                            self.selection.edge_ids.insert(edge.id);
                        }
                    }
                    let n = self.selection.edge_ids.len();
                    if n == 0 {
                        self.status_message = Some(("No edges between selected nodes".to_string(), std::time::Instant::now()));
                    } else {
                        self.status_message = Some((format!("Selected {} edge{} between nodes", n, if n == 1 { "" } else { "s" }), std::time::Instant::now()));
                    }
                }
            }
            PaletteAction::ResetView => {
                self.zoom_target = 1.0;
                self.pan_target = Some([0.0, 0.0]);
                self.status_message = Some(("View reset to 100% — Cmd+1 to fit all".to_string(), std::time::Instant::now()));
            }
            PaletteAction::SelectOrphans => {
                self.selection.clear();
                for node in &self.document.nodes {
                    if node.is_frame { continue; }
                    let connected = self.document.edges.iter()
                        .any(|e| e.source.node_id == node.id || e.target.node_id == node.id);
                    if !connected {
                        self.selection.node_ids.insert(node.id);
                    }
                }
                let n = self.selection.node_ids.len();
                if n == 0 {
                    self.status_message = Some(("All nodes are connected — no orphans found".to_string(), std::time::Instant::now()));
                } else {
                    self.status_message = Some((format!("Selected {} unconnected node{}", n, if n == 1 { "" } else { "s" }), std::time::Instant::now()));
                }
            }
            PaletteAction::RandomizeColors => {
                let palette: &[[u8; 4]] = &[
                    [137, 180, 250, 255], // blue
                    [166, 227, 161, 255], // green
                    [243, 139, 168, 255], // red
                    [249, 226, 175, 255], // yellow
                    [203, 166, 247, 255], // purple
                    [148, 226, 213, 255], // teal
                    [250, 179, 135, 255], // orange
                    [245, 194, 231, 255], // pink
                    [180, 190, 254, 255], // lavender
                    [116, 199, 236, 255], // sapphire
                ];
                let ids: Vec<crate::model::NodeId> = self.selection.node_ids.iter().copied().collect();
                if ids.is_empty() {
                    self.status_message = Some(("Select nodes first, then randomize".to_string(), std::time::Instant::now()));
                } else {
                    for (i, id) in ids.iter().enumerate() {
                        if let Some(node) = self.document.find_node_mut(id) {
                            let color = palette[i % palette.len()];
                            node.style.fill_color = color;
                            node.style.text_color = super::theme::auto_contrast_text(color);
                        }
                    }
                    self.history.push(&self.document);
                    let n = ids.len();
                    self.status_message = Some((format!("Applied {} distinct colors", n.min(palette.len())), std::time::Instant::now()));
                }
            }
            PaletteAction::CollapseAll => {
                let mut count = 0;
                for node in &mut self.document.nodes {
                    if !node.collapsed && !node.is_frame {
                        node.uncollapsed_size = Some(node.size);
                        node.collapsed = true;
                        node.size[1] = 28.0;
                        count += 1;
                    }
                }
                self.history.push(&self.document);
                self.status_message = Some((format!("Collapsed {} nodes", count), std::time::Instant::now()));
            }
            PaletteAction::ExpandAll => {
                let mut count = 0;
                for node in &mut self.document.nodes {
                    if node.collapsed {
                        if let Some(size) = node.uncollapsed_size.take() {
                            node.size = size;
                        }
                        node.collapsed = false;
                        count += 1;
                    }
                }
                self.history.push(&self.document);
                self.status_message = Some((format!("Expanded {} nodes", count), std::time::Instant::now()));
            }
            PaletteAction::OpenRecentFile(idx) => {
                // SAFETY: idx was assigned from recent_entries.iter().enumerate() in this same frame,
                // before Window::show. Nothing mutates self.recent_files between that build and this
                // dispatch (single-threaded, synchronous egui). Index is valid for this frame.
                if let Some(path) = self.recent_files.get(idx).cloned() {
                    self.open_recent_file(path);
                }
            }
        }
    }
}

fn build_entries() -> Vec<PaletteEntry> {
    vec![
        // View
        PaletteEntry { icon: "⊙", label: "Fit all to view".into(),           category: "View",    action: PaletteAction::FitAll },
        PaletteEntry { icon: "⊕", label: "Zoom in".into(),                   category: "View",    action: PaletteAction::ZoomIn },
        PaletteEntry { icon: "⊖", label: "Zoom out".into(),                  category: "View",    action: PaletteAction::ZoomOut },
        PaletteEntry { icon: "⊟", label: "Zoom to 100%".into(),              category: "View",    action: PaletteAction::ZoomReset },
        PaletteEntry { icon: "⊞", label: "Toggle grid".into(),               category: "View",    action: PaletteAction::ToggleGrid },
        PaletteEntry { icon: "⊠", label: "Toggle snap to grid".into(),       category: "View",    action: PaletteAction::ToggleSnap },
        PaletteEntry { icon: "◎", label: "Focus mode".into(),                category: "View",    action: PaletteAction::ToggleFocusMode },
        PaletteEntry { icon: "▣", label: "Presentation mode".into(),         category: "View",    action: PaletteAction::TogglePresentation },
        PaletteEntry { icon: "≋", label: "Flow animation".into(),            category: "View",    action: PaletteAction::ToggleFlowAnimation },
        PaletteEntry { icon: "☀", label: "Toggle dark/light mode".into(),   category: "View",    action: PaletteAction::ToggleDarkMode },
        PaletteEntry { icon: "·", label: "Background: dots".into(),          category: "View",    action: PaletteAction::SetBgDots },
        PaletteEntry { icon: "—", label: "Background: lines".into(),         category: "View",    action: PaletteAction::SetBgLines },
        PaletteEntry { icon: "#", label: "Background: crosshatch".into(),    category: "View",    action: PaletteAction::SetBgCrosshatch },
        PaletteEntry { icon: " ", label: "Background: none".into(),          category: "View",    action: PaletteAction::SetBgNone },
        // Panels
        PaletteEntry { icon: "◀", label: "Toggle left toolbar".into(),       category: "Panels",  action: PaletteAction::ToggleToolbarCollapse },
        PaletteEntry { icon: "▶", label: "Toggle right properties".into(),   category: "Panels",  action: PaletteAction::TogglePropertiesCollapse },
        // Edit
        PaletteEntry { icon: "↩", label: "Undo".into(),                      category: "Edit",    action: PaletteAction::Undo },
        PaletteEntry { icon: "↪", label: "Redo".into(),                      category: "Edit",    action: PaletteAction::Redo },
        PaletteEntry { icon: "⧉", label: "Duplicate selected".into(),        category: "Edit",    action: PaletteAction::Duplicate },
        PaletteEntry { icon: "✕", label: "Delete selected".into(),           category: "Edit",    action: PaletteAction::DeleteSelected },
        PaletteEntry { icon: "⊙", label: "Auto-layout (hierarchical)".into(),category: "Edit",    action: PaletteAction::AutoLayout },
        PaletteEntry { icon: "≡", label: "Copy node style".into(),           category: "Edit",    action: PaletteAction::CopyStyle },
        PaletteEntry { icon: "≣", label: "Paste node style".into(),          category: "Edit",    action: PaletteAction::PasteStyle },
        PaletteEntry { icon: "▼", label: "Collapse all nodes".into(),          category: "Edit",    action: PaletteAction::CollapseAll },
        PaletteEntry { icon: "▲", label: "Expand all nodes".into(),            category: "Edit",    action: PaletteAction::ExpandAll },
        // Selection
        PaletteEntry { icon: "⬚", label: "Select all".into(),                category: "Select",  action: PaletteAction::SelectAll },
        PaletteEntry { icon: "⊘", label: "Deselect all".into(),              category: "Select",  action: PaletteAction::Deselect },
        PaletteEntry { icon: "⇄", label: "Invert selection".into(),          category: "Select",  action: PaletteAction::InvertSelection },
        PaletteEntry { icon: "◇", label: "Select similar shape".into(),       category: "Select",  action: PaletteAction::SelectSimilarShape },
        PaletteEntry { icon: "●", label: "Select similar tag".into(),         category: "Select",  action: PaletteAction::SelectSimilarTag },
        PaletteEntry { icon: "§", label: "Select similar section".into(),     category: "Select",  action: PaletteAction::SelectSimilarSection },
        PaletteEntry { icon: "📋", label: "Copy labels to clipboard".into(),   category: "Edit",    action: PaletteAction::CopyLabelsToClipboard },
        PaletteEntry { icon: "↔", label: "Select all edges".into(),           category: "Select",  action: PaletteAction::SelectAllEdges },
        PaletteEntry { icon: "⇌", label: "Select edges of selected nodes".into(), category: "Select", action: PaletteAction::SelectEdgesOfSelection },
        // Info
        PaletteEntry { icon: "ℹ", label: "Quick diagram stats".into(),        category: "Info",    action: PaletteAction::QuickStats },
        PaletteEntry { icon: "⊙", label: "Reset view to 100%".into(),         category: "View",    action: PaletteAction::ResetView },
        PaletteEntry { icon: "○", label: "Select orphan (unconnected) nodes".into(), category: "Select", action: PaletteAction::SelectOrphans },
        PaletteEntry { icon: "🎨", label: "Randomize colors of selected".into(),  category: "Edit",    action: PaletteAction::RandomizeColors },
        // Diagram
        PaletteEntry { icon: "⬡", label: "Switch to Flowchart mode".into(),  category: "Diagram", action: PaletteAction::SwitchToFlowchart },
        PaletteEntry { icon: "◫", label: "Switch to ER mode".into(),         category: "Diagram", action: PaletteAction::SwitchToER },
        PaletteEntry { icon: "★", label: "Switch to FigJam mode".into(),     category: "Diagram", action: PaletteAction::SwitchToFigJam },
        PaletteEntry { icon: "⊟", label: "Toggle timeline mode".into(),      category: "Diagram", action: PaletteAction::ToggleTimelineMode },
        PaletteEntry { icon: "💡", label: "Load hypothesis map template".into(),    category: "Templates", action: PaletteAction::LoadHypothesisTemplate },
        PaletteEntry { icon: "⊞", label: "Load SWOT analysis template".into(),     category: "Templates", action: PaletteAction::LoadSwotTemplate },
        PaletteEntry { icon: "📅", label: "Load roadmap timeline template".into(),  category: "Templates", action: PaletteAction::LoadRoadmapTemplate },
        PaletteEntry { icon: "⇄",  label: "Load force field analysis".into(),       category: "Templates", action: PaletteAction::LoadForceFieldTemplate },
        PaletteEntry { icon: "◫",  label: "Load lean canvas template".into(),       category: "Templates", action: PaletteAction::LoadLeanCanvasTemplate },
        PaletteEntry { icon: "⊙",  label: "Load OKR tree template".into(),          category: "Templates", action: PaletteAction::LoadOkrTreeTemplate },
        PaletteEntry { icon: "❓",  label: "Load 5 Whys root cause template".into(), category: "Templates", action: PaletteAction::LoadFiveWhysTemplate },
        PaletteEntry { icon: "⊞",  label: "Load Impact/Effort matrix".into(),        category: "Templates", action: PaletteAction::LoadImpactEffortTemplate },
        PaletteEntry { icon: "🗺",  label: "Load customer journey map".into(),        category: "Templates", action: PaletteAction::LoadCustomerJourneyTemplate },
        PaletteEntry { icon: "📋",  label: "Load decision record log (ADR)".into(),   category: "Templates", action: PaletteAction::LoadDecisionRecordTemplate },
        PaletteEntry { icon: "🧠",  label: "Load empathy map template".into(),        category: "Templates", action: PaletteAction::LoadEmpathyMapTemplate },
        PaletteEntry { icon: "💎",  label: "Load value proposition canvas".into(),    category: "Templates", action: PaletteAction::LoadValuePropositionTemplate },
        PaletteEntry { icon: "🐟",  label: "Load fishbone (Ishikawa) diagram".into(), category: "Templates", action: PaletteAction::LoadFishboneTemplate },
        PaletteEntry { icon: "🌍",  label: "Load PESTLE analysis template".into(),    category: "Templates", action: PaletteAction::LoadPestleTemplate },
        PaletteEntry { icon: "🧠",  label: "Load mind map template".into(),           category: "Templates", action: PaletteAction::LoadMindMapTemplate },
        PaletteEntry { icon: "💀",  label: "Load premortem analysis".into(),          category: "Templates", action: PaletteAction::LoadPremorttemTemplate },
        PaletteEntry { icon: "🌹",  label: "Load Rose-Bud-Thorn retro".into(),        category: "Templates", action: PaletteAction::LoadRoseBudThornTemplate },
        PaletteEntry { icon: "◈",   label: "Load Double Diamond design process".into(), category: "Templates", action: PaletteAction::LoadDoubleDiamondTemplate },
        PaletteEntry { icon: "⊞",  label: "Load Assumption Map (test vs assume)".into(), category: "Templates", action: PaletteAction::LoadAssumptionMapTemplate },
        PaletteEntry { icon: "🏢",  label: "Load Business Model Canvas (9-block)".into(), category: "Templates", action: PaletteAction::LoadBusinessModelCanvasTemplate },
        PaletteEntry { icon: "🧬",  label: "Load Hypothesis Validation Canvas".into(),    category: "Templates", action: PaletteAction::LoadHypothesisCanvasTemplate },
        PaletteEntry { icon: "🗺",  label: "Load User Story Map template".into(),         category: "Templates", action: PaletteAction::LoadStoryMapTemplate },
        PaletteEntry { icon: "🧊",  label: "Load ICE Scoring Matrix (prioritize experiments)".into(), category: "Templates", action: PaletteAction::LoadIceScoringTemplate },
        PaletteEntry { icon: "💼",  label: "Load Jobs To Be Done canvas".into(),          category: "Templates", action: PaletteAction::LoadJobsToBeDoneTemplate },
        PaletteEntry { icon: "🔁",  label: "Load Causal Loop Diagram (feedback loops)".into(), category: "Templates", action: PaletteAction::LoadCausalLoopTemplate },
        PaletteEntry { icon: "🗃",  label: "Load Experiment Board (backlog → running → validated)".into(), category: "Templates", action: PaletteAction::LoadExperimentBoardTemplate },
        PaletteEntry { icon: "🌱",  label: "Load Theory of Change (inputs → activities → impact)".into(), category: "Templates", action: PaletteAction::LoadTheoryOfChangeTemplate },
        PaletteEntry { icon: "⚔",   label: "Load Competitive Analysis Matrix".into(),      category: "Templates", action: PaletteAction::LoadCompetitiveAnalysisTemplate },
        PaletteEntry { icon: "🔁",  label: "Load What? So What? Now What? (debrief)".into(), category: "Templates", action: PaletteAction::LoadWhatSoWhatTemplate },
        PaletteEntry { icon: "⊞",  label: "Load 2×2 Prioritization Matrix (Impact vs Effort)".into(), category: "Templates", action: PaletteAction::LoadTwoByTwoMatrixTemplate },
        PaletteEntry { icon: "⚡",  label: "Load Design Sprint (5-day Map/Sketch/Decide/Prototype/Test)".into(), category: "Templates", action: PaletteAction::LoadDesignSprintTemplate },
        PaletteEntry { icon: "🎯",  label: "Load Problem/Solution Fit canvas".into(),     category: "Templates", action: PaletteAction::LoadProblemSolutionFitTemplate },
        PaletteEntry { icon: "🎫",  label: "Load Support Ticket Flow (intake → triage → resolve → close)".into(), category: "Templates", action: PaletteAction::LoadSupportTicketFlowTemplate },
        PaletteEntry { icon: "🚨",  label: "Load Incident Response Runbook (SEV-1/2/3 playbook)".into(), category: "Templates", action: PaletteAction::LoadIncidentResponseTemplate },
        PaletteEntry { icon: "📊",  label: "Load Support Escalation Matrix (L1→L4 tiers)".into(),        category: "Templates", action: PaletteAction::LoadSupportEscalationMatrixTemplate },
        PaletteEntry { icon: "🐛",  label: "Load Bug Triage Process (report → fix → verify)".into(),     category: "Templates", action: PaletteAction::LoadBugTriageTemplate },
        PaletteEntry { icon: "📚",  label: "Load Knowledge Base Structure (categories + lifecycle)".into(), category: "Templates", action: PaletteAction::LoadKnowledgeBaseStructureTemplate },
        PaletteEntry { icon: "📣",  label: "Load Voice of Customer (signals → themes → actions)".into(), category: "Templates", action: PaletteAction::LoadVoiceOfCustomerTemplate },
        PaletteEntry { icon: "🚀",  label: "Load Customer Onboarding Journey (sign-up → aha moment → health)".into(), category: "Templates", action: PaletteAction::LoadCustomerOnboardingTemplate },
        PaletteEntry { icon: "📈",  label: "Load Support Health Dashboard (volume · SLA · CSAT · escalations)".into(), category: "Templates", action: PaletteAction::LoadSupportHealthDashboardTemplate },
        PaletteEntry { icon: "📋",  label: "Load Incident Postmortem (blameless retro: timeline · root cause · action items)".into(), category: "Templates", action: PaletteAction::LoadPostmortemTemplate },
        PaletteEntry { icon: "⏱",  label: "Load Support SLA Matrix (P1/P2/P3/P4 response · resolve · escalation targets)".into(), category: "Templates", action: PaletteAction::LoadSupportSLAMatrixTemplate },
        PaletteEntry { icon: "📋",  label: "Load Support Queue kanban (Intake → Triage → In Progress → Resolved)".into(), category: "Templates", action: PaletteAction::LoadSupportQueueTemplate },
        // Search
        PaletteEntry { icon: "🔍", label: "Search nodes".into(),              category: "Search",  action: PaletteAction::OpenSearch },
        PaletteEntry { icon: "⇄",  label: "Find & Replace".into(),            category: "Search",  action: PaletteAction::OpenFindReplace },
        // Export
        PaletteEntry { icon: "⎘",  label: "Copy as Mermaid to clipboard".into(), category: "Export", action: PaletteAction::ExportMermaid },
    ]
}
