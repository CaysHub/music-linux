//! 底部控制栏：传输按钮 / 播放模式 / 进度条 / 音量 / 文件与列表操作

use eframe::egui;

use crate::app::MusicApp;
use crate::audio::PlaybackState;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        // ---------- 传输按钮 ----------
        let playing = app
            .engine
            .as_ref()
            .is_some_and(|e| e.state == PlaybackState::Playing);
        let paused = app
            .engine
            .as_ref()
            .is_some_and(|e| e.state == PlaybackState::Paused);

        if ui
            .button("⏮")
            .on_hover_text("上一曲")
            .clicked()
        {
            app.prev(ctx);
        }
        let play_icon = if playing { "⏸" } else { "▶" };
        if ui
            .add(
                egui::Button::new(egui::RichText::new(play_icon).size(18.0))
                    .min_size(egui::vec2(36.0, 28.0)),
            )
            .on_hover_text(if playing { "暂停" } else { "播放" })
            .clicked()
        {
            if paused {
                if let Some(engine) = app.engine.as_mut() {
                    engine.resume();
                }
            } else {
                app.toggle_play(ctx);
            }
        }
        if ui.button("⏹").on_hover_text("停止").clicked() {
            app.stop_all();
        }
        if ui
            .button("⏭")
            .on_hover_text("下一曲")
            .clicked()
        {
            app.next(true, ctx);
        }

        // ---------- 播放模式 ----------
        let mode = app.playlist.mode;
        if ui
            .button(format!("{} {}", mode.icon(), mode.label()))
            .on_hover_text("点击切换播放模式")
            .clicked()
        {
            app.cycle_play_mode();
        }

        ui.separator();

        // ---------- 进度条 + 时间 ----------
        let duration = app
            .current_track()
            .map(|t| t.duration.as_secs_f64())
            .unwrap_or(0.0);
        let pos = app.display_position_secs().clamp(0.0, duration.max(0.0));

        if duration <= 0.0 {
            ui.weak("--:-- / --:--");
        } else {
            let mut value = pos;
            let slider_width = (ui.available_width() * 0.40).clamp(120.0, 600.0);
            let resp = ui.add_sized(
                [slider_width, ui.spacing().interact_size.y],
                egui::Slider::new(&mut value, 0.0..=duration).show_value(false),
            );
            if resp.dragged() {
                app.seek_drag = Some(value);
            }
            if resp.drag_stopped() {
                app.seek(std::time::Duration::from_secs_f64(value.max(0.0)));
                app.seek_drag = None;
            }
            let pos_text = super::format_duration(std::time::Duration::from_secs_f64(pos.max(0.0)));
            let dur_text = super::format_duration(std::time::Duration::from_secs_f64(duration));
            ui.monospace(format!("{pos_text} / {dur_text}"));
        }

        // ---------- 音量 ----------
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            draw_file_buttons(app, ui, ctx);
            ui.separator();
            let mut vol = app.volume;
            let resp = ui.add_sized(
                [80.0, ui.spacing().interact_size.y],
                egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false),
            );
            if resp.changed() {
                app.set_volume(vol);
            }
            ui.label("🔊");
        });
    });
}

/// 添加文件/文件夹、打开/保存列表
fn draw_file_buttons(app: &mut MusicApp, ui: &mut egui::Ui, _ctx: &egui::Context) {
    if ui.button("📁 添加文件夹").clicked() {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            app.add_paths(vec![dir]);
        }
    }
    if ui.button("＋ 添加文件").clicked() {
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
    if ui.button("打开列表").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("播放列表", &["m3u", "m3u8"])
            .pick_file()
        {
            app.load_playlist_file(&path);
        }
    }
    if ui.button("保存列表").clicked() {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("m3u8 播放列表", &["m3u8"])
            .set_file_name("playlist.m3u8")
            .save_file()
        {
            app.save_playlist_file(&path);
        }
    }
    if ui.button("清空").clicked() {
        app.clear_playlist();
    }
}
