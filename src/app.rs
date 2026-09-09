//! MusicApp：中央状态机，组合音频引擎、播放列表、歌词与配置。

use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui;

use crate::audio::{AudioEngine, PlaybackState};
use crate::config::AppConfig;
use crate::lyrics;
use crate::playlist::{is_supported_audio, PlayMode, Playlist, Track};
use crate::tags;

const MINI_WINDOW_SIZE: egui::Vec2 = egui::vec2(320.0, 60.0);
const MINI_RESIZE_ATTEMPTS: u8 = 20;

/// 主区域显示的标签页
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Playlist,
    Lyrics,
}

#[derive(Clone)]
pub enum PendingPlaylistAction {
    Remove { index: usize, title: String },
    Clear,
}

pub struct LyricsEditor {
    pub audio_path: PathBuf,
    pub lrc_path: PathBuf,
    pub track_title: String,
    pub text: String,
    pub original_text: String,
    pub error: Option<String>,
}

impl LyricsEditor {
    pub fn is_dirty(&self) -> bool {
        self.text != self.original_text
    }
}

pub struct MusicApp {
    pub engine: Option<AudioEngine>,
    pub playlist: Playlist,
    pub lyrics: Option<lyrics::Lyrics>,
    pub volume: f32,
    pub tab: MainTab,
    pub dark_mode: bool,
    pub mini_mode: bool,
    normal_window_size: Option<egui::Vec2>,
    normal_window_maximized: bool,
    mini_resize_attempts: u8,
    /// 进度条拖动中的预览位置（秒）
    pub seek_drag: Option<f64>,
    /// 用户最近一次手动滚动歌词的时间（暂停自动跟随）
    pub lyrics_user_scroll: Option<std::time::Instant>,
    pub lyrics_editor: Option<LyricsEditor>,
    pub pending_playlist_action: Option<PendingPlaylistAction>,
    pub error: Option<String>,
    pub config: AppConfig,
}

impl MusicApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        // 注册 Material Symbols 图标字体（已作为 Proportional 的回退字体，
        // 任何文本中的图标码点都能直接渲染）
        egui_material_icons::initialize(&cc.egui_ctx);

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
        playlist.current = config.last_index.filter(|&i| i < playlist.tracks.len());

        let dark_mode = true;
        crate::ui::set_app_theme(&cc.egui_ctx, dark_mode);

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
            mini_mode: false,
            normal_window_size: None,
            normal_window_maximized: false,
            mini_resize_attempts: 0,
            seek_drag: None,
            lyrics_user_scroll: None,
            lyrics_editor: None,
            pending_playlist_action: None,
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
            if engine.position() > Duration::from_secs(3) && engine.state != PlaybackState::Stopped
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
                let replay_same =
                    self.playlist.mode == PlayMode::RepeatOne && self.playlist.current == Some(i);
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
            self.lyrics = None;
        }
        self.save_config();
    }

    pub fn clear_playlist(&mut self) {
        self.stop_all();
        self.playlist.clear();
        self.lyrics = None;
        self.save_config();
    }

    // ---------- 歌词编辑 ----------

    pub fn begin_lyrics_edit(&mut self) -> Result<(), String> {
        let Some(track) = self.current_track().cloned() else {
            return Err("请先播放一首歌曲".into());
        };
        let lrc_path = lyrics::sidecar_path(&track.path);
        let (text, original_text) = match std::fs::read(&lrc_path) {
            Ok(bytes) => {
                let text = lyrics::decode_text(&bytes);
                (text.clone(), text)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                (lyrics_template(&track), String::new())
            }
            Err(e) => return Err(format!("无法读取歌词文件 {}: {e}", lrc_path.display())),
        };

        self.lyrics_editor = Some(LyricsEditor {
            audio_path: track.path,
            lrc_path,
            track_title: track.title,
            text,
            original_text,
            error: None,
        });
        Ok(())
    }

    pub fn cancel_lyrics_edit(&mut self) {
        self.lyrics_editor = None;
    }

    pub fn save_lyrics_edit(&mut self) -> Result<PathBuf, String> {
        let Some(editor) = self.lyrics_editor.as_ref() else {
            return Err("当前没有正在编辑的歌词".into());
        };
        let Some(parsed) = lyrics::parse_str(&editor.text) else {
            return Err("未检测到有效时间轴，请使用 [mm:ss.xx]歌词 格式".into());
        };

        let audio_path = editor.audio_path.clone();
        let lrc_path = editor.lrc_path.clone();
        let text = editor.text.clone();
        let file_name = lrc_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("lyrics.lrc");
        let temp_path = lrc_path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));

        if let Err(e) = std::fs::write(&temp_path, text.as_bytes()) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!("无法写入歌词文件 {}: {e}", lrc_path.display()));
        }
        if let Err(e) = std::fs::rename(&temp_path, &lrc_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!("无法保存歌词文件 {}: {e}", lrc_path.display()));
        }

        if self
            .current_track()
            .is_some_and(|track| track.path == audio_path)
        {
            self.lyrics = Some(parsed);
            self.lyrics_user_scroll = None;
        }
        self.lyrics_editor = None;
        Ok(lrc_path)
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

    pub fn enter_mini_mode(&mut self, ctx: &egui::Context) {
        if self.mini_mode {
            return;
        }
        (self.normal_window_size, self.normal_window_maximized) = ctx.input(|input| {
            (
                input.viewport().inner_rect.map(|rect| rect.size()),
                input.viewport().maximized.unwrap_or(false),
            )
        });
        self.mini_mode = true;
        self.mini_resize_attempts = MINI_RESIZE_ATTEMPTS;
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
        request_mini_window_size(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
    }

    pub fn exit_mini_mode(&mut self, ctx: &egui::Context) {
        if !self.mini_mode {
            return;
        }
        self.mini_mode = false;
        self.mini_resize_attempts = 0;
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
            820.0, 500.0,
        )));
        ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(egui::Vec2::INFINITY));
        if self.normal_window_maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                self.normal_window_size
                    .take()
                    .unwrap_or(egui::vec2(1200.0, 700.0)),
            ));
        }
        self.normal_window_maximized = false;
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.playlist
            .current
            .and_then(|i| self.playlist.tracks.get(i))
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
        self.config.open_tracks = self
            .playlist
            .tracks
            .iter()
            .map(|t| t.path.clone())
            .collect();
        self.config.last_index = self.playlist.current;
        self.config.save();
    }
}

fn lyrics_template(track: &Track) -> String {
    let mut text = format!("[ti:{}]\n", track.title);
    if !track.artist.is_empty() {
        text.push_str(&format!("[ar:{}]\n", track.artist));
    }
    if !track.album.is_empty() {
        text.push_str(&format!("[al:{}]\n", track.album));
    }
    text.push('\n');
    text
}

/// 加载与音频同目录同名 .lrc
fn load_lyrics_for(audio: &Path) -> Option<lyrics::Lyrics> {
    let lrc_path = lyrics::sidecar_path(audio);
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
    let data = candidates.iter().find_map(|p| std::fs::read(p).ok());
    let Some(data) = data else {
        eprintln!("警告：未找到系统中文字体（Noto CJK/文泉驿等），中文可能无法显示");
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "cjk".into(),
        std::sync::Arc::new(egui::FontData::from_owned(data)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push("cjk".into());
    }
    ctx.set_fonts(fonts);
}

fn request_mini_window_size(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(MINI_WINDOW_SIZE));
    ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(MINI_WINDOW_SIZE));
    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(MINI_WINDOW_SIZE));
}

impl eframe::App for MusicApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        crate::ui::app_background(self.dark_mode).to_normalized_gamma_f32()
    }

    /// 每帧逻辑（窗口隐藏时也会被调用，适合自动切歌与重绘调度）
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.mini_mode && self.mini_resize_attempts > 0 {
            let reached_target = ctx.input(|input| {
                input.viewport().inner_rect.is_some_and(|rect| {
                    (rect.width() - MINI_WINDOW_SIZE.x).abs() <= 1.0
                        && (rect.height() - MINI_WINDOW_SIZE.y).abs() <= 1.0
                })
            });
            if reached_target {
                self.mini_resize_attempts = 0;
                ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
            } else {
                request_mini_window_size(ctx);
                self.mini_resize_attempts -= 1;
                ctx.request_repaint_after(Duration::from_millis(50));
            }
        }

        // 自动切歌（帧循环轮询，播放中本来就需要刷新进度/歌词）
        if self.engine.as_ref().is_some_and(|e| e.track_finished()) {
            self.on_track_end(ctx);
        }
        if self
            .engine
            .as_ref()
            .is_some_and(|e| e.state == PlaybackState::Playing)
        {
            // 50ms 刷新：进度/歌词 + 列表行均衡器动画
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        crate::ui::draw(self, ui);
    }
}
