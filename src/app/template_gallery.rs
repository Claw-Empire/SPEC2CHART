// template_gallery.rs — Full-canvas overlay for browsing and loading diagram templates.
//
// Opens with Cmd+N. Categories are displayed as sections. Clicking a template
// returns its HRF content; the update() loop in mod.rs parses and loads it.

use crate::templates::{Template, TEMPLATES};

/// Returned by `draw_template_gallery` to indicate what the user selected.
#[derive(Debug)]
pub(crate) enum GallerySelection {
    Template(String),                   // HRF content from a built-in template
    RecentFile(std::path::PathBuf),     // path chosen from recents
    EmptyCanvas,                        // "New empty canvas" button
}

impl super::FlowchartApp {
    /// Draw the template gallery overlay.
    ///
    /// Returns `Some(GallerySelection)` when the user makes a choice. Returns
    /// `None` every frame while the gallery is closed or while the user is still
    /// browsing.
    pub(crate) fn draw_template_gallery(&mut self, ctx: &egui::Context) -> Option<GallerySelection> {
        if !self.show_template_gallery {
            return None;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_template_gallery = false;
            return None;
        }

        let mut selected_content: Option<GallerySelection> = None;

        // Dim the canvas behind the gallery window
        let screen_rect = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("template_gallery_bg"),
        ));
        painter.rect_filled(
            screen_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );

        let mut keep_open = self.show_template_gallery;

        egui::Window::new("New Diagram")
            .id(egui::Id::new("template_gallery"))
            .fixed_size([620.0, 460.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .title_bar(true)
            .show(ctx, |ui| {
                // Close row
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Choose a starting template or begin with a blank canvas.")
                            .size(12.0)
                            .color(self.theme.text_secondary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕ Close").clicked() {
                            keep_open = false;
                        }
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Recent files section — shown only when there are recents
                        if !self.recent_files.is_empty() {
                            ui.heading("Recent");
                            ui.add_space(4.0);
                            let mut to_remove: Vec<std::path::PathBuf> = Vec::new();
                            for path in &self.recent_files {
                                let fname = path.file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                                let full = {
                                    let s = path.to_string_lossy();
                                    if s.len() > 40 { format!("…{}", &s[s.len()-39..]) } else { s.into_owned() }
                                };
                                let exists = path.exists();
                                let label = if exists {
                                    egui::RichText::new(format!("{fname}\n{full}")).size(12.0)
                                } else {
                                    egui::RichText::new(format!("{fname} (not found)\n{full}"))
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(200, 100, 60))
                                };
                                let btn = egui::Button::new(label).min_size([280.0, 44.0].into());
                                if ui.add(btn).clicked() {
                                    if exists {
                                        selected_content = Some(GallerySelection::RecentFile(path.clone()));
                                        keep_open = false;
                                    } else {
                                        to_remove.push(path.clone());
                                    }
                                }
                            }
                            // Deferred removal: avoid mutating recent_files inside the closure
                            if !to_remove.is_empty() {
                                self.recent_files.retain(|p| !to_remove.contains(p));
                                super::save_recent_files(&self.recent_files);
                            }
                            ui.add_space(16.0);
                        }

                        // Blank canvas option at the top
                        ui.heading("Blank");
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let btn = egui::Button::new(
                                egui::RichText::new("Empty Canvas").size(13.0),
                            )
                            .min_size([140.0, 60.0].into());
                            if ui.add(btn).on_hover_text("Start with a blank canvas").clicked() {
                                selected_content = Some(GallerySelection::EmptyCanvas);
                                keep_open = false;
                            }
                        });
                        ui.add_space(16.0);

                        // Grouped template categories — derived from TEMPLATES to stay in sync
                        let mut seen = std::collections::HashSet::new();
                        let categories: Vec<&str> = TEMPLATES.iter()
                            .map(|t| t.category)
                            .filter(|c| seen.insert(*c))
                            .collect();
                        for category in &categories {
                            let templates_in_cat: Vec<&Template> = TEMPLATES
                                .iter()
                                .filter(|t| t.category == *category)
                                .collect();
                            if templates_in_cat.is_empty() {
                                continue;
                            }

                            ui.heading(*category);
                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                                for template in &templates_in_cat {
                                    ui.vertical(|ui| {
                                        ui.set_width(150.0);
                                        let btn = egui::Button::new(
                                            egui::RichText::new(template.name).size(12.5).strong(),
                                        )
                                        .min_size([140.0, 56.0].into());
                                        if ui
                                            .add(btn)
                                            .on_hover_text(template.description)
                                            .clicked()
                                        {
                                            selected_content = Some(GallerySelection::Template(template.content.to_string()));
                                            keep_open = false;
                                        }
                                        ui.label(
                                            egui::RichText::new(template.description)
                                                .small()
                                                .color(egui::Color32::GRAY),
                                        );
                                    });
                                }
                            });
                            ui.add_space(16.0);
                        }
                    });
            });

        self.show_template_gallery = keep_open;
        selected_content
    }
}
