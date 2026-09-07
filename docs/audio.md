# 音频引擎设计

## rodio 0.22 全新 API（重要）

rodio 0.19 → 0.22 经历了破坏性重构，网上资料几乎全是旧 API，**以 docs.rs 0.22
为唯一权威**：

| 旧 API（已删除） | 新 API |
|---|---|
| `OutputStream::try_default()` | `DeviceSinkBuilder::open_default_sink()` |
| `Sink::new(&stream_handle)` | `Player::connect_new(&device.mixer())` |
| `Decoder::new(file)` | `Decoder::try_from(file)` |
| （无） | `Player::get_pos() / try_seek() / empty()` |

`Player` 已内建播放位置查询、seek、播完检测——**无需自定义 Source 包装或手动计时**
（这是选型 rodio 0.22 的决定性原因）。

## AudioEngine 设计

```rust
pub struct AudioEngine {
    _device: MixerDeviceSink,   // 必须保活：drop 即停止播放
    player: Player,
    pub state: PlaybackState,   // Stopped / Playing / Paused
    started_at: Instant,        // 见「竞态防护」
}
```

### 播放：先解码校验，后切换

```rust
pub fn play_file(&mut self, path: &Path) -> Result<(), String> {
    let file = File::open(path)?;
    let src = Decoder::try_from(file)?;   // 失败不影响正在播放的曲目
    self.player.stop();
    self.player.append(src);
    self.player.play();
    self.started_at = Instant::now();
    Ok(())
}
```

`stop()` 后 `append()` 会自动恢复播放（rodio 内部处理，见其 player.rs append 实现）。

### 竞态防护：播完判定

`append()` 与 `sound_count` 生效之间存在短暂竞态（append 后 `empty()` 可能仍为
true）。用启动宽限期规避：

```rust
pub fn track_finished(&self) -> bool {
    self.state == PlaybackState::Playing
        && self.started_at.elapsed() > Duration::from_millis(300)
        && self.player.empty()
}
```

仅播放态判定：用户 stop 后 `empty()` 也为 true，靠 state 区分，不会误触发。

### seek

- 直接 `player.try_seek(pos)`；symphonia 后端按格式精确度不同（MP3 帧边界估算、
  Vorbis 可能有偏差），失败时静默保留原位
- `try_seek` 内部会同步更新 `controls.position`，无需 seek 后抑制显示
- UI 侧：拖动中只更新 `seek_drag` 预览值，`drag_stopped()` 才真正 seek，避免高频抖动

### 自动切歌：帧循环轮询

无后台线程。eframe 0.36 的 `App::logic()` 中：

```rust
fn logic(&mut self, ctx, _frame) {
    if engine.track_finished() { self.on_track_end(ctx); }
    if playing { ctx.request_repaint_after(Duration::from_millis(50)); }
}
```

`logic()` 在窗口隐藏时也会被调用（配合 `request_repaint_after` 链），后台播放切歌
不中断。50ms 同时驱动：进度条、歌词高亮、列表行均衡器动画。

### 单曲循环优化

自然播完 + RepeatOne 时用 `engine.replay()`（`try_seek(ZERO)` + `play`），
避免重新解码。

## 格式支持

rodio 0.22 默认特性已含 symphonia：flac / mp3 / mp4(aac,m4a) / vorbis / wav。
时长优先 lofty `properties().duration()`（播前缓存进 Track），
lofty 拿不到时用 Decoder `total_duration()` 兜底。
