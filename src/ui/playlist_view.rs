//! 播放列表视图：双击播放、当前曲柔和高亮、行尾常驻移除按钮

use eframe::egui;

use crate::app::MusicApp;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    if app.playlist.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.4);
            ui.weak("播放列表为空");
            ui.weak("使用上方「添加文件 / 添加文件夹」按钮导入音乐");
        });
        return;
    }

    let mut play_index: Option<usize> = None;
    let mut remove_index: Option<usize> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, track) in app.playlist.tracks.iter().enumerate() {
                let is_current = app.playlist.current == Some(i);
                let row_id = egui::Id::new("playlist_row").with(i);
                // 用上一帧的 hover 状态决定本帧背景色（egui 惯用技巧）
                let hovered = ui
                    .ctx()
                    .read_response(row_id)
                    .is_some_and(|r| r.hovered());

                // 当前曲：主题色低透明度 + 强调色文字；悬停：浅色底
                let accent = ui.visuals().selection.bg_fill;
                let fill = if is_current {
                    egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 32)
                } else if hovered {
                    ui.visuals().widgets.hovered.weak_bg_fill
                } else {
                    egui::Color32::TRANSPARENT
                };

                let row = egui::Frame::NONE
                    .fill(fill)
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(10, 7))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // 序号 / 播放指示
                            let idx = if is_current {
                                egui::RichText::new("▶")
                                    .size(12.0)
                                    .color(accent)
                                    .monospace()
                            } else {
                                egui::RichText::new(format!("{: >3}", i + 1))
                                    .weak()
                                    .monospace()
                            };
                            ui.add_sized([30.0, 18.0], egui::Label::new(idx));

                            // 标题
                            let title = if is_current {
                                egui::RichText::new(&track.title).strong().color(accent)
                            } else {
                                egui::RichText::new(&track.title)
                            };
                            ui.add(egui::Label::new(title).truncate());

                            // 艺术家
                            if !track.artist.is_empty() {
                                ui.label(egui::RichText::new(track.artist.clone()).weak());
                            }

                            // 右侧：时长 + 移除按钮
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    remove_button(ui, &mut remove_index, i);
                                    ui.label(
                                        egui::RichText::new(super::format_duration(track.duration))
                                            .weak()
                                            .monospace(),
                                    );
                                },
                            );
                        });
                    });

                // 整行交互：双击播放、右键菜单（覆盖在行矩形上的透明 hit-test）
                let hit = ui.interact(row.response.rect, row_id, egui::Sense::click());
                if hit.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if hit.double_clicked() {
                    play_index = Some(i);
                }
                hit.context_menu(|ui| {
                    if ui.button("▶ 播放").clicked() {
                        play_index = Some(i);
                        ui.close();
                    }
                    if ui
                        .button(
                            egui::RichText::new("移除")
                                .color(egui::Color32::from_rgb(220, 90, 90)),
                        )
                        .clicked()
                    {
                        remove_index = Some(i);
                        ui.close();
                    }
                });
            }
        });

    ui.separator();
    ui.horizontal(|ui| {
        ui.weak(format!(
            "共 {} 首 · 双击播放 · 右键更多操作",
            app.playlist.len()
        ));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(egui::RichText::new("清空列表").color(egui::Color32::from_rgb(220, 90, 90)))
                .on_hover_text("清空播放列表")
                .clicked()
            {
                app.clear_playlist();
            }
        });
    });

    if let Some(i) = play_index {
        app.play_index(i, ctx);
    }
    if let Some(i) = remove_index {
        app.remove_track(i);
    }
}

/// 行尾移除按钮：自绘居中 ✕，平时低调灰色，悬停变红
fn remove_button(ui: &mut egui::Ui, remove_index: &mut Option<usize>, i: usize) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 20.0), egui::Sense::click());
    let hovered = resp.hovered();
    if hovered {
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(4),
            egui::Color32::from_rgba_unmultiplied(220, 90, 90, 40),
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    }
    let color = if hovered {
        egui::Color32::from_rgb(225, 100, 100)
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "✕",
        egui::FontId::proportional(11.0),
        color,
    );
    let clicked = resp.clicked();
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if clicked {
        *remove_index = Some(i);
    }
}
