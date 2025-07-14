use crate::gui::settings;
use crate::{config, gui, locale};
use eframe::egui;
use fluent_bundle::{FluentBundle, FluentResource};
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION"); // Get version for use in the filename

pub struct MyApp {
    first_frame: bool,
    locale: FluentBundle<Arc<FluentResource>>,
}

impl MyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        gui::gui_setup(cc);
        Default::default()
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.heading(locale::get_message(&self.locale, "welcome", None));

                if settings::language(ui, &self.locale) {
                    // This returns true if the locales need to be refreshed
                    self.locale = locale::get_locale(None);
                }
                settings::behavior(ui, &self.locale);
                settings::updates(ui, &self.locale);
                if self.first_frame {
                    config::set_config_value("welcomed", false.into());
                    self.first_frame = false
                }

                if ui
                    .button(locale::get_message(&self.locale, "button-finish", None))
                    .clicked()
                {
                    config::set_config_value("welcomed", true.into());
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            first_frame: true,
            locale: locale::get_locale(None),
        }
    }
}
pub fn run_gui() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_icon(
            eframe::icon_data::from_png_bytes(&include_bytes!("../../assets/icon.png")[..])
                .expect("Failed to load icon"),
        ),
        ..Default::default()
    };

    eframe::run_native(
        &format!("RoExtract v{VERSION}").to_owned(),
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}
