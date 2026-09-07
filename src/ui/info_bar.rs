//! 顶部信息栏：当前曲目标题 — 艺术家 — 专辑

use eframe::egui;

use crate::app::MusicApp;
use crate::playlist::Track;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui) {
    let track: Option<&Track> = app.current_track();
    ui.horizontal(|ui| {
        ui.label("🎵");
        match track {
            Some(t) => {
                ui.label(egui::RichText::new(&t.title).strong().size(16.0));
                if !t.artist.is_empty() {
                    ui.separator();
                    ui.weak(&t.artist);
                }
                if !t.album.is_empty() {
                    ui.separator();
                    ui.weak(format!("《{}》", t.album));
                }
            }
            None => {
                ui.weak(egui::RichText::new("未播放").size(16.0));
            }
        }
    });
}
