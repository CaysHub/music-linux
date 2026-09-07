//! 播放列表与播放模式策略。
//!
//! 切曲策略（下一曲/上一曲如何选择）集中在这里，是纯逻辑，无 IO，便于单元测试。

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// 播放模式（参考 MusicPlayer2 的 RepeatMode 取常用子集）
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    /// 顺序播放：播完列表即停止
    Sequential,
    /// 单曲循环
    RepeatOne,
    /// 列表循环
    RepeatAll,
    /// 随机播放（洗牌袋：一轮内不重复）
    Shuffle,
}

impl PlayMode {
    /// 控制栏按钮循环切换顺序
    pub fn next(self) -> Self {
        match self {
            PlayMode::Sequential => PlayMode::RepeatAll,
            PlayMode::RepeatAll => PlayMode::RepeatOne,
            PlayMode::RepeatOne => PlayMode::Shuffle,
            PlayMode::Shuffle => PlayMode::Sequential,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PlayMode::Sequential => "顺序播放",
            PlayMode::RepeatAll => "列表循环",
            PlayMode::RepeatOne => "单曲循环",
            PlayMode::Shuffle => "随机播放",
        }
    }
}

/// 一首曲目。标签信息在添加时读取一次并缓存。
#[derive(Clone, Debug)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
}

impl Track {
    /// 不读标签的占位构造（测试 / m3u 加载阶段使用，之后由后台回填）
    pub fn stub(path: PathBuf) -> Self {
        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        Self {
            path,
            title,
            artist: String::new(),
            album: String::new(),
            duration: Duration::ZERO,
        }
    }
}

/// 支持的音频扩展名（小写）
pub fn is_supported_audio(path: &std::path::Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|e| matches!(e.as_str(), "mp3" | "flac" | "ogg" | "oga" | "opus" | "wav" | "wave" | "m4a" | "mp4" | "aac" | "aiff" | "aif"))
}

pub struct Playlist {
    pub tracks: Vec<Track>,
    pub current: Option<usize>,
    pub mode: PlayMode,
    /// 随机模式：本轮尚未播放的索引（洗牌袋）
    shuffle_bag: Vec<usize>,
    /// 随机模式：已播放历史，用于"上一曲"
    shuffle_history: Vec<usize>,
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}

impl Playlist {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            current: None,
            mode: PlayMode::RepeatAll,
            shuffle_bag: Vec::new(),
            shuffle_history: Vec::new(),
        }
    }

    pub fn set_mode(&mut self, mode: PlayMode) {
        if self.mode != mode {
            self.mode = mode;
            self.shuffle_bag.clear();
            self.shuffle_history.clear();
        }
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn add(&mut self, track: Track) {
        self.tracks.push(track);
    }

    /// 移除一首。若移除的是当前曲目，current 置 None（由调用方决定停止/切歌）。
    pub fn remove(&mut self, index: usize) -> Option<Track> {
        if index >= self.tracks.len() {
            return None;
        }
        let removed = self.tracks.remove(index);
        match self.current {
            Some(c) if c == index => self.current = None,
            Some(c) if c > index => self.current = Some(c - 1),
            _ => {}
        }
        // 洗牌袋索引已失效，重置（简单可靠，MVP 足够）
        self.shuffle_bag.clear();
        self.shuffle_history.clear();
        Some(removed)
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current = None;
        self.shuffle_bag.clear();
        self.shuffle_history.clear();
    }

    /// 计算下一曲索引。
    ///
    /// * `manual == true`：用户手动点击"下一曲"（单曲循环下也要前进）
    /// * `manual == false`：当前曲目自然播完（单曲循环下重播当前曲）
    pub fn next_index(&mut self, manual: bool) -> Option<usize> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }
        let cur = self.current.unwrap_or(0);
        match self.mode {
            PlayMode::RepeatOne => {
                if manual {
                    Some((cur + 1) % len)
                } else {
                    Some(cur) // 自然播完 → 重播
                }
            }
            PlayMode::RepeatAll => Some((cur + 1) % len),
            PlayMode::Sequential => {
                if cur + 1 < len {
                    Some(cur + 1)
                } else {
                    None // 播完列表，停止
                }
            }
            PlayMode::Shuffle => {
                let next = self.draw_shuffle_next(cur, len);
                self.shuffle_history.push(cur);
                next
            }
        }
    }

    pub fn prev_index(&mut self) -> Option<usize> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }
        let cur = self.current.unwrap_or(0);
        match self.mode {
            PlayMode::Shuffle => {
                if let Some(prev) = self.shuffle_history.pop() {
                    Some(prev)
                } else {
                    Some(cur.saturating_sub(1))
                }
            }
            PlayMode::Sequential => Some(cur.saturating_sub(1)),
            PlayMode::RepeatAll | PlayMode::RepeatOne => Some((cur + len - 1) % len),
        }
    }

    /// 从洗牌袋抽取下一曲；袋空则重洗（排除当前曲）开始新一轮。
    fn draw_shuffle_next(&mut self, cur: usize, len: usize) -> Option<usize> {
        if self.shuffle_bag.is_empty() {
            let mut bag: Vec<usize> = (0..len).filter(|&i| i != cur).collect();
            if bag.is_empty() {
                return Some(cur); // 整个列表只有一首歌
            }
            fisher_yates(&mut bag);
            self.shuffle_bag = bag;
        }
        self.shuffle_bag.pop()
    }
}

/// 简单 xorshift PRNG + Fisher-Yates 洗牌（不引依赖）
fn fisher_yates(v: &mut [usize]) {
    if v.len() < 2 {
        return;
    }
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15)
        | 1;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for i in (1..v.len()).rev() {
        let j = (next() as usize) % (i + 1);
        v.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playlist(n: usize, mode: PlayMode) -> Playlist {
        let mut p = Playlist::new();
        p.set_mode(mode);
        for i in 0..n {
            p.add(Track::stub(PathBuf::from(format!("/tmp/t{i}.mp3"))));
        }
        p
    }

    #[test]
    fn sequential_stops_at_end() {
        let mut p = playlist(3, PlayMode::Sequential);
        p.current = Some(2);
        assert_eq!(p.next_index(false), None);
        assert_eq!(p.next_index(true), None);
    }

    #[test]
    fn sequential_advances() {
        let mut p = playlist(3, PlayMode::Sequential);
        p.current = Some(1);
        assert_eq!(p.next_index(false), Some(2));
    }

    #[test]
    fn repeat_all_wraps() {
        let mut p = playlist(3, PlayMode::RepeatAll);
        p.current = Some(2);
        assert_eq!(p.next_index(false), Some(0)); // 末尾绕回开头
        p.current = Some(0);
        assert_eq!(p.prev_index(), Some(2)); // 开头往前绕回末尾
    }

    #[test]
    fn repeat_one_replays_on_auto_advances_on_manual() {
        let mut p = playlist(3, PlayMode::RepeatOne);
        p.current = Some(1);
        assert_eq!(p.next_index(false), Some(1)); // 自然播完重播
        assert_eq!(p.next_index(true), Some(2)); // 手动前进
    }

    #[test]
    fn shuffle_visits_all_once_per_round() {
        let mut p = playlist(5, PlayMode::Shuffle);
        p.current = Some(0);
        let mut played = vec![0usize];
        for _ in 0..4 {
            let i = p.next_index(false).expect("应有下一曲");
            p.current = Some(i);
            played.push(i);
        }
        // 一轮 5 首（含起点）全部不重复
        played.sort();
        played.dedup();
        assert_eq!(played.len(), 5);
        // 袋已空，再抽会开新一轮（排除当前曲）
        let cur = p.current.unwrap();
        let next = p.next_index(false).unwrap();
        assert_ne!(next, cur);
    }

    #[test]
    fn shuffle_prev_uses_history() {
        let mut p = playlist(5, PlayMode::Shuffle);
        p.current = Some(0);
        let a = p.next_index(true).unwrap();
        p.current = Some(a);
        let b = p.next_index(true).unwrap();
        p.current = Some(b);
        assert_eq!(p.prev_index(), Some(a));
        assert_eq!(p.prev_index(), Some(0));
    }

    #[test]
    fn remove_fixes_current_index() {
        let mut p = playlist(3, PlayMode::RepeatAll);
        p.current = Some(2);
        p.remove(0).unwrap();
        assert_eq!(p.current, Some(1));
        // 移除当前曲 → current 置 None
        p.remove(1).unwrap();
        assert_eq!(p.current, None);
    }

    #[test]
    fn empty_playlist_returns_none() {
        let mut p = playlist(0, PlayMode::RepeatAll);
        assert_eq!(p.next_index(true), None);
        assert_eq!(p.prev_index(), None);
    }
}
