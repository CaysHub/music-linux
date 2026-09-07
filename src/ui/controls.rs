//! 底部控制栏：三行布局
//! 行1：传输按钮（大） + 播放模式 ｜ 右侧音量
//! 行2：进度条（时间 — 滑块 — 总时长）
//! 行3：文件操作按钮 ｜ 右侧清空

use eframe::egui;

use crate::app::MusicApp;
use crate::audio::PlaybackState;

/// 统一的传输按钮尺寸
const BTN: f32 = 42.0;
/// 播放按钮（主按钮）尺寸
const BTN_PLAY: f32 = 52.0;
/// 控件行高
const ROW_H: f32 = 24.0;
/// 时间标签固定宽度（等宽字体 h:mm:ss 留足）
const TIME_W: f32 = 64.0;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    draw_transport_row(app, ui, ctx);
    ui.add_space(6.0);
    draw_progress_row(app, ui);
    ui.add_space(6.0);
    draw_utility_row(app, ui);
}

/// 行1：⏮ ▶ ⏹ ⏭  模式 …… 🔊 音量
fn draw_transport_row(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        let playing = app
            .engine
            .as_ref()
            .is_some_and(|e| e.state == PlaybackState::Playing);
        let paused = app
            .engine
            .as_ref()
            .is_some_and(|e| e.state == PlaybackState::Paused);

        // ---------- 传输按钮 ----------
        let icon_btn = |ui: &mut egui::Ui, icon: &str, size: f32, tip: &str| {
            let btn = egui::Button::new(
                egui::RichText::new(icon)
                    .size(size)
                    .color(ui.visuals().widgets.inactive.fg_stroke.color),
            )
            .min_size(egui::vec2(size + 22.0, size + 22.0));
            ui.add(btn).on_hover_text(tip)
        };

        if icon_btn(ui, "⏮", 20.0, "上一曲").clicked() {
            app.prev(ctx);
        }
        let play_icon = if playing { "⏸" } else { "▶" };
        let play_btn = egui::Button::new(
            egui::RichText::new(play_icon)
                .size(26.0)
                .color(ui.visuals().selection.bg_fill),
        )
        .min_size(egui::vec2(BTN_PLAY, BTN_PLAY))
        .corner_radius(egui::CornerRadius::same(BTN_PLAY as u8 / 2));
        if ui
            .add(play_btn)
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
        if icon_btn(ui, "⏹", 20.0, "停止").clicked() {
            app.stop_all();
        }
        if icon_btn(ui, "⏭", 20.0, "下一曲").clicked() {
            app.next(true, ctx);
        }

        ui.add_space(12.0);

        // ---------- 播放模式 ----------
        let mode = app.playlist.mode;
        let mode_btn = egui::Button::new(
            egui::RichText::new(format!("{} {}", mode.icon(), mode.label())).size(14.0),
        )
        .min_size(egui::vec2(0.0, BTN));
        if ui
            .add(mode_btn)
            .on_hover_text("点击切换播放模式")
            .clicked()
        {
            app.cycle_play_mode();
        }

        // ---------- 右侧：音量 ----------
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("🔊").size(16.0));
            let mut vol = app.volume;
            let resp = ui.add_sized(
                [100.0, ROW_H],
                egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false),
            );
            if resp.changed() {
                app.set_volume(vol);
            }
        });
    });
}

/// 行2：位置 — 进度滑块 — 总时长
fn draw_progress_row(app: &mut MusicApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let duration = app
            .current_track()
            .map(|t| t.duration.as_secs_f64())
            .unwrap_or(0.0);
        let pos = app.display_position_secs().clamp(0.0, duration.max(0.0));

        let time_label = |ui: &mut egui::Ui, text: String| {
            ui.add_sized(
                [TIME_W, ROW_H],
                egui::Label::new(egui::RichText::new(text).monospace().weak())
                    .halign(egui::Align::Center),
            )
        };

        if duration <= 0.0 {
            // 无曲目或时长未知：不渲染进度行
            return;
        }

        let pos_text = super::format_duration(std::time::Duration::from_secs_f64(pos.max(0.0)));
        let dur_text = super::format_duration(std::time::Duration::from_secs_f64(duration));
        time_label(ui, pos_text);

        let mut value = pos;
        let slider_w = (ui.available_width() - TIME_W).max(80.0);
        let resp = ui.add_sized(
            [slider_w, ROW_H],
            egui::Slider::new(&mut value, 0.0..=duration).show_value(false),
        );
        if resp.dragged() {
            app.seek_drag = Some(value);
        }
        if resp.drag_stopped() {
            app.seek(std::time::Duration::from_secs_f64(value.max(0.0)));
            app.seek_drag = None;
        }
        time_label(ui, dur_text);
    });
}

/// 行3：文件操作（左） ｜ 清空（右）
fn draw_utility_row(app: &mut MusicApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
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
        if ui.button("📁 添加文件夹").clicked() {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                app.add_paths(vec![dir]);
            }
        }
        ui.separator();
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

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(egui::RichText::new("🗑 清空").color(egui::Color32::from_rgb(220, 90, 90)))
                .on_hover_text("清空播放列表")
                .clicked()
            {
                app.clear_playlist();
            }
        });
    });
}
