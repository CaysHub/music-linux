//! 歌词视图：当前行高亮放大、自动滚动居中（用户手动滚动后暂停跟随）、点击行跳转播放

use eframe::egui;

use crate::app::MusicApp;

const FOLLOW_RESUME: std::time::Duration = std::time::Duration::from_secs(3);

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(lyrics) = app.lyrics.clone() else {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.4);
            ui.weak("无歌词");
            ui.weak("将 .lrc 文件与音频文件放在同一目录并同名即可自动加载");
        });
        return;
    };
    if lyrics.lines.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.4);
            ui.weak("无歌词");
        });
        return;
    }

    let pos = app
        .engine
        .as_ref()
        .map(|e| e.position())
        .unwrap_or_default();
    let active = lyrics.active_line(pos);

    // 用户手动滚动检测（鼠标滚轮）：暂停自动跟随一段时间
    let user_scrolled = ui
        .ctx()
        .input(|i| i.events.iter().any(|ev| matches!(ev, egui::Event::MouseWheel { .. })));
    if user_scrolled && ui.rect_contains_pointer(ui.max_rect()) {
        app.lyrics_user_scroll = Some(std::time::Instant::now());
    }
    let follow = !app
        .lyrics_user_scroll
        .is_some_and(|t| t.elapsed() < FOLLOW_RESUME);

    let mut seek_to: Option<std::time::Duration> = None;

    egui::ScrollArea::vertical()
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
                let rich = if is_active {
                    egui::RichText::new(&text)
                        .strong()
                        .size(20.0)
                        .color(ui.visuals().selection.bg_fill)
                } else {
                    egui::RichText::new(&text).size(15.0).weak()
                };
                let resp = ui.add(
                    egui::Label::new(rich)
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
                ui.add_space(8.0);
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
