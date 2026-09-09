//! UI 布局组装：顶部信息栏 / 中部标签页 / 底部控制栏 / 错误提示。

mod controls;
mod info_bar;
mod lyrics_view;
mod playlist_view;

use eframe::egui;
use egui_material_icons::icons::*;

use crate::app::{MainTab, MusicApp};

const RED: egui::Color32 = egui::Color32::from_rgb(225, 95, 95);
const TOP_BAR_HEIGHT: f32 = 52.0;
const BOTTOM_BAR_HEIGHT: f32 = 54.0;

pub fn app_background(dark_mode: bool) -> egui::Color32 {
    if dark_mode {
        egui::Color32::from_rgb(14, 17, 16)
    } else {
        egui::Color32::from_rgb(236, 241, 238)
    }
}

fn panel_background(dark_mode: bool) -> egui::Color32 {
    if dark_mode {
        egui::Color32::from_rgb(22, 25, 24)
    } else {
        egui::Color32::from_rgb(247, 249, 247)
    }
}

pub fn set_app_theme(ctx: &egui::Context, dark_mode: bool) {
    use egui::TextStyle::*;

    ctx.all_styles_mut(|style| {
        style.text_styles.insert(
            Small,
            egui::FontId::new(11.5, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            Body,
            egui::FontId::new(14.5, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            Button,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            Heading,
            egui::FontId::new(20.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            Monospace,
            egui::FontId::new(14.0, egui::FontFamily::Monospace),
        );

        style.spacing.item_spacing = egui::vec2(9.0, 5.0);
        style.spacing.button_padding = egui::vec2(9.0, 4.0);
        style.spacing.interact_size = egui::vec2(44.0, 28.0);
        style.spacing.slider_rail_height = 7.0;
        style.spacing.menu_margin = egui::Margin::symmetric(8, 7);
        style.spacing.tooltip_width = 360.0;
        style.interaction.interact_radius = 6.0;
        style.animation_time = 0.16;
    });

    ctx.set_theme(if dark_mode {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
    ctx.set_visuals(app_visuals(dark_mode));
}

fn app_visuals(dark_mode: bool) -> egui::Visuals {
    let accent = egui::Color32::from_rgb(62, 184, 150);
    let mut visuals = if dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = egui::Stroke::new(
        1.0,
        if dark_mode {
            egui::Color32::from_rgb(236, 252, 246)
        } else {
            egui::Color32::from_rgb(9, 55, 43)
        },
    );
    visuals.hyperlink_color = accent;
    visuals.error_fg_color = egui::Color32::from_rgb(235, 105, 105);
    visuals.warn_fg_color = egui::Color32::from_rgb(226, 166, 70);
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);

    if dark_mode {
        visuals.panel_fill = panel_background(dark_mode);
        visuals.window_fill = egui::Color32::from_rgb(27, 31, 29);
        visuals.extreme_bg_color = app_background(dark_mode);
        visuals.faint_bg_color = egui::Color32::from_rgb(27, 32, 30);
        visuals.code_bg_color = egui::Color32::from_rgb(31, 38, 35);
        visuals.weak_text_color = Some(egui::Color32::from_rgb(165, 176, 170));

        visuals.widgets.noninteractive.bg_fill = visuals.panel_fill;
        visuals.widgets.noninteractive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(225, 232, 228));
        visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(32, 38, 35);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(36, 43, 40);
        visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(49, 59, 55));
        visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(42, 51, 47);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(46, 58, 53);
        visuals.widgets.hovered.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(71, 112, 98));
        visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(41, 66, 57);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(44, 82, 69);
    } else {
        visuals.panel_fill = panel_background(dark_mode);
        visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
        visuals.extreme_bg_color = app_background(dark_mode);
        visuals.faint_bg_color = egui::Color32::from_rgb(241, 246, 243);
        visuals.code_bg_color = egui::Color32::from_rgb(232, 239, 235);
        visuals.weak_text_color = Some(egui::Color32::from_rgb(94, 106, 99));

        visuals.widgets.noninteractive.bg_fill = visuals.panel_fill;
        visuals.widgets.noninteractive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 38, 34));
        visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(236, 242, 238);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(229, 237, 233);
        visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(207, 218, 212));
        visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(222, 235, 228);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(215, 231, 223);
        visuals.widgets.hovered.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(119, 171, 153));
        visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(196, 224, 214);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(186, 216, 205);
    }

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(6);
        widget.expansion = 0.0;
    }

    visuals
}

pub fn draw(app: &mut MusicApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::ZERO,
        app_background(app.dark_mode),
    );

    egui::Panel::top("info_bar")
        .exact_size(TOP_BAR_HEIGHT)
        .frame(
            egui::Frame::new()
                .fill(panel_background(app.dark_mode))
                .inner_margin(egui::Margin::symmetric(16, 10)),
        )
        .show_separator_line(true)
        .show(ui, |ui| {
            ui.painter().rect_filled(
                ui.max_rect(),
                egui::CornerRadius::ZERO,
                panel_background(app.dark_mode),
            );
            info_bar::draw(app, ui);
        });

    egui::Panel::bottom("controls")
        .exact_size(BOTTOM_BAR_HEIGHT)
        .frame(
            egui::Frame::new()
                .fill(panel_background(app.dark_mode))
                .inner_margin(egui::Margin::symmetric(16, 10)),
        )
        .show_separator_line(true)
        .show(ui, |ui| {
            ui.painter().rect_filled(
                ui.max_rect(),
                egui::CornerRadius::ZERO,
                panel_background(app.dark_mode),
            );
            controls::draw(app, ui, &ctx);
        });

    // 错误提示条
    if app.error.is_some() {
        egui::Panel::bottom("error_bar")
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(120, 40, 40))
                    .inner_margin(egui::Margin::symmetric(12, 4)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(app.error.clone().unwrap_or_default())
                            .color(egui::Color32::WHITE),
                    );
                    if ui.small_button("✕").clicked() {
                        app.error = None;
                    }
                });
            });
    }

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(app_background(app.dark_mode))
                .inner_margin(egui::Margin::symmetric(16, 12)),
        )
        .show(ui, |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
            ui.painter().rect_filled(
                ui.max_rect(),
                egui::CornerRadius::ZERO,
                app_background(app.dark_mode),
            );
            // 标签页切换
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::selectable(app.tab == MainTab::Playlist, "播放列表")
                            .corner_radius(egui::CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    app.tab = MainTab::Playlist;
                }
                if ui
                    .add(
                        egui::Button::selectable(app.tab == MainTab::Lyrics, "歌词")
                            .corner_radius(egui::CornerRadius::same(6)),
                    )
                    .clicked()
                {
                    app.tab = MainTab::Lyrics;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.tab == MainTab::Playlist && !app.playlist.is_empty() {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(format!(
                                        "{} 清空列表",
                                        ICON_DELETE.codepoint
                                    ))
                                    .color(RED)
                                    .size(14.0),
                                )
                                .corner_radius(egui::CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            app.clear_playlist();
                        }
                        ui.add_space(4.0);
                    }
                    let (icon, text) = if app.dark_mode {
                        (ICON_LIGHT_MODE, "浅色")
                    } else {
                        (ICON_DARK_MODE, "深色")
                    };
                    if ui
                        .add(
                            egui::Button::new(format!("{} {text}", icon.codepoint))
                                .corner_radius(egui::CornerRadius::same(6)),
                        )
                        .on_hover_text("切换明暗主题")
                        .clicked()
                    {
                        app.dark_mode = !app.dark_mode;
                        set_app_theme(&ctx, app.dark_mode);
                    }
                });
            });
            ui.add_space(8.0);

            match app.tab {
                MainTab::Playlist => playlist_view::draw(app, ui, &ctx),
                MainTab::Lyrics => lyrics_view::draw(app, ui, &ctx),
            }
        });
}

/// mm:ss（超过一小时则 h:mm:ss）
pub fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}
