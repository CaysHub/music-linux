//! LRC 歌词解析（参考 MusicPlayer2 的 Lyric.cpp 行为子集）。
//!
//! 支持：
//! * 编码探测：UTF-8 BOM → UTF-8 → GBK 回退（中文 LRC 常见 GBK）
//! * 行首多个时间标签展开（压缩 LRC：`[00:01.00][00:05.00]歌词`）
//! * `[offset:±ms]` 整体偏移（正值歌词提前）
//! * `[mm:ss]` `[mm:ss.xx]` `[mm:ss.xxx]` 及旧式 `[mm:ss:xx]`
//! * 忽略 `[ti:]` `[ar:]` `[al:]` 等元数据标签

use std::time::Duration;

/// 一行歌词
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricLine {
    pub time: Duration,
    pub text: String,
}

/// 解析后的歌词，已按时间升序排序
#[derive(Clone, Debug, Default)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
}

impl Lyrics {
    /// 二分查找：当前播放位置对应的行（最后一个 time <= pos）。
    /// pos 在第一行之前时返回 None。
    pub fn active_line(&self, pos: Duration) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }
        if pos < self.lines[0].time {
            return None;
        }
        let idx = self
            .lines
            .partition_point(|l| l.time <= pos)
            .saturating_sub(1);
        Some(idx)
    }
}

/// 解析字节流（自动探测编码）
pub fn parse_bytes(bytes: &[u8]) -> Option<Lyrics> {
    let text = decode(bytes);
    parse_str(&text)
}

/// BOM → UTF-8 → GBK
fn decode(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let (cow, _, had_errors) = encoding_rs::GBK.decode(bytes);
    if had_errors {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        cow.into_owned()
    }
}

/// 解析已解码文本
pub fn parse_str(s: &str) -> Option<Lyrics> {
    let mut offset_ms: i64 = 0;
    let mut lines: Vec<LyricLine> = Vec::new();

    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        // 提取行首连续的 [..] 标签
        let mut times: Vec<Duration> = Vec::new();
        let mut rest = line;
        while rest.starts_with('[') {
            let Some(close) = rest.find(']') else { break };
            let tag = &rest[1..close];
            if let Some(t) = parse_time_tag(tag) {
                times.push(t);
            } else if let Some(v) = tag
                .strip_prefix("offset:")
                .map(|v| v.trim().trim_start_matches(['+', '-']))
            {
                // LRC 规范：正值使歌词整体提前
                let sign_neg = tag.contains('-');
                let ms: i64 = v.parse().unwrap_or(0);
                offset_ms = if sign_neg { -ms } else { ms };
            }
            // 其他元数据标签（ti/ar/al/by...）忽略
            rest = &rest[close + 1..];
            if !rest.starts_with('[') {
                break;
            }
        }

        let text = rest.trim();
        if text.is_empty() {
            continue; // 纯元数据行
        }
        for t in times {
            lines.push(LyricLine {
                time: t,
                text: text.to_string(),
            });
        }
    }

    if lines.is_empty() {
        return None;
    }

    // 应用 offset：正值提前 → 时间减去 offset
    if offset_ms != 0 {
        for l in &mut lines {
            let ms = l.time.as_millis() as i64 - offset_ms;
            l.time = if ms <= 0 {
                Duration::ZERO
            } else {
                Duration::from_millis(ms as u64)
            };
        }
    }

    lines.sort_by_key(|l| l.time);
    Some(Lyrics { lines })
}

/// 解析单个时间标签内容（不含方括号）：`mm:ss` `mm:ss.xx` `mm:ss.xxx` `mm:ss:xx`
fn parse_time_tag(s: &str) -> Option<Duration> {
    let (mm_str, rest) = s.split_once(':')?;
    let mm: u64 = mm_str.trim().parse().ok()?;
    // 旧式 `[mm:ss:xx]`（xx 为百分之一秒）统一成 `.`
    let rest = rest.replace(':', ".");
    let (ss_str, frac) = match rest.split_once('.') {
        Some((a, b)) => (a.to_string(), b.to_string()),
        None => (rest, String::new()),
    };
    let ss: u64 = ss_str.trim().parse().ok()?;
    let millis: u64 = match frac.chars().count() {
        0 => 0,
        1 => frac.parse::<u64>().ok()? * 100,
        2 => frac.parse::<u64>().ok()? * 10,
        _ => frac.chars().take(3).collect::<String>().parse().ok()?,
    };
    Some(Duration::from_millis(mm * 60_000 + ss * 1_000 + millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_parse_and_sort() {
        let lrc = parse_str("[00:10.00]第二行\n[00:05.00]第一行\n").unwrap();
        assert_eq!(lrc.lines.len(), 2);
        assert_eq!(lrc.lines[0].time, Duration::from_millis(5_000));
        assert_eq!(lrc.lines[0].text, "第一行");
    }

    #[test]
    fn multi_time_tags_expand() {
        let lrc = parse_str("[00:01.00][00:05.00]重复歌词\n").unwrap();
        assert_eq!(lrc.lines.len(), 2);
        assert_eq!(lrc.lines[0].text, "重复歌词");
        assert_eq!(lrc.lines[1].text, "重复歌词");
    }

    #[test]
    fn time_tag_variants() {
        assert_eq!(parse_time_tag("01:30"), Some(Duration::from_secs(90)));
        assert_eq!(parse_time_tag("01:30.5"), Some(Duration::from_millis(90_500)));
        assert_eq!(
            parse_time_tag("01:30.500"),
            Some(Duration::from_millis(90_500))
        );
        assert_eq!(
            parse_time_tag("01:30:50"), // 旧式百分秒
            Some(Duration::from_millis(90_500))
        );
        assert_eq!(parse_time_tag("ti:歌名"), None);
    }

    #[test]
    fn metadata_lines_ignored() {
        let lrc = parse_str("[ti:测试]\n[ar:某人]\n\n[00:01.00]歌词行\n").unwrap();
        assert_eq!(lrc.lines.len(), 1);
    }

    #[test]
    fn offset_shifts_earlier() {
        // 正 offset → 歌词提前 500ms
        let lrc = parse_str("[offset:+500]\n[00:10.00]行\n").unwrap();
        assert_eq!(lrc.lines[0].time, Duration::from_millis(9_500));
        // 负 offset → 推后
        let lrc = parse_str("[offset:-500]\n[00:10.00]行\n").unwrap();
        assert_eq!(lrc.lines[0].time, Duration::from_millis(10_500));
    }

    #[test]
    fn gbk_decoded() {
        // "歌词" 的 GBK 编码
        let gbk_bytes = &[0xB8, 0xE8, 0xB4, 0xCA];
        let (cow, _, _) = encoding_rs::GBK.encode("歌词");
        assert_eq!(&cow[..], gbk_bytes);
        let lrc_bytes = &[0x5B, 0x30, 0x30, 0x3A, 0x30, 0x31, 0x2E, 0x30, 0x30, 0x5D]; // [00:01.00]
        let mut full = lrc_bytes.to_vec();
        full.extend_from_slice(gbk_bytes);
        let lrc = parse_bytes(&full).unwrap();
        assert_eq!(lrc.lines[0].text, "歌词");
    }

    #[test]
    fn active_line_lookup() {
        let lrc = parse_str("[00:05.00]A\n[00:10.00]B\n[00:20.00]C\n").unwrap();
        assert_eq!(lrc.active_line(Duration::from_millis(4_999)), None);
        assert_eq!(lrc.active_line(Duration::from_millis(5_000)), Some(0));
        assert_eq!(lrc.active_line(Duration::from_millis(15_000)), Some(1));
        assert_eq!(lrc.active_line(Duration::from_secs(60)), Some(2));
    }

    #[test]
    fn empty_or_garbage_returns_none() {
        assert!(parse_str("").is_none());
        assert!(parse_str("没有任何标签的纯文本").is_none());
        assert!(parse_bytes(&[]).is_none());
    }
}
