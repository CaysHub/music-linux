//! 歌词模块：LRC 解析与查询。

use std::path::{Path, PathBuf};

pub mod lrc;

pub use lrc::{decode_text, parse_bytes, parse_str, Lyrics};

/// 与音频同目录、同名的 LRC 文件路径。
pub fn sidecar_path(audio: &Path) -> PathBuf {
    audio.with_extension("lrc")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_keeps_directory_and_full_stem() {
        assert_eq!(
            sidecar_path(Path::new("/music/song.live.flac")),
            PathBuf::from("/music/song.live.lrc")
        );
    }
}
