//! music-linux：Linux 原生音乐播放器（Rust + egui + rodio），功能参考 MusicPlayer2。

mod app;
mod audio;
mod config;
mod lyrics;
mod m3u;
mod playlist;
mod tags;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([640.0, 420.0])
            .with_title("Music Player"),
        ..Default::default()
    };
    eframe::run_native(
        "music-linux",
        options,
        Box::new(|cc| Ok(Box::new(app::MusicApp::new(cc)))),
    )
}
