//! Video codec helpers for meeting recordings.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VideoCodec {
    Av1,
    H265,
    Vp9,
    Vp8,
    H264,
}

impl VideoCodec {
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Av1 => "AV1",
            Self::H265 => "H265",
            Self::Vp9 => "VP9",
            Self::Vp8 => "VP8",
            Self::H264 => "H264",
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
        let upper = s.trim().to_ascii_uppercase();
        if upper.contains("AV1") || upper.contains("AV01") {
            Some(Self::Av1)
        } else if upper.contains("H265") || upper.contains("HEVC") {
            Some(Self::H265)
        } else if upper.contains("VP9") || upper.contains("VP09") {
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
