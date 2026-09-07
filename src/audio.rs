//! 音频引擎：rodio 0.22 新 API（MixerDeviceSink + Player）封装。
//!
//! 设计要点：
//! * 应用层手动管理"当前单曲"（一次 append 一首），播放模式由 `Playlist` 决定
//! * `Player::get_pos/try_seek/empty` 已内建，无需自定义 Source
//! * `append()` 前先解码校验，失败不影响正在播放的曲目
//! * `stop()` 后 `append()` 会自动恢复（rodio 内部处理，见 player.rs append 实现）

use std::fs::File;
use std::path::Path;
use std::time::{Duration, Instant};

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

pub struct AudioEngine {
    /// 持有设备 sink，drop 即停止播放，必须保活
    _device: MixerDeviceSink,
    player: Player,
    pub state: PlaybackState,
    /// 最近一次开始播放的时间：append 与 empty() 生效之间可能存在短暂竞态，
    /// 启动后一小段时间内不判定"播完"
    started_at: Instant,
}

impl AudioEngine {
    pub fn new() -> Result<Self, String> {
        let device = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| format!("无法打开音频设备: {e}"))?;
        let player = Player::connect_new(&device.mixer());
        Ok(Self {
            _device: device,
            player,
            state: PlaybackState::Stopped,
            started_at: Instant::now(),
        })
    }

    /// 播放一个文件（先解码校验，成功后才切换）
    pub fn play_file(&mut self, path: &Path) -> Result<(), String> {
        let file =
            File::open(path).map_err(|e| format!("无法打开文件 {}: {e}", path.display()))?;
        let src =
            Decoder::try_from(file).map_err(|e| format!("无法解码 {}: {e}", path.display()))?;
        self.player.stop();
        self.player.append(src);
        self.player.play();
        self.state = PlaybackState::Playing;
        self.started_at = Instant::now();
        Ok(())
    }

    /// 重播当前曲（seek 到 0 并继续；若已停止则无效，由调用方重新 play_file）
    pub fn replay(&mut self) {
        if self.state != PlaybackState::Stopped {
            let _ = self.player.try_seek(Duration::ZERO);
            self.player.play();
            self.state = PlaybackState::Playing;
            self.started_at = Instant::now();
        }
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.player.pause();
            self.state = PlaybackState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == PlaybackState::Paused {
            self.player.play();
            self.state = PlaybackState::Playing;
        }
    }

    pub fn stop(&mut self) {
        self.player.stop();
        self.state = PlaybackState::Stopped;
    }

    pub fn seek(&self, pos: Duration) {
        if self.state != PlaybackState::Stopped {
            let _ = self.player.try_seek(pos);
        }
    }

    /// 当前播放位置
    pub fn position(&self) -> Duration {
        match self.state {
            PlaybackState::Stopped => Duration::ZERO,
            _ => self.player.get_pos(),
        }
    }

    pub fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume);
    }

    /// 当前曲目是否自然播完（仅在播放态判定）
    pub fn track_finished(&self) -> bool {
        self.state == PlaybackState::Playing
            && self.started_at.elapsed() > Duration::from_millis(300)
            && self.player.empty()
    }
}
