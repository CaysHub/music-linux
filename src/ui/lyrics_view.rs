//! 歌词视图：当前行高亮放大、自动滚动居中（用户手动滚动后暂停跟随）、点击行跳转播放

use eframe::egui;
use egui_material_icons::icons::*;

use crate::app::MusicApp;

const FOLLOW_RESUME: std::time::Duration = std::time::Duration::from_secs(3);
const UPCOMING_FADE_SECS: f32 = 8.0;
const RECENT_FADE_SECS: f32 = 6.0;
const LAST_LINE_FALLBACK_SECS: f32 = 4.0;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    if app.lyrics_editor.is_some() {
        draw_editor(app, ui, ctx);
    } else {
        draw_lyrics(app, ui, ctx);
    }
}

fn draw_editor(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(editor) = app.lyrics_editor.as_ref() else {
        return;
    };
    let title = editor.track_title.clone();
    let is_dirty = editor.is_dirty();
    let timeline_count = crate::lyrics::parse_str(&editor.text)
        .map(|lyrics| lyrics.lines.len())
        .unwrap_or(0);
    let position = app.display_position_secs();
    let mut cancel_requested = false;
    let mut save_requested = ctx.input_mut(|input| {
        input.consume_shortcut(&egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND,
            egui::Key::S,
        ))
    });
    let mut insert_time_requested = false;

    ui.horizontal(|ui| {
        let actions_width = 176.0;
        let title_width = (ui.available_width() - actions_width).max(80.0);
        ui.allocate_ui_with_layout(
            egui::vec2(title_width, 32.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(egui::RichText::new(ICON_EDIT_NOTE).size(22.0));
                ui.add(
                    egui::Label::new(egui::RichText::new(format!("编辑歌词 · {title}")).strong())
                        .truncate(),
                );
                if is_dirty {
                    ui.label(egui::RichText::new("未保存").small().weak());
                }
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    timeline_count > 0,
                    egui::Button::new(format!("{} 保存", ICON_SAVE.codepoint))
                        .min_size(egui::vec2(82.0, 32.0)),
                )
                .on_hover_text("保存为 UTF-8 编码的同名 LRC 文件（Ctrl+S）")
                .clicked()
            {
                save_requested = true;
            }
            if ui
                .add(
                    egui::Button::new(format!("{} 取消", ICON_CLOSE.codepoint))
                        .min_size(egui::vec2(82.0, 32.0)),
                )
                .clicked()
            {
                cancel_requested = true;
            }
        });
    });

    ui.horizontal(|ui| {
        let insert_width = 132.0;
        let status_width = (ui.available_width() - insert_width).max(80.0);
        ui.allocate_ui_with_layout(
            egui::vec2(status_width, 28.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let status = if timeline_count > 0 {
                    format!("已识别 {timeline_count} 行时间轴")
                } else {
                    "未检测到有效时间轴".to_string()
                };
                let color = if timeline_count > 0 {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().warn_fg_color
                };
                ui.label(egui::RichText::new(status).small().color(color));
            },
        );
        if ui
            .add(
                egui::Button::new(format!("{} 插入当前时间", ICON_SCHEDULE.codepoint))
                    .min_size(egui::vec2(insert_width, 28.0)),
            )
            .on_hover_text("在文本末尾插入当前播放位置的时间标签")
            .clicked()
        {
            insert_time_requested = true;
        }
    });

    ui.separator();

    if insert_time_requested {
        if let Some(editor) = app.lyrics_editor.as_mut() {
            append_timestamp(&mut editor.text, position);
            editor.error = None;
        }
    }

    let editor_error = app
        .lyrics_editor
        .as_ref()
        .and_then(|editor| editor.error.clone());
    if let Some(error) = editor_error {
        ui.label(egui::RichText::new(error).color(ui.visuals().error_fg_color));
    }

    if let Some(editor) = app.lyrics_editor.as_mut() {
        let response = ui.add_sized(
            [ui.available_width(), ui.available_height()],
            egui::TextEdit::multiline(&mut editor.text)
                .code_editor()
                .desired_width(f32::INFINITY)
                .hint_text("[00:00.00]歌词"),
        );
        if response.changed() {
            editor.error = None;
        }
    }

    if cancel_requested {
        app.cancel_lyrics_edit();
    } else if save_requested {
        let is_valid = app
            .lyrics_editor
            .as_ref()
            .is_some_and(|editor| crate::lyrics::parse_str(&editor.text).is_some());
        if !is_valid {
            if let Some(editor) = app.lyrics_editor.as_mut() {
                editor.error = Some("未检测到有效时间轴，请使用 [mm:ss.xx]歌词 格式".into());
            }
        } else if let Err(error) = app.save_lyrics_edit() {
            if let Some(editor) = app.lyrics_editor.as_mut() {
                editor.error = Some(error);
            }
        }
    }
}

fn append_timestamp(text: &mut String, seconds: f64) {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format_lrc_timestamp(seconds));
}

fn format_lrc_timestamp(seconds: f64) -> String {
    let centiseconds = (seconds.max(0.0) * 100.0).round() as u64;
    let minutes = centiseconds / 6_000;
    let seconds = (centiseconds / 100) % 60;
    let fraction = centiseconds % 100;
    format!("[{minutes:02}:{seconds:02}.{fraction:02}]")
}

fn draw_lyrics(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(lyrics) = app.lyrics.clone() else {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.4);
            ui.label(egui::RichText::new("无歌词").size(16.0).weak());
            ui.label(egui::RichText::new("可通过标签栏新建歌词").weak());
        });
        return;
    };
    if lyrics.lines.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.4);
            ui.label(egui::RichText::new("无歌词").size(16.0).weak());
        });
        return;
    }

    let pos = app
        .engine
        .as_ref()
        .map(|e| e.position())
        .unwrap_or_default();
    let active = lyrics.active_line(pos);
    let duration = app.current_track().map(|t| t.duration);

    // 用户手动滚动检测（鼠标滚轮）：暂停自动跟随一段时间
    let user_scrolled = ui.ctx().input(|i| {
        i.events
            .iter()
            .any(|ev| matches!(ev, egui::Event::MouseWheel { .. }))
    });
    if user_scrolled && ui.rect_contains_pointer(ui.max_rect()) {
        app.lyrics_user_scroll = Some(std::time::Instant::now());
    }
    let follow = !app
        .lyrics_user_scroll
        .is_some_and(|t| t.elapsed() < FOLLOW_RESUME);

    let mut seek_to: Option<std::time::Duration> = None;

    egui::ScrollArea::vertical()
        .id_salt("lyrics_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(ui.available_height() * 0.35);
            for (i, line) in lyrics.lines.iter().enumerate() {
                let is_active = active == Some(i);
                let text = if line.text.is_empty() {
                    "♪".to_string()
                } else {
                    line.text.clone()
                };
                let color = lyric_color(ui, &lyrics, i, active, pos, duration);
                let rich = if is_active {
                    egui::RichText::new(&text).strong().size(22.0).color(color)
                } else {
                    egui::RichText::new(&text).size(16.0).color(color)
                };
                let line_h = if is_active { 34.0 } else { 28.0 };
                let resp = ui.add_sized(
                    [ui.available_width(), line_h],
                    egui::Label::new(rich)
                        .halign(egui::Align::Center)
                        .selectable(false)
                        .sense(egui::Sense::click()),
                );
                if resp.clicked() {
                    seek_to = Some(line.time);
                }
                if is_active && follow {
                    // 平滑滚动到当前行（仅在 ScrollArea 内部调用有效）
                    resp.scroll_to_me(Some(egui::Align::Center));
                }
            }
            ui.add_space(ui.available_height() * 0.35);
        });

    if let Some(t) = seek_to {
        if let Some(engine) = app.engine.as_ref() {
            engine.seek(t);
        }
        // seek 后立即恢复跟随
        app.lyrics_user_scroll = None;
        ctx.request_repaint();
    }
}

fn lyric_color(
    ui: &egui::Ui,
    lyrics: &crate::lyrics::Lyrics,
    i: usize,
    active: Option<usize>,
    pos: std::time::Duration,
    track_duration: Option<std::time::Duration>,
) -> egui::Color32 {
    let accent = ui.visuals().selection.bg_fill;
    let text = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let weak = ui.visuals().weak_text_color();
    let warm = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(244, 203, 112)
    } else {
        egui::Color32::from_rgb(190, 122, 31)
    };

    let Some(line) = lyrics.lines.get(i) else {
        return weak;
    };

    if active == Some(i) {
        let end = lyrics
            .lines
            .get(i + 1)
            .map(|next| next.time)
            .or(track_duration)
            .unwrap_or_else(|| {
                line.time + std::time::Duration::from_secs_f32(LAST_LINE_FALLBACK_SECS)
            });
        let total = duration_secs_between(line.time, end).max(0.35);
        let remaining = duration_secs_between(pos, end).clamp(0.0, total);
        let progress = 1.0 - remaining / total;
        return mix_color(accent, warm, progress);
    }

    if line.time > pos {
        let remaining = duration_secs_between(pos, line.time);
        let near = (1.0 - remaining / UPCOMING_FADE_SECS).clamp(0.0, 1.0);
        return mix_color(weak, mix_color(accent, text, 0.20), near * 0.72);
    }

    let elapsed = duration_secs_between(line.time, pos);
    let recent = (1.0 - elapsed / RECENT_FADE_SECS).clamp(0.0, 1.0);
    mix_color(weak, mix_color(accent, text, 0.50), recent * 0.28)
}

fn duration_secs_between(a: std::time::Duration, b: std::time::Duration) -> f32 {
    b.saturating_sub(a).as_secs_f32()
}

fn mix_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_lrc_timestamp() {
        assert_eq!(format_lrc_timestamp(0.0), "[00:00.00]");
        assert_eq!(format_lrc_timestamp(65.439), "[01:05.44]");
        assert_eq!(format_lrc_timestamp(6_005.0), "[100:05.00]");
    }

    #[test]
    fn appends_timestamp_on_a_new_line() {
        let mut text = "[ti:Song]".to_string();
        append_timestamp(&mut text, 1.25);
        assert_eq!(text, "[ti:Song]\n[00:01.25]");
    }
}
