//! 曲目标签读取（lofty），失败一律回退文件名，绝不让单个坏文件打断流程。

use std::path::Path;
use std::time::Duration;

use rodio::{Decoder, Source};

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;

use crate::playlist::Track;

/// 读取标签与时长。任何失败都静默降级：标题用文件名，艺术家/专辑留空（UI 显示占位符）。
pub fn load_track(path: &Path) -> Track {
    let mut track = Track::stub(path.to_path_buf());

    if let Ok(tagged) = lofty::read_from_path(path) {
        if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
            if let Some(t) = non_empty(tag.title()) {
                track.title = t;
            }
            if let Some(a) = non_empty(tag.artist()) {
                track.artist = a;
            }
            if let Some(al) = non_empty(tag.album()) {
                track.album = al;
            }
        }
        let d = tagged.properties().duration();
        if d > Duration::ZERO {
            track.duration = d;
        }
    }

    // lofty 拿不到时长时，用 rodio Decoder 探测
    if track.duration == Duration::ZERO {
        if let Ok(file) = std::fs::File::open(path) {
            if let Ok(dec) = Decoder::try_from(file) {
                if let Some(d) = dec.total_duration() {
                    track.duration = d;
                }
            }
        }
    }

    track
}

fn non_empty(s: Option<std::borrow::Cow<'_, str>>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}
