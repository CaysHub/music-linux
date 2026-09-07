//! 应用配置持久化：~/.config/music-linux/config.json

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::playlist::PlayMode;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub volume: f32,
    pub play_mode: PlayMode,
    /// 上次会话打开的曲目路径（用于恢复列表）
    pub open_tracks: Vec<PathBuf>,
    pub last_index: Option<usize>,
    pub last_playlist_path: Option<PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            volume: 0.8,
            play_mode: PlayMode::RepeatAll,
            open_tracks: Vec::new(),
            last_index: None,
            last_playlist_path: None,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("music-linux").join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}
