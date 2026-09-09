//! 始终置顶的双行迷你播放器：紧凑播放控制 + 单行进度歌词。

use std::time::Duration;

use eframe::egui;
use egui_material_icons::icons::*;

use crate::app::MusicApp;
use crate::audio::PlaybackState;
use crate::lyrics::Lyrics;
use crate::playlist::PlayMode;

const ROW_HEIGHT: f32 = 30.0;
const BUTTON_SIZE: egui::Vec2 = egui::vec2(22.0, 20.0);
const VOLUME_WIDTH: f32 = 38.0;
const LYRIC_SIDE_PADDING: f32 = 18.0;
const LYRIC_FONT_SIZE: f32 = 16.0;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let background = super::app_background(app.dark_mode);
    ui.painter()
        .rect_filled(ui.max_rect(), egui::CornerRadius::ZERO, background);

    ui.spacing_mut().item_spacing = egui::vec2(5.0, 0.0);
    ui.spacing_mut().button_padding = egui::vec2(2.0, 2.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| draw_controls(app, ui, ctx),
    );

    let divider_y = ui.min_rect().top() + ROW_HEIGHT;
    ui.painter().hline(
        ui.max_rect().x_range(),
        divider_y,
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
    draw_lyric_line(app, ui, ctx);
}

fn draw_controls(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let drag = drag_handle(ui);
    if drag.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    let playing = app
        .engine
        .as_ref()
        .is_some_and(|engine| engine.state == PlaybackState::Playing);
    let paused = app
        .engine
        .as_ref()
        .is_some_and(|engine| engine.state == PlaybackState::Paused);

    if icon_button(ui, ICON_SKIP_PREVIOUS, 13.0, "上一曲").clicked() {
        app.prev(ctx);
    }
    let play_icon = if playing { ICON_PAUSE } else { ICON_PLAY_ARROW };
    if icon_button(ui, play_icon, 15.0, if playing { "暂停" } else { "播放" }).clicked() {
        if paused {
            if let Some(engine) = app.engine.as_mut() {
                engine.resume();
            }
        } else {
            app.toggle_play(ctx);
        }
    }
    if icon_button(ui, ICON_STOP, 13.0, "停止").clicked() {
        app.stop_all();
    }
    if icon_button(ui, ICON_SKIP_NEXT, 13.0, "下一曲").clicked() {
        app.next(true, ctx);
    }

    let mode = app.playlist.mode;
    if icon_button(ui, mode_icon(mode), 13.0, mode.label()).clicked() {
        app.cycle_play_mode();
    }

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if icon_button(ui, ICON_CLOSE, 13.0, "关闭播放器").clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if icon_button(ui, ICON_OPEN_IN_FULL, 13.0, "恢复主界面").clicked() {
            app.exit_mini_mode(ctx);
        }

        let mut volume = app.volume;
        ui.spacing_mut().slider_width = VOLUME_WIDTH;
        if ui
            .add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false))
            .changed()
        {
            app.set_volume(volume);
        }
        let volume_icon = if app.volume <= 0.0 {
            ICON_VOLUME_OFF
        } else {
            ICON_VOLUME_UP
        };
        static_icon(ui, volume_icon, 13.0);
    });
}

fn icon_button(
    ui: &mut egui::Ui,
    icon: egui_material_icons::MaterialIcon,
    size: f32,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(BUTTON_SIZE, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(4),
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        paint_icon(ui, rect, icon, size, visuals.fg_stroke.color);
    }
    response.on_hover_text(tooltip)
}

fn drag_handle(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(BUTTON_SIZE, egui::Sense::drag());
    if ui.is_rect_visible(rect) {
        let color = ui.style().interact(&response).fg_stroke.color;
        paint_icon(ui, rect, ICON_DRAG_INDICATOR, 13.0, color);
    }
    response.on_hover_text("拖动迷你窗口")
}

fn static_icon(ui: &mut egui::Ui, icon: egui_material_icons::MaterialIcon, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, BUTTON_SIZE.y), egui::Sense::hover());
    paint_icon(ui, rect, icon, size, ui.visuals().text_color());
}

fn paint_icon(
    ui: &egui::Ui,
    rect: egui::Rect,
    icon: egui_material_icons::MaterialIcon,
    size: f32,
    color: egui::Color32,
) {
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.codepoint,
        egui::FontId::new(size, icon.font_family()),
        color,
    );
}

fn mode_icon(mode: PlayMode) -> egui_material_icons::MaterialIcon {
    match mode {
        PlayMode::Sequential => ICON_ARROW_FORWARD,
        PlayMode::RepeatOne => ICON_REPEAT_ONE,
        PlayMode::RepeatAll => ICON_REPEAT,
        PlayMode::Shuffle => ICON_SHUFFLE,
    }
}

fn draw_lyric_line(app: &MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let position = app
        .engine
        .as_ref()
        .map(|engine| engine.position())
        .unwrap_or_default();
    let duration = app.current_track().map(|track| track.duration);
    let lyric = app
        .lyrics
        .as_ref()
        .and_then(|lyrics| lyric_at(lyrics, position, duration));
    let (text, progress) = match lyric {
        Some((text, progress)) => (text.to_string(), progress),
        None => (
            app.current_track()
                .map(|track| track.title.clone())
                .unwrap_or_else(|| "暂无歌词".to_string()),
            0.0,
        ),
    };

    let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
    if response.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    let base = ui.visuals().weak_text_color();
    let accent = ui.visuals().selection.bg_fill;
    let warm = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(244, 203, 112)
    } else {
        egui::Color32::from_rgb(190, 122, 31)
    };
    let lyric_rect = rect.shrink2(egui::vec2(LYRIC_SIDE_PADDING, 2.0));
    let available_width = lyric_rect.width().max(1.0);
    let mut font_size = LYRIC_FONT_SIZE;
    let mut galley = ui.painter().layout_no_wrap(
        text.clone(),
        egui::FontId::new(font_size, egui::FontFamily::Proportional),
        base,
    );
    if galley.size().x > available_width {
        font_size = (font_size * available_width / galley.size().x).clamp(12.0, LYRIC_FONT_SIZE);
        galley = ui.painter().layout_no_wrap(
            text,
            egui::FontId::new(font_size, egui::FontFamily::Proportional),
            base,
        );
    }

    let text_pos = egui::pos2(
        lyric_rect.center().x - galley.size().x * 0.5,
        lyric_rect.center().y - galley.size().y * 0.5,
    );
    let lyric_painter = ui.painter().with_clip_rect(lyric_rect);
    lyric_painter.galley(text_pos, galley.clone(), base);

    if progress <= 0.0 {
        return;
    }
    let frontier = text_pos.x + galley.size().x * progress.clamp(0.0, 1.0);
    let played_clip = lyric_rect.intersect(egui::Rect::from_min_max(
        lyric_rect.min,
        egui::pos2(frontier, lyric_rect.max.y),
    ));
    ui.painter()
        .with_clip_rect(played_clip)
        .galley_with_override_text_color(text_pos, galley.clone(), accent);

    let fade_width = galley.size().x.min(28.0);
    let fade_start = (frontier - fade_width).max(text_pos.x);
    const STEPS: usize = 8;
    for step in 0..STEPS {
        let left = egui::lerp(fade_start..=frontier, step as f32 / STEPS as f32);
        let right = egui::lerp(fade_start..=frontier, (step + 1) as f32 / STEPS as f32);
        let color = mix_color(accent, warm, (step + 1) as f32 / STEPS as f32);
        let clip = lyric_rect.intersect(egui::Rect::from_min_max(
            egui::pos2(left, lyric_rect.min.y),
            egui::pos2(right, lyric_rect.max.y),
        ));
        ui.painter()
            .with_clip_rect(clip)
            .galley_with_override_text_color(text_pos, galley.clone(), color);
    }
}

fn lyric_at(
    lyrics: &Lyrics,
    position: Duration,
    track_duration: Option<Duration>,
) -> Option<(&str, f32)> {
    let first = lyrics.lines.first()?;
    let Some(index) = lyrics.active_line(position) else {
        return Some((&first.text, 0.0));
    };
    let line = &lyrics.lines[index];
    let end = lyrics
        .lines
        .get(index + 1)
        .map(|next| next.time)
        .or(track_duration.filter(|duration| *duration > line.time))
        .unwrap_or_else(|| line.time + Duration::from_secs(4));
    let span = end.saturating_sub(line.time).as_secs_f32().max(0.01);
    let elapsed = position.saturating_sub(line.time).as_secs_f32();
    Some((&line.text, (elapsed / span).clamp(0.0, 1.0)))
}

fn mix_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::lrc::LyricLine;

    #[test]
    fn lyric_progress_uses_next_line_boundary() {
        let lyrics = Lyrics {
            lines: vec![
                LyricLine {
                    time: Duration::from_secs(10),
                    text: "第一句".into(),
                },
                LyricLine {
                    time: Duration::from_secs(20),
                    text: "第二句".into(),
                },
            ],
        };

        let (text, progress) = lyric_at(&lyrics, Duration::from_secs(15), None).unwrap();
        assert_eq!(text, "第一句");
        assert!((progress - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn lyric_before_first_line_is_not_highlighted() {
        let lyrics = Lyrics {
            lines: vec![LyricLine {
                time: Duration::from_secs(5),
                text: "开场".into(),
            }],
        };

        assert_eq!(
            lyric_at(&lyrics, Duration::from_secs(2), None),
            Some(("开场", 0.0))
        );
    }
}
