# 总体架构

## 技术栈

| 组件 | 选型 | 版本 | 备注 |
|---|---|---|---|
| GUI | egui / eframe | 0.36 | 纯 Rust 即时模式，无 GTK 依赖 |
| 音频 | rodio（内含 symphonia 解码） | 0.22 | 全新 API，见 [audio.md](audio.md) |
| 标签 | lofty | 0.25 | 需显式导入 trait，见 [pitfalls.md](pitfalls.md) |
| 图标 | egui_material_icons | 0.8 | Material Symbols 字体，egui 0.36 兼容 |
| 文件对话框 | rfd | 0.17 | xdg-portal 后端，构建期无需 GTK |
| 序列化 | serde + serde_json | 1 | 配置持久化 |
| 编码容错 | encoding_rs | 0.8 | GBK ↔ UTF-8（中文 LRC/m3u 常见 GBK） |
| 路径 | dirs | 6 | `~/.config` 定位 |

## 模块划分

```
src/
├── main.rs        # eframe 启动；移除 WAYLAND_DISPLAY 强制 X11（见 pitfalls）
├── app.rs         # MusicApp 中央状态机：组合所有模块、播放动作、配置写回
├── audio.rs       # AudioEngine：rodio Player 封装（播放/暂停/seek/位置）
├── playlist.rs    # Track/Playlist/PlayMode + 切曲策略（纯逻辑，单测）
├── tags.rs        # lofty 标签读取 + rodio 时长探测回退
├── lyrics/        # LRC 解析（纯函数，单测）+ active_line 二分查找
├── m3u.rs         # m3u/m3u8 读写（纯函数，单测）
├── config.rs      # AppConfig → ~/.config/music-linux/config.json
└── ui/            # info_bar / playlist_view / lyrics_view / controls
```

依赖方向：`ui → app → {audio, playlist, tags, lyrics, m3u, config}`。
playlist / lyrics / m3u 不依赖 egui 与音频，可独立测试。

## 线程模型（本方案最大优点）

**全部应用状态在 UI 线程的 `MusicApp`，无自定义线程、无 channel。**

- rodio 内部自带音频线程，通过 `Player`（`Send + Sync`）通信
- 自动切歌：egui 帧循环轮询 `Player::empty()`，无后台线程
- 播放中每 50ms `request_repaint_after`（进度/歌词/均衡器动画共用一次重绘）

## 核心状态结构

```rust
pub struct MusicApp {
    engine: Option<AudioEngine>,   // None = 无音频设备（UI 仍可用）
    playlist: Playlist,            // 曲目 + current + PlayMode + 洗牌袋
    lyrics: Option<Lyrics>,        // 当前曲歌词（播曲时加载）
    volume: f32,
    tab: MainTab,                  // 播放列表 / 歌词
    seek_drag: Option<f64>,        // 进度条拖动预览
    error: Option<String>,         // 错误提示条
    config: AppConfig,             // 写穿式持久化
}
```

`engine` 用 `Option` 而非 panic：无音频设备时窗口照常打开，操作时提示错误。

## 播放动作流

```
用户操作(ui) → MusicApp::play_index/next/prev/toggle_play
            → playlist.next_index(manual)   // 纯策略：返回下一曲索引
            → engine.play_file(path)        // 先解码校验，成功才切换
            → 加载同名 .lrc / 更新窗口标题 / 保存配置
```

`next_index(manual)` 区分手动切曲与自然播完：单曲循环模式下手动前进、自动重播。

## 配置持久化

写穿式（write-through）：音量/模式/列表/当前曲在任何变更后立即落盘
（JSON 很小，代价可忽略），无需退出钩子，程序被杀也不丢状态。
