//! m3u / m3u8 播放列表读写（参考 MusicPlayer2 的 Playlist.cpp，手写实现）。
//!
//! * 保存：一律 m3u8（UTF-8）
//! * 加载：兼容 m3u（本地编码，GBK 回退）与 m3u8（UTF-8）

use std::path::{Path, PathBuf};

use crate::playlist::Track;

/// 解析 m3u/m3u8 字节流为曲目路径列表。
/// 相对路径基于 `base_dir`（m3u 文件所在目录）解析。
pub fn parse_bytes(bytes: &[u8], base_dir: &Path) -> Vec<PathBuf> {
    let text = decode(bytes);
    let mut paths = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let p = PathBuf::from(line);
        if p.is_absolute() {
            paths.push(p);
        } else {
            paths.push(base_dir.join(p));
        }
    }
    paths
}

/// 从文件读取（自动判断编码）
pub fn parse_file(path: &Path) -> Vec<PathBuf> {
    let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    match std::fs::read(path) {
        Ok(bytes) => parse_bytes(&bytes, &base_dir),
        Err(_) => Vec::new(),
    }
}

/// 生成 m3u8 文本
pub fn to_m3u8_string(tracks: &[Track]) -> String {
    let mut s = String::from("#EXTM3U\n");
    for t in tracks {
        let display = if t.artist.is_empty() {
            t.title.clone()
        } else {
            format!("{} - {}", t.artist, t.title)
        };
        s.push_str(&format!("#EXTINF:{},{}\n", t.duration.as_secs(), display));
        s.push_str(&t.path.to_string_lossy());
        s.push('\n');
    }
    s
}

/// 保存为 m3u8 文件
pub fn save_file(path: &Path, tracks: &[Track]) -> std::io::Result<()> {
    std::fs::write(path, to_m3u8_string(tracks))
}

/// UTF-8 → GBK 回退
fn decode(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let (cow, _, _) = encoding_rs::GBK.decode(bytes);
    cow.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn track(path: &str) -> Track {
        Track::stub(PathBuf::from(path))
    }

    #[test]
    fn parse_absolute_and_relative() {
        let text = "#EXTM3U\n#EXTINF:123,歌 - 曲\n/tmp/a.mp3\nb.flac\n#comment\n";
        let paths = parse_bytes(text.as_bytes(), Path::new("/music"));
        assert_eq!(paths, vec![PathBuf::from("/tmp/a.mp3"), PathBuf::from("/music/b.flac")]);
    }

    #[test]
    fn parse_gbk() {
        // "/tmp/歌.mp3" 的 GBK 编码
        let (cow, _, _) = encoding_rs::GBK.encode("/tmp/歌.mp3\n");
        let paths = parse_bytes(&cow, Path::new("/music"));
        assert_eq!(paths, vec![PathBuf::from("/tmp/歌.mp3")]);
    }

    #[test]
    fn roundtrip() {
        let mut t = track("/tmp/x.mp3");
        t.title = "标题".into();
        t.artist = "歌手".into();
        t.duration = Duration::from_secs(61);
        let text = to_m3u8_string(&[t]);
        let paths = parse_bytes(text.as_bytes(), Path::new("/tmp"));
        assert_eq!(paths, vec![PathBuf::from("/tmp/x.mp3")]);
        assert!(text.contains("#EXTINF:61,歌手 - 标题"));
    }

    #[test]
    fn empty_input() {
        assert!(parse_bytes(b"", Path::new("/")).is_empty());
        assert!(parse_bytes(b"#EXTM3U\n", Path::new("/")).is_empty());
    }
}
