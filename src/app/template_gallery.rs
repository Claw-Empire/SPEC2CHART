// template_gallery.rs — Full-canvas overlay for browsing and loading diagram templates.
//
// Opens with Cmd+N. Categories are displayed as sections. Clicking a template
// returns its HRF content; the update() loop in mod.rs parses and loads it.

use crate::templates::{Template, TEMPLATES};

impl super::FlowchartApp {
    /// Draw the template gallery overlay.
    ///
    /// Returns `Some(content)` when the user selects a template (content is the
    /// raw HRF string, or empty for "Empty Canvas"). Returns `None` every frame
    /// while the gallery is closed or while the user is still browsing.
    pub(crate) fn draw_template_gallery(&mut self, ctx: &egui::Context) -> Option<String> {
        if !self.show_template_gallery {
            return None;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_template_gallery = false;
            return None;
        }

        let mut selected_content: Option<String> = None;

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
                        // Blank canvas option at the top
                        ui.heading("Blank");
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let btn = egui::Button::new(
                                egui::RichText::new("Empty Canvas").size(13.0),
                            )
                            .min_size([140.0, 60.0].into());
                            if ui.add(btn).on_hover_text("Start with a blank canvas").clicked() {
                                selected_content = Some(String::new());
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
                                            selected_content = Some(template.content.to_string());
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
