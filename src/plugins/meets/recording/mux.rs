//! Combine per-person staging files with video-rs (FFmpeg muxer). Streams stay
//! uncomposited; each output stream is tagged with `joined_user_id` and titled
//! with the participant's display name.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use video_rs::ffmpeg::media::Type as AvMediaType;
use video_rs::ffmpeg::util::channel_layout::ChannelLayout;
use video_rs::ffmpeg::{self, Rational};
use video_rs::{Options, Reader, ReaderBuilder};

use crate::plugins::meets::recording::codec::VideoCodec;

const OPUS_RTP_CLOCK: Rational = Rational(1, 48_000);

pub fn mux_staging_files(
    tracks: &[(i64, bool, PathBuf)],
    output: &Path,
    codec: VideoCodec,
    participant_names: &HashMap<i64, String>,
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

    struct MediaSource {
        reader: Reader,
        in_idx: usize,
        out_idx: usize,
    }

    struct OpusSource {
        frames: Vec<(i64, Vec<u8>)>,
        out_idx: usize,
    }

    enum Source {
        Media(MediaSource),
        Opus(OpusSource),
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
        if !*is_video {
            match read_opusdump(path) {
                Ok(frames) if !frames.is_empty() => {
                    let out_idx = add_opus_stream(
                        &mut output_ctx,
                        *joined_user_id,
                        participant_names,
                        &frames,
                    )?;
                    sources.push(Source::Opus(OpusSource { frames, out_idx }));
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "skip opus staging file");
                }
            }
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
        let in_idx = reader.best_video_stream_index().unwrap_or(0);
        if reader.input.stream(in_idx).is_none() {
            continue;
        }
        let codecpar = reader.input.stream(in_idx).unwrap().parameters();
        let mut ost = output_ctx
            .add_stream(video_rs::ffmpeg::encoder::find(codecpar.id()))
            .map_err(|e| anyhow::anyhow!("add stream: {e}"))?;
        ost.set_parameters(codecpar);
        clear_codec_tag(&mut ost);
        let mut meta = video_rs::ffmpeg::Dictionary::new();
        meta.set("joined_user_id", &joined_user_id.to_string());
        meta.set(
            "title",
            &stream_title(*joined_user_id, participant_names, "video"),
        );
        ost.set_metadata(meta);
        let out_idx = ost.index();
        drop(ost);
        sources.push(Source::Media(MediaSource {
            reader,
            in_idx,
            out_idx,
        }));
    }

    if sources.is_empty() {
        anyhow::bail!("ffmpeg could not open any staging streams");
    }

    output_ctx
        .write_header()
        .map_err(|e| anyhow::anyhow!("write header: {e}"))?;

    for src in &mut sources {
        match src {
            Source::Media(src) => {
                let out_time_base = output_ctx
                    .stream(src.out_idx)
                    .map(|s| s.time_base())
                    .unwrap_or(Rational(1, 1000));
                loop {
                    match src.reader.read(src.in_idx) {
                        Ok(packet) => {
                            let (mut av, in_tb) = packet.into_inner_parts();
                            av.set_stream(src.out_idx);
                            av.rescale_ts(in_tb, out_time_base);
                            av.set_position(-1);
                            av.write_interleaved(&mut output_ctx)
                                .map_err(|e| anyhow::anyhow!("write packet: {e}"))?;
                        }
                        Err(_) => break,
                    }
                }
            }
            Source::Opus(src) => {
                let out_time_base = output_ctx
                    .stream(src.out_idx)
                    .map(|s| s.time_base())
                    .unwrap_or(Rational(1, 1000));
                for (ts, payload) in &src.frames {
                    let mut packet = ffmpeg::Packet::copy(payload);
                    packet.set_stream(src.out_idx);
                    packet.set_pts(Some(*ts));
                    packet.set_dts(Some(*ts));
                    packet.set_duration(opus_frame_samples(payload));
                    packet.rescale_ts(OPUS_RTP_CLOCK, out_time_base);
                    packet.set_position(-1);
                    packet
                        .write_interleaved(&mut output_ctx)
                        .map_err(|e| anyhow::anyhow!("write opus packet: {e}"))?;
                }
            }
        }
    }

    output_ctx
        .write_trailer()
        .map_err(|e| anyhow::anyhow!("write trailer: {e}"))?;
    Ok(())
}

fn add_opus_stream(
    output_ctx: &mut ffmpeg::format::context::Output,
    joined_user_id: i64,
    participant_names: &HashMap<i64, String>,
    frames: &[(i64, Vec<u8>)],
) -> anyhow::Result<usize> {
    let (channels, _) = opus_audio_params(&frames[0].1);
    let mut ost = output_ctx
        .add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::OPUS))
        .map_err(|e| anyhow::anyhow!("add opus stream: {e}"))?;
    let mut par = ffmpeg::codec::Parameters::new();
    par.set_medium(AvMediaType::Audio);
    par.set_id(ffmpeg::codec::Id::OPUS);
    set_opus_codecpar(&mut par, channels);
    set_extradata(&mut par, &opus_head(channels));
    ost.set_parameters(par);
    // WebM expects millisecond timestamps for Opus.
    ost.set_time_base(Rational(1, 1000));
    clear_codec_tag(&mut ost);
    let mut meta = ffmpeg::Dictionary::new();
    meta.set("joined_user_id", &joined_user_id.to_string());
    meta.set(
        "title",
        &stream_title(joined_user_id, participant_names, "audio"),
    );
    ost.set_metadata(meta);
    Ok(ost.index())
}

fn stream_title(id: i64, names: &HashMap<i64, String>, kind: &str) -> String {
    let name = names.get(&id).map(String::as_str).unwrap_or("participant");
    format!("{name} ({kind})")
}

fn read_opusdump(path: &Path) -> anyhow::Result<Vec<(i64, Vec<u8>)>> {
    let mut file = File::open(path)?;
    let mut out = Vec::new();
    loop {
        let mut size_buf = [0u8; 4];
        if file.read_exact(&mut size_buf).is_err() {
            break;
        }
        let size = u32::from_le_bytes(size_buf) as usize;
        let mut ts_buf = [0u8; 8];
        file.read_exact(&mut ts_buf)
            .map_err(|e| anyhow::anyhow!("opusdump timestamp: {e}"))?;
        let ts = i64::try_from(u64::from_le_bytes(ts_buf)).unwrap_or(0);
        let mut payload = vec![0u8; size];
        if file.read_exact(&mut payload).is_err() {
            break;
        }
        out.push((ts, payload));
    }
    Ok(out)
}

fn opus_head(channels: u8) -> Vec<u8> {
    let mut head = b"OpusHead".to_vec();
    head.push(1); // version
    head.push(channels);
    head.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
    head.extend_from_slice(&48_000u32.to_le_bytes());
    head.extend_from_slice(&0i16.to_le_bytes()); // output gain
    head.push(0); // channel mapping family
    head
}

fn set_opus_codecpar(par: &mut ffmpeg::codec::Parameters, channels: u8) {
    unsafe {
        let ptr = par.as_mut_ptr();
        (*ptr).sample_rate = 48_000;
        let layout = if channels <= 1 {
            ChannelLayout::MONO
        } else {
            ChannelLayout::STEREO
        };
        (*ptr).ch_layout = layout.into();
    }
}

fn set_extradata(par: &mut ffmpeg::codec::Parameters, data: &[u8]) {
    unsafe {
        use ffmpeg::ffi::{AV_INPUT_BUFFER_PADDING_SIZE, av_free, av_malloc};
        let ptr = par.as_mut_ptr();
        if !(*ptr).extradata.is_null() {
            av_free((*ptr).extradata as _);
            (*ptr).extradata = std::ptr::null_mut();
            (*ptr).extradata_size = 0;
        }
        let alloc = av_malloc(data.len() + AV_INPUT_BUFFER_PADDING_SIZE as usize);
        if alloc.is_null() {
            return;
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), alloc as *mut u8, data.len());
        (*ptr).extradata = alloc as *mut u8;
        (*ptr).extradata_size = data.len() as i32;
    }
}

fn clear_codec_tag(ost: &mut ffmpeg::format::stream::StreamMut<'_>) {
    unsafe {
        (*ost.parameters().as_mut_ptr()).codec_tag = 0;
    }
}

fn opus_audio_params(payload: &[u8]) -> (u8, i64) {
    if payload.is_empty() {
        return (2, 960);
    }
    let config = payload[0] >> 3;
    let channels = if config < 16 { 1 } else { 2 };
    let samples = match config {
        0..=3 | 16..=19 => 480,
        4..=7 | 20..=23 => 960,
        8..=11 | 24..=27 => 1920,
        12..=15 | 28..=31 => 2880,
        _ => 960,
    };
    (channels, samples)
}

fn opus_frame_samples(payload: &[u8]) -> i64 {
    opus_audio_params(payload).1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn opusdump_roundtrip() {
        let dir = std::env::temp_dir().join("lariv-meets-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("1_audio.opusdump");
        let mut f = File::create(&path).unwrap();
        let payload = [0x48, 0x01, 0x02];
        f.write_all(&(payload.len() as u32).to_le_bytes()).unwrap();
        f.write_all(&9000u64.to_le_bytes()).unwrap();
        f.write_all(&payload).unwrap();
        let frames = read_opusdump(&path).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].0, 9000);
        assert_eq!(frames[0].1, payload);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn opus_head_has_magic() {
        let head = opus_head(2);
        assert_eq!(&head[..8], b"OpusHead");
        assert_eq!(head[8], 1);
        assert_eq!(head[9], 2);
    }

    #[test]
    fn mux_opusdump_to_webm_header() {
        let dir = std::env::temp_dir().join("lariv-meets-test-mux");
        let _ = std::fs::create_dir_all(&dir);
        let audio = dir.join("1_audio.opusdump");
        let out = dir.join("combined.webm");
        let mut f = File::create(&audio).unwrap();
        let payload = [0x48, 0x01, 0x02];
        f.write_all(&(payload.len() as u32).to_le_bytes()).unwrap();
        f.write_all(&0u64.to_le_bytes()).unwrap();
        f.write_all(&payload).unwrap();
        drop(f);
        let tracks = vec![(1_i64, false, audio.clone())];
        let names = HashMap::from([(1_i64, "Alice".into())]);
        mux_staging_files(&tracks, &out, VideoCodec::Av1, &names).expect("mux opus to webm");
        assert!(out.exists());
        let _ = std::fs::remove_file(audio);
        let _ = std::fs::remove_file(out);
    }
}
