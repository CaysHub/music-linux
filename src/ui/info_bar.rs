//! 顶部信息栏：当前曲目信息（左） + 文件操作和主题工具栏（右）

use eframe::egui;
use egui_material_icons::icons::*;

use crate::app::MusicApp;
use crate::audio::PlaybackState;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let playing = app
        .engine
        .as_ref()
        .is_some_and(|engine| engine.state == PlaybackState::Playing);
    ui.horizontal(|ui| {
        // ---------- 左：当前曲目 ----------
        ui.add(egui::Label::new(
            ICON_MUSIC_NOTE.rich_text().size(18.0).strong(),
        ));
        match app.current_track() {
            Some(t) => {
                ui.add(
                    egui::Label::new(egui::RichText::new(&t.title).strong().size(16.0)).truncate(),
                );
                let (equalizer_rect, _) =
                    ui.allocate_exact_size(egui::vec2(20.0, 16.0), egui::Sense::hover());
                super::draw_equalizer(ui, equalizer_rect, ui.visuals().selection.bg_fill, playing);
                if !t.artist.is_empty() {
                    ui.weak(&t.artist);
                }
                if !t.album.is_empty() {
                    ui.weak(format!("《{}》", t.album));
                }
            }
            None => {
                ui.weak(egui::RichText::new("未播放").size(16.0));
            }
        }

        // ---------- 右：工具栏 ----------
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(toolbar_button(format!(
                    "{} 置顶歌词",
                    ICON_PUSH_PIN.codepoint
                )))
                .on_hover_text("切换到始终置顶的迷你歌词窗口")
                .clicked()
            {
                app.enter_mini_mode(&ctx);
            }
            let (icon, text) = if app.dark_mode {
                (ICON_LIGHT_MODE, "浅色")
            } else {
                (ICON_DARK_MODE, "深色")
            };
            if ui
                .add(toolbar_button(format!("{} {text}", icon.codepoint)))
                .on_hover_text("切换深色/浅色主题")
                .clicked()
            {
                app.dark_mode = !app.dark_mode;
                super::set_app_theme(&ctx, app.dark_mode);
            }
            if ui
                .add(toolbar_button(format!("{} 添加文件", ICON_ADD.codepoint)))
                .on_hover_text("添加音频文件")
                .clicked()
            {
                let files = rfd::FileDialog::new()
                    .add_filter(
                        "音频文件",
                        &[
                            "mp3", "flac", "ogg", "oga", "opus", "wav", "m4a", "aac", "aiff",
                        ],
                    )
                    .pick_files();
                if let Some(files) = files {
                    app.add_paths(files);
                }
            }
            if ui
                .add(toolbar_button(format!(
                    "{} 添加文件夹",
                    ICON_FOLDER_OPEN.codepoint
                )))
                .on_hover_text("递归添加文件夹内音频")
                .clicked()
            {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    app.add_paths(vec![dir]);
                }
            }
        });
    });
}

fn toolbar_button(text: String) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text).size(14.0))
        .min_size(egui::vec2(92.0, 30.0))
        .corner_radius(egui::CornerRadius::same(6))
}
