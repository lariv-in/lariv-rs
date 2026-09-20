//! Combine per-person staging files with video-rs (FFmpeg muxer). Streams stay
//! uncomposited; each output stream is tagged with `joined_user_id`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use video_rs::ffmpeg::media::Type as AvMediaType;
use video_rs::{Options, Reader, ReaderBuilder};

use crate::plugins::meets::sfu::codec::VideoCodec;

pub fn mux_staging_files(
    tracks: &[(i64, bool, PathBuf)],
    output: &Path,
    codec: VideoCodec,
) -> anyhow::Result<()> {
    if tracks.is_empty() {
        anyhow::bail!("no staging tracks to mux");
    }
    video_rs::init().map_err(|e| anyhow::anyhow!("{e}"))?;
    let format = if codec.is_webm_native() {
        "webm"
    } else {
        "matroska"
    };

    let mut output_ctx = video_rs::ffmpeg::format::output_as(output, format)
        .map_err(|e| anyhow::anyhow!("open mux output: {e}"))?;

    struct Source {
        reader: Reader,
        in_idx: usize,
        out_idx: usize,
    }

    let mut probe = HashMap::new();
    probe.insert("analyzeduration".into(), "10000000".into());
    probe.insert("probesize".into(), "10000000".into());
    let probe = Options::from(probe);

    let mut sources = Vec::new();
    for (joined_user_id, is_video, path) in tracks {
        if !path.exists() {
            continue;
        }
        let reader = match ReaderBuilder::new(path.as_path())
            .with_options(&probe)
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "skip staging file");
                continue;
            }
        };
        let in_idx = if *is_video {
            reader.best_video_stream_index().unwrap_or(0)
        } else {
            reader
                .input
                .streams()
                .best(AvMediaType::Audio)
                .map(|s| s.index())
                .unwrap_or(0)
        };
        let Some(ist) = reader.input.stream(in_idx) else {
            continue;
        };
        let codecpar = ist.parameters();
        let mut ost = output_ctx
            .add_stream(video_rs::ffmpeg::encoder::find(codecpar.id()))
            .map_err(|e| anyhow::anyhow!("add stream: {e}"))?;
        ost.set_parameters(codecpar);
        let kind = if *is_video { "video" } else { "audio" };
        let mut meta = video_rs::ffmpeg::Dictionary::new();
        meta.set("joined_user_id", &joined_user_id.to_string());
        meta.set("title", &format!("joined_user_{joined_user_id}_{kind}"));
        ost.set_metadata(meta);
        let out_idx = ost.index();
        drop(ost);
        sources.push(Source {
            reader,
            in_idx,
            out_idx,
        });
    }

    if sources.is_empty() {
        anyhow::bail!("ffmpeg could not open any staging streams");
    }

    output_ctx
        .write_header()
        .map_err(|e| anyhow::anyhow!("write header: {e}"))?;

    for src in &mut sources {
        loop {
            match src.reader.read(src.in_idx) {
                Ok(packet) => {
                    let mut av = packet.into_inner();
                    av.set_stream(src.out_idx);
                    av.write_interleaved(&mut output_ctx)
                        .map_err(|e| anyhow::anyhow!("write packet: {e}"))?;
                }
                Err(_) => break,
            }
        }
    }

    output_ctx
        .write_trailer()
        .map_err(|e| anyhow::anyhow!("write trailer: {e}"))?;
    Ok(())
}
