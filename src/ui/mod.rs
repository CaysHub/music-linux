//! UI 布局组装：顶部信息栏 / 中部标签页 / 底部控制栏 / 错误提示。

mod controls;
mod info_bar;
mod lyrics_view;
mod playlist_view;

use eframe::egui;

use crate::app::{MainTab, MusicApp};

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();

    egui::Panel::top("info_bar")
        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 8)))
        .show_separator_line(false)
        .show(ui, |ui| info_bar::draw(app, ui));

    egui::Panel::bottom("controls")
        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 8)))
        .show_separator_line(false)
        .show(ui, |ui| controls::draw(app, ui, &ctx));

    // 错误提示条
    if app.error.is_some() {
        egui::Panel::bottom("error_bar")
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(120, 40, 40))
                    .inner_margin(egui::Margin::symmetric(12, 4)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(app.error.clone().unwrap_or_default())
                            .color(egui::Color32::WHITE),
                    );
                    if ui.small_button("✕").clicked() {
                        app.error = None;
                    }
                });
            });
    }

    egui::CentralPanel::default().show(ui, |ui| {
        // 标签页切换
        ui.horizontal(|ui| {
            ui.selectable_value(&mut app.tab, MainTab::Playlist, "播放列表");
            ui.selectable_value(&mut app.tab, MainTab::Lyrics, "歌词");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let icon = if app.dark_mode { "☀ 浅色" } else { "🌙 深色" };
                if ui.small_button(icon).clicked() {
                    app.dark_mode = !app.dark_mode;
                    let visuals = if app.dark_mode {
                        egui::Visuals::dark()
                    } else {
                        egui::Visuals::light()
                    };
                    ctx.set_visuals(visuals);
                }
            });
        });
        ui.separator();

        match app.tab {
            MainTab::Playlist => playlist_view::draw(app, ui, &ctx),
            MainTab::Lyrics => lyrics_view::draw(app, ui, &ctx),
        }
    });
}

/// mm:ss（超过一小时则 h:mm:ss）
pub fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}
