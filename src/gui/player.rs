//! Built-in audio player for previewing sounds without leaving the app.

use std::{io::Cursor, time::Duration};

use fluent_bundle::{FluentBundle, FluentResource};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::sync::Arc;

use crate::{
    config, dialog, locale,
    logic::{self, AssetInfo},
};

type AudioDecoder = Decoder<Cursor<Vec<u8>>>;

/// Decode an asset for playback, also returning its length when known.
pub fn decode(asset: &AssetInfo) -> Result<(AudioDecoder, Option<Duration>), String> {
    let bytes = logic::extract_asset_to_bytes(asset.clone()).map_err(|e| e.to_string())?;
    let length = bytes.len() as u64;
    let mut builder = Decoder::builder()
        .with_data(Cursor::new(bytes))
        .with_byte_len(length)
        .with_seekable(true);
    if let Some(format) = asset.format {
        builder = builder.with_hint(format);
    }
    let decoder = builder.build().map_err(|e| e.to_string())?;
    let duration = decoder.total_duration();
    Ok((decoder, duration))
}

pub fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

struct Track {
    asset: AssetInfo,
    duration: Option<Duration>,
}

struct Output {
    // Dropping the stream stops all sound, so it lives as long as the sink.
    stream: OutputStream,
    /// One sink per track; shared so seeking (which waits on the audio thread) can run elsewhere.
    sink: Arc<Sink>,
}

type Loading = dialog::Pending<Result<(AudioDecoder, Option<Duration>), String>>;

#[derive(Default)]
pub struct Player {
    output: Option<Output>,
    track: Option<Track>,
    loading: Option<(AssetInfo, Loading)>,
    /// Position being dragged on the seek bar, applied on release.
    seeking: Option<f32>,
    error: Option<String>,
}

impl Player {
    pub fn is_playing(&self) -> bool {
        self.output
            .as_ref()
            .is_some_and(|output| !output.sink.is_paused() && !output.sink.empty())
    }

    /// The asset loaded or loading in the player.
    pub fn current(&self) -> Option<&AssetInfo> {
        self.loading
            .as_ref()
            .map(|(asset, _)| asset)
            .or(self.track.as_ref().map(|track| &track.asset))
    }

    /// Load `asset` and start playing it.
    pub fn play(&mut self, asset: AssetInfo) {
        self.error = None;
        let to_decode = asset.clone();
        self.loading = Some((asset, dialog::spawn(move || decode(&to_decode))));
    }

    /// True once the loaded track has played to the end.
    fn finished(&self) -> bool {
        self.track.is_some()
            && self
                .output
                .as_ref()
                .is_none_or(|output| output.sink.empty())
    }

    /// Pause or resume, or play `selected` if nothing is playing.
    pub fn toggle(&mut self, selected: Option<AssetInfo>) {
        match &self.output {
            Some(output) if !output.sink.empty() => {
                if output.sink.is_paused() {
                    output.sink.play();
                } else {
                    output.sink.pause();
                }
            }
            _ => {
                if let Some(asset) = selected.or(self.track.as_ref().map(|t| t.asset.clone())) {
                    self.play(asset);
                }
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(output) = &self.output {
            output.sink.stop();
        }
        self.loading = None;
        self.track = None;
    }

    fn position(&self) -> Duration {
        self.output
            .as_ref()
            .map_or(Duration::ZERO, |output| output.sink.get_pos())
    }

    /// Hand a finished load over to the audio output.
    fn poll_loading(&mut self, locale: &FluentBundle<Arc<FluentResource>>) {
        let Some(result) = self
            .loading
            .as_ref()
            .and_then(|(_, pending)| pending.take())
        else {
            return;
        };
        let (asset, _) = self.loading.take().unwrap();
        let (decoder, duration) = match result {
            Ok(decoded) => decoded,
            Err(e) => {
                log_error!("Failed to play {}: {}", asset.name, e);
                self.error = Some(locale::get_message(locale, "player-failed", None));
                self.track = None;
                return;
            }
        };

        if self.output.is_none() {
            match OutputStreamBuilder::open_default_stream() {
                Ok(mut stream) => {
                    stream.log_on_drop(false);
                    let sink = Arc::new(Sink::connect_new(stream.mixer()));
                    self.output = Some(Output { stream, sink });
                }
                Err(e) => {
                    log_error!("Failed to open audio output: {}", e);
                    self.error = Some(locale::get_message(locale, "player-no-device", None));
                    return;
                }
            }
        }

        // A new sink replaces whatever was playing; dropping the old one stops it.
        let output = self.output.as_mut().unwrap();
        output.sink = Arc::new(Sink::connect_new(output.stream.mixer()));
        output.sink.set_volume(volume());
        output.sink.append(decoder);
        self.track = Some(Track { asset, duration });
    }

    /// Whether there is anything to show in the player bar.
    pub fn is_visible(&self) -> bool {
        self.track.is_some() || self.loading.is_some() || self.error.is_some()
    }

    /// The player bar.
    pub fn ui(&mut self, ui: &mut egui::Ui, locale: &FluentBundle<Arc<FluentResource>>) {
        self.poll_loading(locale);

        ui.horizontal(|ui| {
            let playing = self.is_playing();
            let icon = if playing { "⏸" } else { "▶" };
            if ui.button(icon).clicked() {
                if self.finished() {
                    // Play it again from the start
                    let asset = self.track.as_ref().unwrap().asset.clone();
                    self.play(asset);
                } else {
                    self.toggle(None);
                }
            }
            if ui.button("⏹").clicked() {
                self.stop();
                self.error = None; // Also closes the bar
            }

            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
                return;
            }

            let name = self
                .current()
                .map(|asset| config::get_asset_alias(&asset.name))
                .unwrap_or_default();
            let duration = self.track.as_ref().and_then(|track| track.duration);
            let position = self.position();

            // Time, volume and name on the right; the seek bar takes the rest
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut volume = volume();
                let response = ui.add(
                    egui::Slider::new(&mut volume, 0.0..=1.0)
                        .show_value(false)
                        .text("🔊"),
                );
                if response.changed() {
                    config::set_config_value("player_volume", (volume as f64).into());
                    if let Some(output) = &self.output {
                        output.sink.set_volume(volume);
                    }
                }

                let time = match duration {
                    Some(duration) => format!(
                        "{} / {}",
                        format_duration(position.min(duration)),
                        format_duration(duration)
                    ),
                    None => format_duration(position),
                };
                ui.monospace(time);

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(name).truncate());
                    if let Some(duration) = duration.filter(|d| !d.is_zero()) {
                        self.seek_bar(ui, position, duration);
                    }
                });
            });
        });

        if self.is_playing() || self.loading.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(100)); // Move the seek bar
        }
    }

    fn seek_bar(&mut self, ui: &mut egui::Ui, position: Duration, duration: Duration) {
        let mut fraction = self
            .seeking
            .unwrap_or(position.as_secs_f32() / duration.as_secs_f32())
            .clamp(0.0, 1.0);
        ui.spacing_mut().slider_width = (ui.available_width() - 8.0).max(40.0);
        let response = ui.add(egui::Slider::new(&mut fraction, 0.0..=1.0).show_value(false));
        if response.dragged() {
            self.seeking = Some(fraction); // Seek once released
        } else if response.drag_stopped() || response.clicked() || response.changed() {
            if let Some(output) = &self.output {
                let sink = output.sink.clone();
                let target = duration.mul_f32(fraction);
                // try_seek waits for the audio thread, so don't do it here
                std::thread::spawn(move || {
                    if let Err(e) = sink.try_seek(target) {
                        log_warn!("Failed to seek: {}", e);
                    }
                });
            }
            self.seeking = None;
        }
    }
}

fn volume() -> f32 {
    config::get_config()
        .get("player_volume")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.8) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0:00");
        assert_eq!(format_duration(Duration::from_secs(65)), "1:05");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1:02:05");
    }
}
