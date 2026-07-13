use crate::locale;
use crate::updater;
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use fluent_bundle::{FluentBundle, FluentResource};
use std::sync::Arc;

/// In-app update prompt shown inside the main window. The check runs in the
/// background so the window can open immediately.
pub struct UpdatePrompt {
    cache: CommonMarkCache,
    locale: FluentBundle<Arc<FluentResource>>,
    release: updater::Release,
    url: String,
}

impl UpdatePrompt {
    pub fn new(release: updater::Release, url: String) -> Self {
        Self {
            cache: CommonMarkCache::default(),
            locale: locale::get_locale(None),
            release,
            url,
        }
    }

    /// Render the prompt. Returns `true` when it should be dismissed (the user
    /// declined or closed it).
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut dismiss = false;

        egui::Window::new(locale::get_message(&self.locale, "new-updates", None))
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                // Show a spinner while the binary downloads.
                if updater::is_downloading() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(locale::get_message(&self.locale, "downloading-update", None));
                    });
                    return;
                }

                ui.label(locale::get_message(&self.locale, "update-changelog", None));
                ui.separator();
                ui.heading(&self.release.name);

                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    CommonMarkViewer::new().max_image_width(Some(512)).show(
                        ui,
                        &mut self.cache,
                        &self.release.body,
                    );
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(locale::get_message(
                        &self.locale,
                        "download-update-question",
                        None,
                    ));
                    if ui
                        .button(locale::get_message(&self.locale, "button-yes", None))
                        .clicked()
                    {
                        let tag_name = if self.release.tag_name.contains("dev-build") {
                            Some(self.release.tag_name.clone())
                        } else {
                            None
                        };
                        updater::download_and_install(ui.ctx().clone(), self.url.clone(), tag_name);
                    }
                    if ui
                        .button(locale::get_message(&self.locale, "button-no", None))
                        .clicked()
                    {
                        dismiss = true;
                    }
                });
            });

        dismiss
    }
}
