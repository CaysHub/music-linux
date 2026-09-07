//! 播放列表视图：双击播放、当前曲高亮、单行移除

use eframe::egui;

use crate::app::MusicApp;

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    if app.playlist.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.4);
            ui.weak("播放列表为空");
            ui.weak("使用下方「添加文件 / 添加文件夹」按钮导入音乐");
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
                let row = egui::Frame::NONE
                    .fill(if is_current {
                        ui.visuals().selection.bg_fill
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // 序号 / 播放指示
                            let indicator = if is_current { "▶" } else { "" };
                            ui.label(
                                egui::RichText::new(format!("{:>3} {}", i + 1, indicator))
                                    .weak()
                                    .monospace(),
                            );
                            let title_text = if is_current {
                                egui::RichText::new(&track.title).strong()
                            } else {
                                egui::RichText::new(&track.title)
                            };
                            let resp = ui.selectable_label(is_current, title_text);
                            if resp.double_clicked() {
                                play_index = Some(i);
                            }
                            if !track.artist.is_empty() {
                                ui.weak(format!("— {}", track.artist));
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button("✕")
                                        .on_hover_text("移除")
                                        .clicked()
                                    {
                                        remove_index = Some(i);
                                    }
                                    let dur = super::format_duration(track.duration);
                                    ui.label(egui::RichText::new(dur).weak().monospace());
                                },
                            );
                        });
                    });
                // 整行双击也可播放（点击行内空白区域）
                if row.response.double_clicked() {
                    play_index = Some(i);
                }
            }
        });

    ui.separator();
    ui.weak(format!("共 {} 首", app.playlist.len()));

    if let Some(i) = play_index {
        app.play_index(i, ctx);
    }
    if let Some(i) = remove_index {
        app.remove_track(i);
    }
}
