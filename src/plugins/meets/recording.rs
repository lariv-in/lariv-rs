//! Per-participant staging + video-rs mux into one uncomposited container.

mod codec;
mod mux;
pub mod room_subscriber;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Datelike, Timelike, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
use tokio::sync::Mutex as AsyncMutex;

use crate::plugins::filesystem::node::{self, NodeFile};
use crate::plugins::filesystem::storage::DynFilestore;
use crate::plugins::meets::entities::meeting_recording;
use crate::plugins::meets::recording::codec::VideoCodec;

use mux::mux_staging_files;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameKind {
    Ivf,
    AnnexB,
    OpusDump,
}

pub struct Recorder {
    dir: PathBuf,
    code: String,
    video_codec: VideoCodec,
    started_at: DateTime<Utc>,
    created_by_id: i64,
    inner: Mutex<RecorderInner>,
    mux_lock: AsyncMutex<()>,
    finishing: AtomicBool,
}

struct RecorderInner {
    video: HashMap<i64, StagingTrack>,
    audio: HashMap<i64, StagingTrack>,
}

struct StagingTrack {
    path: PathBuf,
    writer: BufWriter<File>,
    kind: FrameKind,
    frame_count: u32,
    has_keyframe: bool,
    base_ts: Option<u64>,
}

impl Recorder {
    pub fn start(code: &str, video_codec: VideoCodec, created_by_id: i64) -> std::io::Result<Self> {
        let dir = std::env::temp_dir().join("lariv-meets").join(code);
        fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            code: code.to_string(),
            video_codec,
            started_at: Utc::now(),
            created_by_id,
            inner: Mutex::new(RecorderInner {
                video: HashMap::new(),
                audio: HashMap::new(),
            }),
            mux_lock: AsyncMutex::new(()),
            finishing: AtomicBool::new(false),
        })
    }

    pub fn mark_finishing(&self) {
        self.finishing.store(true, Ordering::Release);
    }

    pub fn push_frame(
        &self,
        joined_user_id: i64,
        is_video: bool,
        timestamp_us: u64,
        is_keyframe: bool,
        payload: &[u8],
    ) -> bool {
        if self.finishing.load(Ordering::Acquire) || payload.is_empty() {
            return false;
        }
        let Ok(mut inner) = self.inner.lock() else {
            return false;
        };
        let map = if is_video {
            &mut inner.video
        } else {
            &mut inner.audio
        };
        if !map.contains_key(&joined_user_id) {
            let Ok(track) =
                StagingTrack::create(&self.dir, joined_user_id, is_video, self.video_codec)
            else {
                return false;
            };
            map.insert(joined_user_id, track);
        }
        let Some(track) = map.get_mut(&joined_user_id) else {
            return false;
        };
        if is_video && !is_keyframe && !track.has_keyframe {
            return false;
        }
        if track.write_frame(payload, timestamp_us).is_err() {
            return false;
        }
        if is_video && is_keyframe && !track.has_keyframe {
            track.has_keyframe = true;
            return true;
        }
        false
    }

    pub async fn remux_now(
        &self,
        participant_names: &HashMap<i64, String>,
    ) -> anyhow::Result<PathBuf> {
        let _guard = self.mux_lock.lock().await;
        let (snap_dir, paths) = self.snapshot_staging()?;
        let ext = if self.video_codec.is_webm_native() {
            "webm"
        } else {
            "mkv"
        };
        let out = self.dir.join(format!("combined.{ext}"));
        let result = mux_staging_files(&paths, &out, self.video_codec, participant_names);
        let _ = fs::remove_dir_all(&snap_dir);
        result?;
        Ok(out)
    }

    fn snapshot_staging(&self) -> std::io::Result<(PathBuf, Vec<(i64, bool, PathBuf)>)> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| std::io::Error::other("recorder lock poisoned"))?;
        let snap_dir = self.dir.join(format!(
            "snap-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&snap_dir);
        fs::create_dir_all(&snap_dir)?;
        let mut paths = Vec::new();
        for (id, track) in &mut inner.video {
            snapshot_track(*id, true, track, &snap_dir, &mut paths)?;
        }
        for (id, track) in &mut inner.audio {
            snapshot_track(*id, false, track, &snap_dir, &mut paths)?;
        }
        Ok((snap_dir, paths))
    }

    fn flush_writers(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            for t in inner.video.values_mut() {
                let _ = t.writer.flush();
            }
            for t in inner.audio.values_mut() {
                let _ = t.writer.flush();
            }
        }
    }

    pub async fn finalize_vnode(
        &self,
        db: &DatabaseConnection,
        store: &DynFilestore,
        conference_room_code: &str,
    ) -> anyhow::Result<i64> {
        self.mark_finishing();
        self.flush_writers();
        let joined_ids = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| anyhow::anyhow!("recorder lock poisoned"))?;
            inner
                .video
                .keys()
                .chain(inner.audio.keys())
                .copied()
                .collect::<Vec<_>>()
        };
        let participant_names =
            crate::plugins::meets::logic::join::participant_names_for_ids(db, joined_ids)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        let combined = self.remux_now(&participant_names).await?;
        let bytes = tokio::fs::read(&combined).await?;
        let ext = if self.video_codec.is_webm_native() {
            "webm"
        } else {
            "mkv"
        };
        let filename = recording_vnode_name(self.started_at, ext);
        let parent = node::ensure_directory_path(
            db,
            store,
            None,
            &[
                "meets".to_string(),
                "recordings".to_string(),
                self.code.clone(),
            ],
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        let parent_node = match parent {
            Some(id) => node::get_by_id(db, id)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?,
            None => None,
        };
        let vnode = node::create(
            db,
            store,
            filename.clone(),
            false,
            Some(NodeFile::Bytes {
                filename,
                data: bytes,
            }),
            parent_node.as_ref(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        if vnode.is_directory {
            anyhow::bail!("recording vnode must be a file");
        }
        let rec = meeting_recording::ActiveModel {
            conference_room_code: Set(conference_room_code.to_string()),
            meeting_start_at: Set(self.started_at),
            meeting_created_by_id: Set(self.created_by_id),
            video_recording_id: Set(vnode.id),
            transcript: Set(String::new()),
            ..Default::default()
        };
        rec.insert(db).await?;
        Ok(vnode.id)
    }
}

fn snapshot_track(
    id: i64,
    is_video: bool,
    track: &mut StagingTrack,
    snap_dir: &Path,
    paths: &mut Vec<(i64, bool, PathBuf)>,
) -> std::io::Result<()> {
    if track.frame_count == 0 {
        return Ok(());
    }
    track.writer.flush()?;
    track.writer.get_ref().sync_all()?;
    let name = track.path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "staging path has no filename",
        )
    })?;
    let dst = snap_dir.join(name);
    fs::copy(&track.path, &dst)?;
    paths.push((id, is_video, dst));
    Ok(())
}

impl StagingTrack {
    fn create(
        dir: &Path,
        joined_user_id: i64,
        is_video: bool,
        codec: VideoCodec,
    ) -> std::io::Result<Self> {
        let (kind, name) = if is_video {
            match codec {
                VideoCodec::H264 | VideoCodec::H265 => {
                    (FrameKind::AnnexB, format!("{joined_user_id}_video.h264"))
                }
                _ => (FrameKind::Ivf, format!("{joined_user_id}_video.ivf")),
            }
        } else {
            (
                FrameKind::OpusDump,
                format!("{joined_user_id}_audio.opusdump"),
            )
        };
        let path = dir.join(name);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        if kind == FrameKind::Ivf {
            write_ivf_header(&mut writer, codec)?;
        }
        Ok(Self {
            path,
            writer,
            kind,
            frame_count: 0,
            has_keyframe: !is_video,
            base_ts: None,
        })
    }

    fn write_frame(&mut self, payload: &[u8], timestamp_us: u64) -> std::io::Result<()> {
        let ts = self.normalize_ts(timestamp_us);
        match self.kind {
            FrameKind::Ivf => {
                let size = u32::try_from(payload.len()).unwrap_or(u32::MAX);
                self.writer.write_all(&size.to_le_bytes())?;
                self.writer.write_all(&(ts as u32).to_le_bytes())?;
                self.writer.write_all(payload)?;
                self.frame_count = self.frame_count.saturating_add(1);
            }
            FrameKind::AnnexB => {
                self.writer.write_all(payload)?;
                self.frame_count = self.frame_count.saturating_add(1);
            }
            FrameKind::OpusDump => {
                let size = u32::try_from(payload.len()).unwrap_or(u32::MAX);
                self.writer.write_all(&size.to_le_bytes())?;
                self.writer.write_all(&(ts as u32).to_le_bytes())?;
                self.writer.write_all(payload)?;
                self.frame_count = self.frame_count.saturating_add(1);
            }
        }
        Ok(())
    }

    fn normalize_ts(&mut self, raw: u64) -> u64 {
        match self.base_ts {
            None => {
                self.base_ts = Some(raw);
                0
            }
            Some(base) => raw.saturating_sub(base),
        }
    }
}

fn recording_vnode_name(started_at: DateTime<Utc>, ext: &str) -> String {
    format!(
        "recording-{:04}{:02}{:02}-{:02}{:02}{:02}-{:03}.{ext}",
        started_at.year(),
        started_at.month(),
        started_at.day(),
        started_at.hour(),
        started_at.minute(),
        started_at.second(),
        started_at.timestamp_subsec_millis(),
    )
}

fn write_ivf_header(w: &mut impl Write, codec: VideoCodec) -> std::io::Result<()> {
    w.write_all(b"DKIF")?;
    w.write_all(&0u16.to_le_bytes())?;
    w.write_all(&32u16.to_le_bytes())?;
    w.write_all(&codec.fourcc())?;
    w.write_all(&1280u16.to_le_bytes())?;
    w.write_all(&720u16.to_le_bytes())?;
    w.write_all(&90000u32.to_le_bytes())?;
    w.write_all(&1u32.to_le_bytes())?;
    w.write_all(&0u32.to_le_bytes())?;
    w.write_all(&0u32.to_le_bytes())?;
    Ok(())
}
