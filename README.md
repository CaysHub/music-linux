# music-linux

使用 Rust、egui 和 rodio 编写的 Linux 原生音乐播放器，交互设计参考
[MusicPlayer2](https://github.com/zhongyang219/MusicPlayer2)。项目提供本地音乐播放、
LRC 歌词编辑，以及始终置顶的双行迷你播放器。

## 功能

- 播放控制：播放 / 暂停 / 停止 / 上一曲 / 下一曲、进度跳转和音量调节
- 播放模式：顺序播放、列表循环、单曲循环和随机播放
- 播放列表：添加多个文件、递归导入文件夹、双击播放；移除和清空均需二次确认
- 音频格式：MP3、FLAC、OGG、OGA、Opus、WAV、M4A、MP4、AAC 和 AIFF
- 音频信息：读取标题、艺术家、专辑和时长，标签缺失时回退到文件名
- LRC 歌词：自动加载同目录同名歌词、自动滚动、当前行高亮、点击歌词跳转
- 歌词编辑：新建或修改带时间轴的 LRC，支持插入当前时间和 `Ctrl+S` 保存
- 置顶模式：460 x 78 无标题栏窗口，36 px 控制行和 42 px 渐变歌词行
- 界面：深色 / 浅色主题、Material Symbols 图标、正在播放均衡器动画
- 会话恢复：保存音量、播放模式、播放列表和当前曲目

## 环境要求

- Linux 桌面环境（当前版本在 GNOME Wayland 会主动改用 XWayland）
- Rust 1.95 或更高版本
- ALSA 开发库；Ubuntu / Debian 可执行：

```bash
sudo apt install libasound2-dev
```

## 运行

开发模式：

```bash
cargo run --package music-linux --bin music-linux --profile dev
```

发布模式：

```bash
cargo run --release
```

安装到当前用户的 Ubuntu 应用菜单：

```bash
./packaging/linux/install-user.sh
```

安装脚本会构建 release 版本，并将程序、桌面条目和应用图标安装到 `~/.local`。
从应用菜单启动后，窗口和程序坞会使用同一个图标。

首次打开后，使用顶部的“添加文件”或“添加文件夹”建立播放列表，双击歌曲即可播放。
程序会把会话状态保存到 `~/.config/music-linux/config.json`。

## 歌词编辑

播放一首歌曲并切换到“歌词”标签页。歌词路径和“新建歌词”或“编辑歌词”按钮会显示在
标签栏同一行。可直接粘贴带时间轴的 LRC，也可边播放边使用“插入当前时间”。例如：

```lrc
[ti:歌曲名]
[ar:艺术家]
[00:12.34]第一句歌词
[00:18.70]第二句歌词
```

保存前必须至少包含一行有效时间轴。文件以 UTF-8 编码写入歌曲所在目录，文件名与歌曲
相同、扩展名为 `.lrc`。已有 GBK 编码的歌词可以读取，保存后统一为 UTF-8。

## 置顶迷你播放器

点击顶部“置顶歌词”进入迷你模式。窗口固定为 460 x 78，始终显示在其他窗口上方：

- 第一行左侧是拖动把手和四个播放控制，右侧是模式、恢复和关闭，中间显示歌名与动态均衡器
- 第二行显示当前歌词，颜色按当前歌词的时间进度从左向右推进
- 拖动左侧把手、歌名区域或歌词行可移动窗口
- 点击恢复按钮返回进入迷你模式前的窗口状态并在显示器中居中；主界面显示在最前方，切换到其他窗口后自动取消置顶

## 技术栈

| 组件 | 选型 |
|---|---|
| GUI | egui / eframe 0.36 |
| 音频 | rodio 0.22 + symphonia 解码 |
| 标签 | lofty 0.25 |
| 文件对话框 | rfd 0.17（xdg-portal 后端） |
| 歌词 | LRC 解析器，支持多时间标签、offset 和 GBK 编码探测 |
| 配置 | serde + serde_json |

应用状态集中在 UI 线程的 `MusicApp`，音频播放由 rodio 管理。播放期间每 50ms 请求一次
界面重绘，用于同步进度、歌词和列表动画。

## 开发

```bash
cargo fmt --check
cargo test --package music-linux
cargo build --package music-linux --bin music-linux --profile dev
```

项目结构：

```text
src/
|-- main.rs             # eframe 启动入口
|-- app.rs              # 应用状态与交互逻辑
|-- audio.rs            # rodio 音频引擎封装
|-- playlist.rs         # 播放列表与播放模式
|-- tags.rs             # 音频标签和时长读取
|-- config.rs           # 会话配置持久化
|-- lyrics/             # LRC 解析与查询
`-- ui/
    |-- info_bar.rs     # 顶部信息和操作栏
    |-- controls.rs     # 主窗口播放控制栏
    |-- playlist_view.rs
    |-- lyrics_view.rs  # 歌词展示与编辑
    `-- mini_player.rs  # 置顶双行迷你播放器
```

更详细的设计与实现说明见 [docs/](docs/)，包括
[架构](docs/architecture.md)、[音频引擎](docs/audio.md)、
[播放列表](docs/playlist.md)、[歌词](docs/lyrics.md)、
[界面设计](docs/ui-design.md)和[踩坑记录](docs/pitfalls.md)。

## 后续计划

- [ ] 频谱分析（FFT 可视化）
- [ ] 卡拉 OK 逐字歌词
- [ ] 专辑封面显示
- [ ] 均衡器
- [ ] MPRIS（媒体键 / 系统托盘）
- [ ] 在线歌词下载

## License

MIT
