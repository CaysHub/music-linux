//! 顶部信息栏：当前曲目信息（左） + 文件操作工具栏（右）

use eframe::egui;
use egui_material_icons::icons::*;

use crate::app::MusicApp;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // ---------- 左：当前曲目 ----------
        ui.add(egui::Label::new(
            ICON_MUSIC_NOTE.rich_text().size(15.0).strong(),
        ));
        match app.current_track() {
            Some(t) => {
                ui.add(
                    egui::Label::new(egui::RichText::new(&t.title).strong().size(15.0))
                        .truncate(),
                );
                if !t.artist.is_empty() {
                    ui.weak(&t.artist);
                }
                if !t.album.is_empty() {
                    ui.weak(format!("《{}》", t.album));
                }
            }
            None => {
                ui.weak(egui::RichText::new("未播放").size(15.0));
            }
        }

        // ---------- 右：工具栏 ----------
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(format!("{} 添加文件", ICON_ADD.codepoint))
                .on_hover_text("添加音频文件")
                .clicked()
            {
                let files = rfd::FileDialog::new()
                    .add_filter(
                        "音频文件",
                        &["mp3", "flac", "ogg", "oga", "opus", "wav", "m4a", "aac", "aiff"],
                    )
                    .pick_files();
                if let Some(files) = files {
                    app.add_paths(files);
                }
            }
            if ui
                .button(format!("{} 添加文件夹", ICON_FOLDER_OPEN.codepoint))
                .on_hover_text("递归添加文件夹内音频")
                .clicked()
            {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    app.add_paths(vec![dir]);
                }
            }
            ui.separator();
            if ui
                .button(format!("{} 打开列表", ICON_FILE_OPEN.codepoint))
                .on_hover_text("加载 m3u/m3u8 播放列表")
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("播放列表", &["m3u", "m3u8"])
                    .pick_file()
                {
                    app.load_playlist_file(&path);
                }
            }
            if ui
                .button(format!("{} 保存列表", ICON_SAVE.codepoint))
                .on_hover_text("保存为 m3u8")
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("m3u8 播放列表", &["m3u8"])
                    .set_file_name("playlist.m3u8")
                    .save_file()
                {
                    app.save_playlist_file(&path);
                }
            }
        });
    });
}
