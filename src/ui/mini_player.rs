//! 始终置顶的双行迷你播放器：紧凑播放控制 + 单行进度歌词。

use std::time::Duration;

use eframe::egui;
use egui_material_icons::icons::*;

use crate::app::MusicApp;
use crate::audio::PlaybackState;
use crate::lyrics::Lyrics;
use crate::playlist::PlayMode;

const CONTROL_ROW_HEIGHT: f32 = 36.0;
const LYRIC_ROW_HEIGHT: f32 = 42.0;
const BUTTON_SIZE: egui::Vec2 = egui::vec2(22.0, 20.0);
const CONTROL_RIGHT_PADDING: f32 = 10.0;
const TITLE_SIDE_PADDING: f32 = 8.0;
const TITLE_EQUALIZER_GAP: f32 = 5.0;
const TITLE_EQUALIZER_SIZE: egui::Vec2 = egui::vec2(20.0, 16.0);
const TITLE_FONT_SIZE: f32 = 14.0;
const LYRIC_SIDE_PADDING: f32 = 24.0;
const LYRIC_VERTICAL_PADDING: f32 = 5.0;
const LYRIC_FONT_SIZE: f32 = 20.0;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let background = super::app_background(app.dark_mode);
    ui.painter()
        .rect_filled(ui.max_rect(), egui::CornerRadius::ZERO, background);

    ui.spacing_mut().item_spacing = egui::vec2(5.0, 0.0);
    ui.spacing_mut().button_padding = egui::vec2(2.0, 2.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), CONTROL_ROW_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| draw_controls(app, ui, ctx),
    );

    let divider_y = ui.min_rect().top() + CONTROL_ROW_HEIGHT;
    ui.painter().hline(
        ui.max_rect().x_range(),
        divider_y,
        ui.visuals().widgets.noninteractive.bg_stroke,
    );
    draw_lyric_line(app, ui, ctx);
}

fn draw_controls(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
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

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(CONTROL_RIGHT_PADDING);
        if icon_button(ui, ICON_CLOSE, 13.0, "关闭播放器").clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if icon_button(ui, ICON_OPEN_IN_FULL, 13.0, "恢复主界面").clicked() {
            app.exit_mini_mode(ctx);
        }
        let mode = app.playlist.mode;
        if icon_button(ui, mode_icon(mode), 13.0, mode.label()).clicked() {
            app.cycle_play_mode();
        }

        draw_track_title(app, ui, ctx, playing);
    });
}

fn draw_track_title(app: &MusicApp, ui: &mut egui::Ui, ctx: &egui::Context, playing: bool) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), BUTTON_SIZE.y),
        egui::Sense::drag(),
    );
    if response.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    response.on_hover_text("拖动迷你窗口");

    let current_track = app.current_track();
    let title = current_track
        .map(|track| track.title.as_str())
        .unwrap_or("未播放");
    let equalizer_width = if current_track.is_some() {
        TITLE_EQUALIZER_GAP + TITLE_EQUALIZER_SIZE.x
    } else {
        0.0
    };
    let max_title_width = (rect.width() - TITLE_SIDE_PADDING * 2.0 - equalizer_width).max(1.0);
    let galley = truncated_title_galley(ui, title, max_title_width);
    let group_width = galley.size().x + equalizer_width;
    let group_left = rect.center().x - group_width * 0.5;
    let text_pos = egui::pos2(group_left, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(
        text_pos,
        galley,
        if current_track.is_some() {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        },
    );

    if current_track.is_some() {
        let equalizer_rect = egui::Rect::from_center_size(
            egui::pos2(
                group_left + group_width - TITLE_EQUALIZER_SIZE.x * 0.5,
                rect.center().y,
            ),
            TITLE_EQUALIZER_SIZE,
        );
        super::draw_equalizer(ui, equalizer_rect, ui.visuals().selection.bg_fill, playing);
    }
}

fn truncated_title_galley(
    ui: &egui::Ui,
    title: &str,
    max_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let font = egui::FontId::new(TITLE_FONT_SIZE, egui::FontFamily::Proportional);
    let color = ui.visuals().text_color();
    let full = ui
        .painter()
        .layout_no_wrap(title.to_owned(), font.clone(), color);
    if full.size().x <= max_width {
        return full;
    }

    let chars: Vec<char> = title.chars().collect();
    let mut low = 0;
    let mut high = chars.len();
    let mut best = ui
        .painter()
        .layout_no_wrap("…".to_owned(), font.clone(), color);
    while low <= high {
        let middle = low + (high - low) / 2;
        let candidate = format!("{}…", chars[..middle].iter().collect::<String>());
        let galley = ui.painter().layout_no_wrap(candidate, font.clone(), color);
        if galley.size().x <= max_width {
            best = galley;
            low = middle + 1;
        } else if middle == 0 {
            break;
        } else {
            high = middle - 1;
        }
    }
    best
}

fn icon_button(
    ui: &mut egui::Ui,
    icon: egui_material_icons::MaterialIcon,
    size: f32,
    tooltip: &str,
) -> egui::Response {
    icon_button_response(ui, icon, size).on_hover_text(tooltip)
}

fn icon_button_response(
    ui: &mut egui::Ui,
    icon: egui_material_icons::MaterialIcon,
    size: f32,
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
    response
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

    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), LYRIC_ROW_HEIGHT),
        egui::Sense::drag(),
    );
    if response.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    let lyric_rect = rect.shrink2(egui::vec2(LYRIC_SIDE_PADDING, LYRIC_VERTICAL_PADDING));
    let available_width = lyric_rect.width().max(1.0);
    let mut font_size = LYRIC_FONT_SIZE;
    let mut galley = ui.painter().layout_no_wrap(
        text.clone(),
        egui::FontId::new(font_size, egui::FontFamily::Proportional),
        ui.visuals().weak_text_color(),
    );
    if galley.size().x > available_width {
        font_size = (font_size * available_width / galley.size().x).clamp(12.0, LYRIC_FONT_SIZE);
        galley = ui.painter().layout_no_wrap(
            text,
            egui::FontId::new(font_size, egui::FontFamily::Proportional),
            ui.visuals().weak_text_color(),
        );
    }

    let text_pos = egui::pos2(
        lyric_rect.center().x - galley.size().x * 0.5,
        lyric_rect.center().y - galley.size().y * 0.5,
    );
    super::paint_lyric_progress(ui, lyric_rect, text_pos, galley, progress);
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
    Some((
        &line.text,
        super::lyric_progress(lyrics, index, position, track_duration),
    ))
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
