# music-linux

Linux 原生音乐播放器，使用 Rust 编写，功能参考 [MusicPlayer2](https://github.com/zhongyang219/MusicPlayer2)。

> 设计文档见 [docs/](docs/)：架构、音频引擎、播放列表、歌词、UI 规范与
> [踩坑记录](docs/pitfalls.md)（egui 0.36 / rodio 0.22 破坏性 API 变更等）。

## 功能

- 播放控制：播放 / 暂停 / 停止 / 上一曲 / 下一曲，进度条拖动 seek，音量调节
- 播放模式：顺序播放 / 列表循环 / 单曲循环 / 随机播放（洗牌袋算法，一轮不重复）
- 播放列表：添加文件 / 递归添加文件夹、双击播放、行尾红色移除按钮（悬浮二次确认）
- m3u 播放列表：保存（m3u8/UTF-8）、加载（兼容 m3u/m3u8，GBK 编码自动回退）
- 多格式解码：MP3 / FLAC / OGG / WAV / M4A(AAC) 等（symphonia）
- 标签读取：标题 / 艺术家 / 专辑 / 时长（lofty），无标签时回退文件名
- LRC 歌词：同名 `.lrc` 自动加载、当前行放大高亮、自动滚动居中（手动滚动后暂停跟随）、点击歌词行跳转播放
- 正在播放指示：列表当前行 4 柱跳动均衡器动画
- Material Symbols 矢量图标、深色 / 浅色主题切换
- 会话持久化：音量、播放模式、播放列表与当前曲目（`~/.config/music-linux/config.json`）

## 构建与运行

依赖：Rust 1.95+，以及 ALSA 开发库：

```bash
sudo apt install libasound2-dev
```

```bash
cargo run --release
```

## 技术栈

| 组件 | 选型 |
|---|---|
| GUI | egui / eframe 0.36（纯 Rust，无 GTK 依赖） |
| 音频 | rodio 0.22（MixerDeviceSink + Player）+ symphonia 解码 |
| 标签 | lofty 0.25 |
| 文件对话框 | rfd（xdg-portal 后端，运行期依赖 D-Bus） |
| 歌词 | 手写 LRC 解析（多时间标签 / offset / GBK 编码探测） |
| 播放列表 | 手写 m3u/m3u8 读写 |

架构说明：所有应用状态位于 UI 线程的 `MusicApp`，无自定义线程；自动切歌通过 egui 帧循环轮询 `Player::empty()` 实现，播放中每 100ms 请求重绘以刷新进度与歌词。

## 目录结构

```
src/
├── main.rs        # eframe 启动
├── app.rs         # MusicApp 中央状态机
├── audio.rs       # rodio 音频引擎封装
├── playlist.rs    # 播放列表 + 播放模式策略（纯逻辑，含单测）
├── tags.rs        # lofty 标签读取
├── lyrics/        # LRC 解析（纯函数，含单测）
├── m3u.rs         # m3u/m3u8 读写（纯函数，含单测）
├── config.rs      # 配置持久化
└── ui/            # 界面：信息栏 / 列表 / 歌词 / 控制栏
```

## 后续计划

- [ ] 频谱分析（FFT 可视化）
- [ ] 卡拉 OK 逐字歌词
- [ ] 专辑封面显示
- [ ] 均衡器
- [ ] MPRIS（媒体键 / 系统托盘）
- [ ] 在线歌词下载

## License

MIT
