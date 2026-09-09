# music-linux 设计文档

本目录记录 music-linux 播放器的全部设计决策与实现要点。

| 文档 | 内容 |
|---|---|
| [architecture.md](architecture.md) | 总体架构、技术栈、线程模型 |
| [audio.md](audio.md) | 音频引擎设计（rodio 0.22 新 API） |
| [playlist.md](playlist.md) | 播放列表与播放模式策略、文件夹导入 |
| [lyrics.md](lyrics.md) | LRC 歌词解析与展示设计 |
| [ui-design.md](ui-design.md) | UI 布局设计、交互规范、视觉规范 |
| [pitfalls.md](pitfalls.md) | 踩坑记录（egui 0.36 / rodio 0.22 / GNOME，必读） |

## 项目背景

参考 [MusicPlayer2](https://github.com/zhongyang219/MusicPlayer2)（Windows/MFC）
用 Rust 实现的 Linux 原生音乐播放器。第一期为精简版 MVP：播放控制、播放列表、
多格式解码、标签读取、LRC 歌词。

## 核心设计原则

1. **简单可靠优先**：无自定义音频线程、无 channel——所有应用状态在 UI 线程的
   `MusicApp`，rodio 自带音频线程
2. **纯逻辑可测试**：播放模式策略与 LRC 解析均配有单元测试
3. **坏文件零影响**：标签读取失败一律静默回退（文件名/占位符），绝不让单个坏文件
   打断添加流程
