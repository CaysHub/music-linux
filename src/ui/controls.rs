//! 底部控制栏：单行紧凑布局
//! [⏮][▶][⏹][⏭] [模式] | 时间 ──进度滑块── 时间 | 🔊 音量
//!
//! 注意：egui 的 Slider 用 `spacing().slider_width`（默认 100px）决定自身宽度，
//! 无视 add_sized 分配的宽度——必须先设置 `ui.spacing_mut().slider_width`。

use eframe::egui;
use egui_material_icons::icons::*;

use crate::app::MusicApp;
use crate::audio::PlaybackState;
use crate::playlist::PlayMode;

/// 时间标签固定宽度（等宽字体 h:mm:ss）
const TIME_W: f32 = 52.0;
/// 音量滑块宽度
const VOL_W: f32 = 70.0;
/// 进度条之后所有内容的预留宽度（右侧时间 + 音量图标 + 音量条 + 间距 + 余量）
const TAIL_W: f32 = 200.0;

fn mode_icon(mode: PlayMode) -> egui_material_icons::MaterialIcon {
    match mode {
        PlayMode::Sequential => ICON_ARROW_FORWARD,
        PlayMode::RepeatAll => ICON_REPEAT,
        PlayMode::RepeatOne => ICON_REPEAT_ONE,
        PlayMode::Shuffle => ICON_SHUFFLE,
    }
}

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

        // ---------- 传输按钮（Material Symbols 图标） ----------
        let icon_btn = |ui: &mut egui::Ui,
                        icon: egui_material_icons::MaterialIcon,
                        size: f32,
                        tip: &str| {
            ui.add(egui::Button::new(icon.rich_text().size(size)))
                .on_hover_text(tip)
        };
        if icon_btn(ui, ICON_SKIP_PREVIOUS, 20.0, "上一曲").clicked() {
            app.prev(ctx);
        }
        let play_icon = if playing { ICON_PAUSE } else { ICON_PLAY_ARROW };
        if icon_btn(ui, play_icon, 24.0, if playing { "暂停" } else { "播放" }).clicked() {
            if paused {
                if let Some(engine) = app.engine.as_mut() {
                    engine.resume();
                }
            } else {
                app.toggle_play(ctx);
            }
        }
        if icon_btn(ui, ICON_STOP, 20.0, "停止").clicked() {
            app.stop_all();
        }
        if icon_btn(ui, ICON_SKIP_NEXT, 20.0, "下一曲").clicked() {
            app.next(true, ctx);
        }

        // ---------- 播放模式 ----------
        ui.add_space(4.0);
        let mode = app.playlist.mode;
        let mode_btn = egui::Button::new(
            egui::RichText::new(format!("{} {}", mode_icon(mode).codepoint, mode.label()))
                .size(13.0),
        );
        if ui
            .add(mode_btn)
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

        let time_label = |ui: &mut egui::Ui, text: String, align: egui::Align| {
            ui.add_sized(
                [TIME_W, 18.0],
                egui::Label::new(egui::RichText::new(text).monospace().weak()).halign(align),
            )
        };

        if duration <= 0.0 {
            time_label(ui, "--:--".into(), egui::Align::LEFT);
        } else {
            let pos_text = super::format_duration(std::time::Duration::from_secs_f64(pos.max(0.0)));
            let dur_text = super::format_duration(std::time::Duration::from_secs_f64(duration));
            // 当前时间右对齐、总时长左对齐，紧贴进度条两端
            time_label(ui, pos_text, egui::Align::RIGHT);

            let mut value = pos;
            let slider_w = (ui.available_width() - TAIL_W).max(60.0);
            ui.spacing_mut().slider_width = slider_w;
            let resp = ui.add(egui::Slider::new(&mut value, 0.0..=duration).show_value(false));
            if resp.dragged() {
                app.seek_drag = Some(value);
            }
            if resp.drag_stopped() {
                app.seek(std::time::Duration::from_secs_f64(value.max(0.0)));
                app.seek_drag = None;
            }
            time_label(ui, dur_text, egui::Align::LEFT);
        }

        // ---------- 右侧：音量 ----------
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut vol = app.volume;
            ui.spacing_mut().slider_width = VOL_W;
            let resp = ui.add(egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false));
            if resp.changed() {
                app.set_volume(vol);
            }
            let vol_icon = if app.volume <= 0.0 {
                ICON_VOLUME_OFF
            } else {
                ICON_VOLUME_UP
            };
            ui.add(egui::Label::new(vol_icon.rich_text().size(17.0)).selectable(false));
        });
    });
}
