//! music-linux：Linux 原生音乐播放器（Rust + egui + rodio），功能参考 MusicPlayer2。

mod app;
mod audio;
mod config;
mod lyrics;
mod playlist;
mod tags;
mod ui;

fn main() -> eframe::Result {
    // GNOME Wayland 下 winit 的客户端标题栏渲染存在中文乱码/按钮错位问题，
    // 移除 WAYLAND_DISPLAY 强制走 X11(XWayland)：Mutter 的 X11 服务端标题栏渲染成熟可靠。
    // 如需恢复 Wayland 原生，删除此行即可。
    std::env::remove_var("WAYLAND_DISPLAY");

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_min_inner_size([820.0, 500.0])
            .with_transparent(false)
            .with_title("Music Player 音乐播放器"),
        ..Default::default()
    };
    eframe::run_native(
        "music-linux",
        options,
        Box::new(|cc| Ok(Box::new(app::MusicApp::new(cc)))),
    )
}
