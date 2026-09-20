//! Per-participant staging + video-rs mux into one uncomposited container.

mod depacketize;
mod mux;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
use tokio::sync::Mutex as AsyncMutex;

use crate::plugins::filesystem::node::{self, NodeFile};
use crate::plugins::filesystem::storage::DynFilestore;
use crate::plugins::meets::entities::meeting_recording;
use crate::plugins::meets::sfu::codec::VideoCodec;

use depacketize::{AccessUnit, AccessUnitAssembler, FrameKind};
use mux::mux_staging_files;

pub struct Recorder {
    dir: PathBuf,
    code: String,
    video_codec: VideoCodec,
    started_at: DateTime<Utc>,
    created_by_id: i64,
    inner: Mutex<RecorderInner>,
    mux_lock: AsyncMutex<()>,
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
    assembler: AccessUnitAssembler,
    has_keyframe: bool,
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
        })
    }

    /// Returns `true` when a complete video keyframe was written (safe to remux).
    pub fn push_rtp(
        &self,
        joined_user_id: i64,
        is_video: bool,
        timestamp: u32,
        marker: bool,
        payload: &[u8],
    ) -> bool {
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
        let mut wrote_keyframe = false;
        for unit in track.assembler.push(timestamp, marker, payload) {
            if is_video && !unit.is_keyframe && !track.has_keyframe {
                continue;
            }
            let ts = u64::from(unit.timestamp);
            if track.write_frame(&unit, ts).is_err() {
                continue;
            }
            if is_video && unit.is_keyframe && !track.has_keyframe {
                track.has_keyframe = true;
                wrote_keyframe = true;
            }
        }
        wrote_keyframe
    }

    pub fn staging_paths(&self) -> Vec<(i64, bool, PathBuf)> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (id, t) in &inner.video {
            if t.frame_count > 0 {
                out.push((*id, true, t.path.clone()));
            }
        }
        for (id, t) in &inner.audio {
            if t.frame_count > 0 {
                out.push((*id, false, t.path.clone()));
            }
        }
        out
    }

    pub async fn remux_now(&self) -> anyhow::Result<PathBuf> {
        let _guard = self.mux_lock.lock().await;
        self.flush_writers();
        let paths = self.staging_paths();
        let ext = if self.video_codec.is_webm_native() {
            "webm"
        } else {
            "mkv"
        };
        let out = self.dir.join(format!("combined.{ext}"));
        mux_staging_files(&paths, &out, self.video_codec)?;
        Ok(out)
    }

    fn flush_writers(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            for t in inner.video.values_mut() {
                t.flush_pending();
                let _ = t.writer.flush();
            }
            for t in inner.audio.values_mut() {
                t.flush_pending();
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
        let combined = self.remux_now().await?;
        let bytes = tokio::fs::read(&combined).await?;
        let filename = combined
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("{}.webm", self.code));
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
            filename,
            false,
            Some(NodeFile::Bytes {
                filename: combined
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "recording.webm".into()),
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
            assembler: AccessUnitAssembler::new(is_video, codec),
            has_keyframe: !is_video,
        })
    }

    fn write_frame(&mut self, unit: &AccessUnit, timestamp: u64) -> std::io::Result<()> {
        match self.kind {
            FrameKind::Ivf => {
                let size = u32::try_from(unit.payload.len()).unwrap_or(u32::MAX);
                self.writer.write_all(&size.to_le_bytes())?;
                self.writer.write_all(&timestamp.to_le_bytes())?;
                self.writer.write_all(&unit.payload)?;
                self.frame_count = self.frame_count.saturating_add(1);
            }
            FrameKind::AnnexB => {
                self.writer.write_all(&unit.payload)?;
                self.frame_count = self.frame_count.saturating_add(1);
            }
            FrameKind::OpusDump => {
                let size = u32::try_from(unit.payload.len()).unwrap_or(u32::MAX);
                self.writer.write_all(&size.to_le_bytes())?;
                self.writer.write_all(&timestamp.to_le_bytes())?;
                self.writer.write_all(&unit.payload)?;
                self.frame_count = self.frame_count.saturating_add(1);
            }
        }
        Ok(())
    }

    fn flush_pending(&mut self) {
        if let Some(unit) = self.assembler.take() {
            if unit.payload.is_empty() {
                return;
            }
            if self.kind != FrameKind::OpusDump && !unit.is_keyframe && !self.has_keyframe {
                return;
            }
            let ts = u64::from(unit.timestamp);
            if self.write_frame(&unit, ts).is_ok() && unit.is_keyframe {
                self.has_keyframe = true;
            }
        }
    }
}

fn write_ivf_header(w: &mut impl Write, codec: VideoCodec) -> std::io::Result<()> {
    w.write_all(b"DKIF")?;
    w.write_all(&0u16.to_le_bytes())?; // version
    w.write_all(&32u16.to_le_bytes())?; // header size
    w.write_all(&codec.fourcc())?;
    w.write_all(&1280u16.to_le_bytes())?;
    w.write_all(&720u16.to_le_bytes())?;
    w.write_all(&90000u32.to_le_bytes())?; // timebase den (RTP clock)
    w.write_all(&1u32.to_le_bytes())?; // timebase num
    w.write_all(&0u32.to_le_bytes())?; // frame count (unknown)
    w.write_all(&0u32.to_le_bytes())?;
    Ok(())
}
