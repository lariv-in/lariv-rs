//! Video codec priority: AV1 > H.265 > VP9 > VP8 > H.264. Audio is Opus-only.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VideoCodec {
    Av1,
    H265,
    Vp9,
    Vp8,
    H264,
}

pub const VIDEO_PRIORITY: &[VideoCodec] = &[
    VideoCodec::Av1,
    VideoCodec::H265,
    VideoCodec::Vp9,
    VideoCodec::Vp8,
    VideoCodec::H264,
];

impl VideoCodec {
    pub fn as_sdp(self) -> &'static str {
        match self {
            Self::Av1 => "AV1",
            Self::H265 => "H265",
            Self::Vp9 => "VP9",
            Self::Vp8 => "VP8",
            Self::H264 => "H264",
        }
    }

    pub fn mime_types(self) -> &'static [&'static str] {
        match self {
            Self::Av1 => &["video/AV1", "video/av1"],
            Self::H265 => &["video/H265", "video/h265", "video/HEVC", "video/hevc"],
            Self::Vp9 => &["video/VP9", "video/vp9"],
            Self::Vp8 => &["video/VP8", "video/vp8"],
            Self::H264 => &["video/H264", "video/h264"],
        }
    }

    pub fn fourcc(self) -> [u8; 4] {
        match self {
            Self::Av1 => *b"AV01",
            Self::H265 => *b"H265",
            Self::Vp9 => *b"VP90",
            Self::Vp8 => *b"VP80",
            Self::H264 => *b"H264",
        }
    }

    pub fn is_webm_native(self) -> bool {
        matches!(self, Self::Av1 | Self::Vp9 | Self::Vp8)
    }

    pub fn parse(s: &str) -> Option<Self> {
        let t = s.trim();
        let upper = t.to_ascii_uppercase();
        if upper.contains("AV1") {
            Some(Self::Av1)
        } else if upper.contains("H265") || upper.contains("HEVC") {
            Some(Self::H265)
        } else if upper.contains("VP9") {
            Some(Self::Vp9)
        } else if upper.contains("VP8") {
            Some(Self::Vp8)
        } else if upper.contains("H264") || upper.contains("AVC") {
            Some(Self::H264)
        } else {
            None
        }
    }
}

pub fn parse_codec_list(names: &[String]) -> HashSet<VideoCodec> {
    names.iter().filter_map(|s| VideoCodec::parse(s)).collect()
}

/// Highest-priority codec present in every participant's capability set.
pub fn negotiate(participants: &[HashSet<VideoCodec>]) -> Option<VideoCodec> {
    if participants.is_empty() {
        return None;
    }
    let mut intersection = participants[0].clone();
    for set in participants.iter().skip(1) {
        intersection = intersection.intersection(set).copied().collect();
    }
    VIDEO_PRIORITY
        .iter()
        .copied()
        .find(|c| intersection.contains(c))
}

pub fn video_codec_priority_json() -> String {
    serde_json::to_string(
        &VIDEO_PRIORITY
            .iter()
            .map(|c| c.as_sdp())
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| r#"["AV1","H265","VP9","VP8","H264"]"#.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_av1_when_all_support_it() {
        let a = HashSet::from([VideoCodec::Av1, VideoCodec::Vp8, VideoCodec::H264]);
        let b = HashSet::from([VideoCodec::Av1, VideoCodec::Vp9, VideoCodec::H264]);
        assert_eq!(negotiate(&[a, b]), Some(VideoCodec::Av1));
    }

    #[test]
    fn falls_back_when_new_joiner_lacks_av1() {
        let a = HashSet::from([VideoCodec::Av1, VideoCodec::Vp8]);
        let b = HashSet::from([VideoCodec::Vp8, VideoCodec::H264]);
        assert_eq!(negotiate(&[a, b]), Some(VideoCodec::Vp8));
    }

    #[test]
    fn none_when_disjoint() {
        let a = HashSet::from([VideoCodec::Av1]);
        let b = HashSet::from([VideoCodec::H264]);
        assert_eq!(negotiate(&[a, b]), None);
    }
}
