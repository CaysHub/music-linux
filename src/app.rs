//! MusicApp：中央状态机，组合音频引擎、播放列表、歌词与配置。

use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui;

use crate::audio::{AudioEngine, PlaybackState};
use crate::config::AppConfig;
use crate::lyrics;
use crate::m3u;
use crate::playlist::{is_supported_audio, PlayMode, Playlist, Track};
use crate::tags;

/// 主区域显示的标签页
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Playlist,
    Lyrics,
}

pub struct MusicApp {
    pub engine: Option<AudioEngine>,
    pub playlist: Playlist,
    pub lyrics: Option<lyrics::Lyrics>,
    pub volume: f32,
    pub tab: MainTab,
    pub dark_mode: bool,
    /// 进度条拖动中的预览位置（秒）
    pub seek_drag: Option<f64>,
    /// 用户最近一次手动滚动歌词的时间（暂停自动跟随）
    pub lyrics_user_scroll: Option<std::time::Instant>,
    pub error: Option<String>,
    pub config: AppConfig,
}

impl MusicApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);

        let config = AppConfig::load();
        let engine_result = AudioEngine::new();
        if let Err(e) = &engine_result {
            eprintln!("{e}");
        }
        let engine = engine_result.ok();
        if let Some(eng) = engine.as_ref() {
            eng.set_volume(config.volume.clamp(0.0, 1.0));
        }

        let mut playlist = Playlist::new();
        playlist.set_mode(config.play_mode);
        // 恢复上次会话的列表（不自动播放）
        for path in &config.open_tracks {
            if path.is_file() {
                playlist.add(tags::load_track(path));
            }
        }
        playlist.current = config
            .last_index
            .filter(|&i| i < playlist.tracks.len());

        let dark_mode = true;
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let error = engine
            .is_none()
            .then(|| "音频设备不可用，播放功能已禁用".to_string());

        Self {
            engine,
            playlist,
            lyrics: None,
            volume: config.volume.clamp(0.0, 1.0),
            tab: MainTab::Playlist,
            dark_mode,
            seek_drag: None,
            lyrics_user_scroll: None,
            error,
            config,
        }
    }

    // ---------- 播放控制 ----------

    /// 播放指定曲目，并加载同名歌词
    pub fn play_index(&mut self, index: usize, ctx: &egui::Context) {
        let Some(track) = self.playlist.tracks.get(index).cloned() else {
            return;
        };
        let Some(engine) = self.engine.as_mut() else {
            self.error = Some("音频设备不可用".into());
            return;
        };
        match engine.play_file(&track.path) {
            Ok(()) => {
                self.playlist.current = Some(index);
                self.lyrics = load_lyrics_for(&track.path);
                self.lyrics_user_scroll = None;
                let title = if track.artist.is_empty() {
                    track.title.clone()
                } else {
                    format!("{} - {}", track.artist, track.title)
                };
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                    "{title} - Music Player"
                )));
                self.config.last_index = Some(index);
                self.save_config();
            }
            Err(e) => self.error = Some(e),
        }
    }

    pub fn toggle_play(&mut self, ctx: &egui::Context) {
        let Some(engine) = self.engine.as_mut() else {
            self.error = Some("音频设备不可用".into());
            return;
        };
        match engine.state {
            PlaybackState::Playing => engine.pause(),
            PlaybackState::Paused => engine.resume(),
            PlaybackState::Stopped => {
                let idx = self.playlist.current.unwrap_or(0);
                if self.playlist.is_empty() {
                    return;
                }
                let idx = idx.min(self.playlist.len() - 1);
                engine.state = PlaybackState::Stopped;
                self.play_index(idx, ctx);
            }
        }
    }

    /// manual: 用户点击"下一曲"；否则为自然播完
    pub fn next(&mut self, manual: bool, ctx: &egui::Context) {
        if self.playlist.is_empty() {
            return;
        }
        match self.playlist.next_index(manual) {
            Some(i) => self.play_index(i, ctx),
            None => self.stop_all(),
        }
    }

    pub fn prev(&mut self, ctx: &egui::Context) {
        if self.playlist.is_empty() {
            return;
        }
        // 播放超过 3 秒时"上一曲"先回到本曲开头（常见播放器行为）
        if let Some(engine) = self.engine.as_ref() {
            if engine.position() > Duration::from_secs(3)
                && engine.state != PlaybackState::Stopped
            {
                engine.seek(Duration::ZERO);
                return;
            }
        }
        if let Some(i) = self.playlist.prev_index() {
            self.play_index(i, ctx);
        }
    }

    pub fn stop_all(&mut self) {
        if let Some(engine) = self.engine.as_mut() {
            engine.stop();
        }
        self.lyrics_user_scroll = None;
    }

    /// 用户拖动进度条 seek，同时恢复歌词自动跟随
    pub fn seek(&mut self, pos: Duration) {
        if let Some(engine) = self.engine.as_ref() {
            engine.seek(pos);
        }
        self.lyrics_user_scroll = None;
    }

    /// 当前曲目自然播完
    fn on_track_end(&mut self, ctx: &egui::Context) {
        match self.playlist.next_index(false) {
            Some(i) => {
                let replay_same = self.playlist.mode == PlayMode::RepeatOne
                    && self.playlist.current == Some(i);
                if replay_same {
                    // 单曲循环：seek 回开头重播，避免重新解码
                    if let Some(engine) = self.engine.as_mut() {
                        engine.replay();
                    }
                } else {
                    self.play_index(i, ctx);
                }
            }
            None => self.stop_all(),
        }
    }

    // ---------- 列表操作 ----------

    pub fn add_paths(&mut self, paths: Vec<PathBuf>) {
        let mut added = 0usize;
        for p in paths {
            if p.is_dir() {
                let mut files = Vec::new();
                walk_dir(&p, &mut files);
                files.sort();
                for f in files {
                    self.playlist.add(tags::load_track(&f));
                    added += 1;
                }
            } else if is_supported_audio(&p) && p.is_file() {
                self.playlist.add(tags::load_track(&p));
                added += 1;
            }
        }
        if added == 0 {
            self.error = Some("没有找到可添加的音频文件".into());
        }
        self.save_config();
    }

    pub fn remove_track(&mut self, index: usize) {
        let was_current = self.playlist.current == Some(index);
        self.playlist.remove(index);
        if was_current {
            self.stop_all();
        }
        self.save_config();
    }

    pub fn clear_playlist(&mut self) {
        self.stop_all();
        self.playlist.clear();
        self.lyrics = None;
        self.save_config();
    }

    // ---------- m3u ----------

    pub fn load_playlist_file(&mut self, path: &Path) {
        let paths = m3u::parse_file(path);
        if paths.is_empty() {
            self.error = Some(format!("播放列表为空或无法读取: {}", path.display()));
            return;
        }
        self.stop_all();
        self.playlist.clear();
        for p in paths {
            if p.is_file() {
                self.playlist.add(tags::load_track(&p));
            }
        }
        self.config.last_playlist_path = Some(path.to_path_buf());
        self.save_config();
    }

    pub fn save_playlist_file(&mut self, path: &Path) {
        match m3u::save_file(path, &self.playlist.tracks) {
            Ok(()) => {
                self.config.last_playlist_path = Some(path.to_path_buf());
                self.save_config();
            }
            Err(e) => self.error = Some(format!("保存失败: {e}")),
        }
    }

    // ---------- 其他 ----------

    pub fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 1.0);
        if let Some(engine) = self.engine.as_ref() {
            engine.set_volume(self.volume);
        }
        self.save_config();
    }

    pub fn cycle_play_mode(&mut self) {
        let mode = self.playlist.mode.next();
        self.playlist.set_mode(mode);
        self.save_config();
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.playlist.current.and_then(|i| self.playlist.tracks.get(i))
    }

    /// 当前播放位置（秒），拖动进度条时返回预览值
    pub fn display_position_secs(&self) -> f64 {
        if let Some(d) = self.seek_drag {
            return d;
        }
        self.engine
            .as_ref()
            .map(|e| e.position().as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn save_config(&mut self) {
        self.config.volume = self.volume;
        self.config.play_mode = self.playlist.mode;
        self.config.open_tracks = self.playlist.tracks.iter().map(|t| t.path.clone()).collect();
        self.config.last_index = self.playlist.current;
        self.config.save();
    }
}

/// 加载与音频同目录同名 .lrc
fn load_lyrics_for(audio: &Path) -> Option<lyrics::Lyrics> {
    let lrc_path = audio.with_extension("lrc");
    let bytes = std::fs::read(lrc_path).ok()?;
    lyrics::parse_bytes(&bytes)
}

/// 递归收集支持的音频文件
fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_dir(&p, out);
        } else if is_supported_audio(&p) {
            out.push(p);
        }
    }
}

/// 加载系统中文字体（egui 默认无 CJK，否则中文全是方块）
fn setup_fonts(ctx: &egui::Context) {
    let candidates = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
    ];
    let data = candidates
        .iter()
        .find_map(|p| std::fs::read(p).ok());
    let Some(data) = data else {
        eprintln!("警告：未找到系统中文字体（Noto CJK/文泉驿等），中文可能无法显示");
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".into(), std::sync::Arc::new(egui::FontData::from_owned(data)));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push("cjk".into());
    }
    ctx.set_fonts(fonts);
}

impl eframe::App for MusicApp {
    /// 每帧逻辑（窗口隐藏时也会被调用，适合自动切歌与重绘调度）
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 自动切歌（帧循环轮询，播放中本来就需要刷新进度/歌词）
        if self.engine.as_ref().is_some_and(|e| e.track_finished()) {
            self.on_track_end(ctx);
        }
        if self
            .engine
            .as_ref()
            .is_some_and(|e| e.state == PlaybackState::Playing)
        {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::ui::draw(self, ui);
    }
}
