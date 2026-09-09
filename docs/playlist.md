# 播放列表与播放模式设计

## 数据结构

```rust
pub enum PlayMode { Sequential, RepeatOne, RepeatAll, Shuffle }

pub struct Track {
    pub path: PathBuf,
    pub title: String,          // 添加时用 lofty 读一次缓存；失败回退文件名
    pub artist: String,
    pub album: String,
    pub duration: Duration,     // lofty，回退 Decoder 探测
}

pub struct Playlist {
    tracks: Vec<Track>,
    current: Option<usize>,
    mode: PlayMode,
    shuffle_bag: Vec<usize>,      // 随机模式：本轮未播放索引
    shuffle_history: Vec<usize>,  // 随机模式：已播放历史（上一曲用）
}
```

## 切曲策略（纯函数，单测覆盖）

核心：`next_index(&mut self, manual: bool) -> Option<usize>`

- `manual == true`：用户点击"下一曲"（单曲循环下也要前进）
- `manual == false`：自然播完（单曲循环下返回当前索引重播）

| 模式 | 自动（播完） | 手动（点击） |
|---|---|---|
| Sequential | 下一首；播完列表返回 None（停止） | 同左 |
| RepeatAll | `(cur+1) % len` 绕回 | 同左 |
| RepeatOne | 当前索引（重播） | `(cur+1) % len` 前进 |
| Shuffle | 洗牌袋抽取 | 同左 |

`prev_index`：Shuffle 弹出历史栈；RepeatAll/RepeatOne 绕回末尾；Sequential 到头停。

**注意**：`next_index` 不修改 `current`，由调用方（MusicApp）应用返回值——便于测试，
也让"预览下一曲"成为可能。

### 洗牌袋算法

随机模式一轮内不重复：袋空时用 `(0..len).filter(≠current)` 重洗（xorshift +
Fisher-Yates，不引依赖）。只有一首歌时返回当前索引。

### 移除与索引修正

`remove(index)`：`current > index` 时减一；移除当前曲时 `current = None`，
由调用方决定停止或切歌。洗牌袋索引随之失效，简单起见直接清空（MVP 取舍）。

## 文件夹导入

递归遍历 + 扩展名白名单：
`mp3 flac ogg oga opus wav wave m4a mp4 aac aiff aif`，
目录内按路径排序后批量读标签入列。
