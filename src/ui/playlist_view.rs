//! 播放列表视图：双击播放、当前曲跳动均衡器动画、行尾红色移除按钮

use eframe::egui;
use egui_material_icons::icons::*;

use crate::app::{MusicApp, PendingPlaylistAction};
use crate::audio::PlaybackState;

const RED: egui::Color32 = egui::Color32::from_rgb(225, 95, 95);
const ROW_H: f32 = 46.0;
const ROW_PAD_X: f32 = 12.0;
const INDEX_W: f32 = 34.0;
const TIME_W: f32 = 58.0;
const REMOVE_W: f32 = 66.0;
const COLUMN_GAP: f32 = 10.0;

fn delete_label() -> String {
    format!("{} 移除", ICON_DELETE.codepoint)
}

fn row_fill(ui: &egui::Ui, index: usize) -> egui::Color32 {
    if ui.visuals().dark_mode {
        if index % 2 == 1 {
            egui::Color32::from_rgb(27, 32, 30)
        } else {
            egui::Color32::from_rgb(18, 22, 20)
        }
    } else if index % 2 == 1 {
        egui::Color32::from_rgb(241, 246, 243)
    } else {
        egui::Color32::from_rgb(250, 252, 250)
    }
}

fn current_row_fill(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgb(29, 61, 51)
    } else {
        egui::Color32::from_rgb(214, 238, 229)
    }
}

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    if app.playlist.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.4);
            ui.label(egui::RichText::new("播放列表为空").size(16.0).weak());
            ui.label(egui::RichText::new("使用上方「添加文件 / 添加文件夹」按钮导入音乐").weak());
        });
        return;
    }

    let mut play_index: Option<usize> = None;
    let mut remove_index: Option<usize> = None;
    let playing = app
        .engine
        .as_ref()
        .is_some_and(|e| e.state == PlaybackState::Playing);

    let list_h = ui.available_height();
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), list_h),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt("playlist_tracks")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    for (i, track) in app.playlist.tracks.iter().enumerate() {
                        let is_current = app.playlist.current == Some(i);
                        let row_id = egui::Id::new("playlist_row").with(i);
                        let row_w = ui.available_width();
                        let (row_rect, _) =
                            ui.allocate_exact_size(egui::vec2(row_w, ROW_H), egui::Sense::hover());
                        let hovered = ui.rect_contains_pointer(row_rect);

                        // 当前曲：不透明强调底色 + 强调色文字；悬停：浅色底。
                        let accent = ui.visuals().selection.bg_fill;
                        let fill = if is_current {
                            current_row_fill(ui)
                        } else if hovered {
                            ui.visuals().widgets.hovered.weak_bg_fill
                        } else {
                            row_fill(ui, i)
                        };
                        let stroke = if is_current {
                            egui::Stroke::new(1.0, accent)
                        } else {
                            egui::Stroke::NONE
                        };
                        ui.painter().rect(
                            row_rect.shrink(0.5),
                            egui::CornerRadius::same(6),
                            fill,
                            stroke,
                            egui::StrokeKind::Inside,
                        );

                        let mut remove_rect = egui::Rect::NOTHING;
                        let content_rect = row_rect.shrink2(egui::vec2(ROW_PAD_X, 0.0));
                        let mut row_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .id_salt(row_id.with("content"))
                                .max_rect(content_rect)
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        );
                        row_ui.set_clip_rect(ui.clip_rect().intersect(row_rect));

                        let (indicator_rect, _) = row_ui
                            .allocate_exact_size(egui::vec2(INDEX_W, 24.0), egui::Sense::hover());
                        row_ui.painter().text(
                            indicator_rect.right_center(),
                            egui::Align2::RIGHT_CENTER,
                            format!("{: >3}", i + 1),
                            egui::FontId::monospace(13.5),
                            if is_current {
                                accent
                            } else {
                                ui.visuals().weak_text_color()
                            },
                        );

                        row_ui.add_space(COLUMN_GAP);
                        let right_w = TIME_W + REMOVE_W + COLUMN_GAP;
                        let text_w = (row_ui.available_width() - right_w).max(90.0);
                        let (text_rect, _) = row_ui
                            .allocate_exact_size(egui::vec2(text_w, ROW_H), egui::Sense::hover());
                        draw_track_text(
                            &row_ui,
                            text_rect,
                            &track.title,
                            &track.artist,
                            &track.album,
                            is_current,
                            accent,
                            playing,
                        );

                        row_ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                remove_rect = remove_button(ui, &mut remove_index, &track.title, i);
                                ui.add_space(COLUMN_GAP);
                                ui.add_sized(
                                    [TIME_W, 20.0],
                                    egui::Label::new(
                                        egui::RichText::new(super::format_duration(track.duration))
                                            .weak()
                                            .monospace()
                                            .size(13.5),
                                    )
                                    .halign(egui::Align::RIGHT),
                                );
                            },
                        );

                        // 整行交互：双击播放、右键菜单。
                        // 注意：hit 区域必须排除行尾移除按钮，否则会遮挡其点击
                        //（egui 中后注册的组件在点击测试中位于顶层）。
                        let mut hit_rect = row_rect;
                        if remove_rect.is_finite() {
                            hit_rect.set_right(remove_rect.left() - 4.0);
                        }
                        let hit = ui.interact(hit_rect, row_id, egui::Sense::click());
                        if hit.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if hit.double_clicked() {
                            play_index = Some(i);
                        }
                        hit.context_menu(|ui| {
                            if ui
                                .button(format!("{} 播放", ICON_PLAY_ARROW.codepoint))
                                .clicked()
                            {
                                play_index = Some(i);
                                ui.close();
                            }
                            if ui
                                .button(egui::RichText::new(delete_label()).color(RED).size(14.0))
                                .clicked()
                            {
                                remove_index = Some(i);
                                ui.close();
                            }
                        });
                    }
                });
        },
    );

    if let Some(i) = play_index {
        app.play_index(i, ctx);
    }
    if let Some(i) = remove_index {
        if let Some(track) = app.playlist.tracks.get(i) {
            app.pending_playlist_action = Some(PendingPlaylistAction::Remove {
                index: i,
                title: track.title.clone(),
            });
        }
    }
}

fn draw_track_text(
    ui: &egui::Ui,
    rect: egui::Rect,
    title: &str,
    artist: &str,
    album: &str,
    is_current: bool,
    accent: egui::Color32,
    playing: bool,
) {
    let meta = track_meta(artist, album);
    let clip_rect = ui.clip_rect().intersect(rect);
    let painter = ui.painter().with_clip_rect(clip_rect);
    let title_color = if is_current {
        accent
    } else {
        ui.visuals().text_color()
    };
    let title_y = if meta.is_empty() {
        rect.center().y
    } else {
        rect.top() + 17.0
    };
    let title_galley = painter.layout_no_wrap(
        title.to_owned(),
        egui::FontId::proportional(if is_current { 15.5 } else { 15.0 }),
        title_color,
    );
    let equalizer_rect = is_current.then(|| {
        const EQUALIZER_WIDTH: f32 = 20.0;
        const EQUALIZER_HEIGHT: f32 = 16.0;
        const TITLE_GAP: f32 = 8.0;
        let left =
            (rect.left() + title_galley.size().x + TITLE_GAP).min(rect.right() - EQUALIZER_WIDTH);
        egui::Rect::from_min_size(
            egui::pos2(left, title_y - EQUALIZER_HEIGHT * 0.5),
            egui::vec2(EQUALIZER_WIDTH, EQUALIZER_HEIGHT),
        )
    });
    let title_clip = if let Some(equalizer_rect) = equalizer_rect {
        clip_rect.intersect(egui::Rect::from_min_max(
            rect.min,
            egui::pos2(equalizer_rect.left() - 6.0, rect.max.y),
        ))
    } else {
        clip_rect
    };
    ui.painter().with_clip_rect(title_clip).galley(
        egui::pos2(rect.left(), title_y - title_galley.size().y * 0.5),
        title_galley,
        title_color,
    );
    if let Some(equalizer_rect) = equalizer_rect {
        super::draw_equalizer(ui, equalizer_rect, accent, playing);
    }
    if !meta.is_empty() {
        painter.text(
            egui::pos2(rect.left(), rect.top() + 35.0),
            egui::Align2::LEFT_CENTER,
            meta,
            egui::FontId::proportional(12.5),
            ui.visuals().weak_text_color(),
        );
    }
}

fn track_meta(artist: &str, album: &str) -> String {
    match (artist.is_empty(), album.is_empty()) {
        (true, true) => String::new(),
        (false, true) => artist.to_owned(),
        (true, false) => format!("《{album}》"),
        (false, false) => format!("{artist} · 《{album}》"),
    }
}

/// 行尾移除按钮：与右键菜单相同的删除图标和“移除”文案。
/// 返回按钮矩形，供整行 hit 区域排除。
fn remove_button(
    ui: &mut egui::Ui,
    remove_index: &mut Option<usize>,
    title: &str,
    i: usize,
) -> egui::Rect {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(REMOVE_W, 28.0), egui::Sense::click());
    let hovered = resp.hovered();
    let active = resp.is_pointer_button_down_on();

    let (bg, fg, stroke) = if active {
        (
            egui::Color32::from_rgb(97, 42, 42),
            egui::Color32::from_rgb(255, 204, 204),
            egui::Color32::from_rgb(172, 75, 75),
        )
    } else if hovered {
        (
            egui::Color32::from_rgb(80, 38, 38),
            egui::Color32::from_rgb(248, 145, 145),
            egui::Color32::from_rgb(141, 63, 63),
        )
    } else {
        (
            egui::Color32::from_rgb(55, 31, 31),
            RED,
            egui::Color32::from_rgb(91, 45, 45),
        )
    };
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(6),
        bg,
        egui::Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        delete_label(),
        egui::TextStyle::Button.resolve(ui.style()),
        fg,
    );
    let clicked = resp.clicked();
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!("移除：{title}"));
    if clicked {
        *remove_index = Some(i);
    }

    rect
}
