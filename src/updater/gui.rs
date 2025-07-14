use crate::updater;
use crate::{gui, locale};
use eframe::egui;
use egui_commonmark::*;
use fluent_bundle::{FluentBundle, FluentResource};
use std::sync::Arc;

struct App {
    locale: FluentBundle<Arc<FluentResource>>,
    cache: CommonMarkCache,
    url: String,
    json: updater::Release,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, json: updater::Release, url: String) -> Self {
        gui::gui_setup(cc);

        // Return self
        Self {
            cache: CommonMarkCache::default(),
            locale: locale::get_locale(None),
            url,
            json,
        }
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(locale::get_message(&self.locale, "new-updates", None));
            ui.label(locale::get_message(&self.locale, "update-changelog", None));

            ui.separator();

            ui.heading(&self.json.name);

            egui::ScrollArea::vertical().show(ui, |ui| {
                CommonMarkViewer::new().max_image_width(Some(512)).show(
                    ui,
                    &mut self.cache,
                    &self.json.body,
                );
            });
        });
        egui::TopBottomPanel::bottom("buttons").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(locale::get_message(
                    &self.locale,
                    "download-update-question",
                    None,
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui
                        .button(locale::get_message(&self.locale, "button-no", None))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui
                        .button(locale::get_message(&self.locale, "button-yes", None))
                        .clicked()
                    {
                        let tag_name = if self.json.tag_name.contains("dev-build") {
                            Some(self.json.tag_name.as_str())
                        } else {
                            None
                        };

                        updater::download_update(&self.url, tag_name);
                        updater::run_install_script(true);
                    }
                })
            });
        });
    }
}

pub fn run_gui(json: updater::Release, url: String) -> eframe::Result {
    eframe::run_native(
        &format!("RoExtract Updater v{VERSION}"),
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            cc.egui_ctx.style_mut(|style| {
                // Show the url of a hyperlink on hover
                style.url_in_tooltip = true;
            });

            Ok(Box::new(App::new(cc, json, url)))
        }),
    )
}
