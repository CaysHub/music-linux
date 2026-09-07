//! 底部控制栏：单行紧凑布局
//! [⏮][▶][⏹][⏭] [模式] | 时间 ──进度滑块── 时间 | 🔊 音量

use eframe::egui;

use crate::app::MusicApp;
use crate::audio::PlaybackState;

/// 时间标签固定宽度（等宽字体 h:mm:ss）
const TIME_W: f32 = 52.0;
/// 音量滑块宽度
const VOL_W: f32 = 70.0;
/// 进度条之后所有内容的预留宽度（右侧时间 + 音量图标 + 音量条 + 间距 + 余量）。
/// 留足余量，确保右侧区组内容不会与进度条重叠。
const TAIL_W: f32 = 200.0;
/// 滑块/标签高度
const ROW_H: f32 = 20.0;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        let playing = app
            .engine
            .as_ref()
            .is_some_and(|e| e.state == PlaybackState::Playing);
        let paused = app
            .engine
            .as_ref()
            .is_some_and(|e| e.state == PlaybackState::Paused);

        // ---------- 传输按钮（紧凑统一尺寸） ----------
        if ui
            .button(egui::RichText::new("⏮").size(15.0))
            .on_hover_text("上一曲")
            .clicked()
        {
            app.prev(ctx);
        }
        let play_icon = if playing { "⏸" } else { "▶" };
        if ui
            .button(egui::RichText::new(play_icon).size(16.0))
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
        if ui
            .button(egui::RichText::new("⏹").size(15.0))
            .on_hover_text("停止")
            .clicked()
        {
            app.stop_all();
        }
        if ui
            .button(egui::RichText::new("⏭").size(15.0))
            .on_hover_text("下一曲")
            .clicked()
        {
            app.next(true, ctx);
        }

        // ---------- 播放模式 ----------
        ui.add_space(4.0);
        let mode = app.playlist.mode;
        if ui
            .button(egui::RichText::new(format!("{} {}", mode.icon(), mode.label())).size(13.0))
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

        let time_label = |ui: &mut egui::Ui, text: String| {
            ui.add_sized(
                [TIME_W, ROW_H],
                egui::Label::new(egui::RichText::new(text).monospace().weak())
                    .halign(egui::Align::Center),
            )
        };

        if duration <= 0.0 {
            time_label(ui, "--:--".into());
        } else {
            let pos_text = super::format_duration(std::time::Duration::from_secs_f64(pos.max(0.0)));
            let dur_text = super::format_duration(std::time::Duration::from_secs_f64(duration));
            time_label(ui, pos_text);

            let mut value = pos;
            let slider_w = (ui.available_width() - TAIL_W).max(60.0);
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
        }

        // ---------- 右侧：音量 ----------
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut vol = app.volume;
            let resp = ui.add_sized(
                [VOL_W, ROW_H],
                egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false),
            );
            if resp.changed() {
                app.set_volume(vol);
            }
            ui.label("🔊");
        });
    });
}
