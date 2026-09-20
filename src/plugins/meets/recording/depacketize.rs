//! RTP payload → access units. AV1/VP8/VP9/H26x descriptors are stripped and
//! fragments are reassembled before anything is written to a staging file.

use crate::plugins::meets::sfu::codec::VideoCodec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameKind {
    Ivf,
    AnnexB,
    OpusDump,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessUnit {
    pub payload: Vec<u8>,
    pub timestamp: u32,
    pub is_keyframe: bool,
}

pub struct AccessUnitAssembler {
    is_video: bool,
    codec: VideoCodec,
    ts: Option<u32>,
    buf: Vec<u8>,
    keyframe: bool,
    av1_fragment: Vec<u8>,
    h26x_fu: Vec<u8>,
}

impl AccessUnitAssembler {
    pub fn new(is_video: bool, codec: VideoCodec) -> Self {
        Self {
            is_video,
            codec,
            ts: None,
            buf: Vec::new(),
            keyframe: false,
            av1_fragment: Vec::new(),
            h26x_fu: Vec::new(),
        }
    }

    /// Push one RTP payload. Returns zero or more complete access units.
    pub fn push(&mut self, timestamp: u32, marker: bool, payload: &[u8]) -> Vec<AccessUnit> {
        let mut out = Vec::new();
        if payload.is_empty() {
            return out;
        }
        if let Some(prev) = self.ts
            && prev != timestamp
            && let Some(unit) = self.flush()
        {
            out.push(unit);
        }
        self.ts = Some(timestamp);
        if self.is_video {
            match self.codec {
                VideoCodec::Av1 => self.push_av1(payload),
                VideoCodec::Vp8 => self.push_vp8(payload),
                VideoCodec::Vp9 => self.push_vp9(payload),
                VideoCodec::H264 => self.push_h264(payload),
                VideoCodec::H265 => self.push_h265(payload),
            }
        } else {
            self.buf.extend_from_slice(payload);
        }
        let complete = if self.is_video { marker } else { true };
        if complete && let Some(unit) = self.flush() {
            out.push(unit);
        }
        out
    }

    fn flush(&mut self) -> Option<AccessUnit> {
        if self.buf.is_empty() {
            self.ts = None;
            self.keyframe = false;
            return None;
        }
        Some(AccessUnit {
            payload: std::mem::take(&mut self.buf),
            timestamp: self.ts.take().unwrap_or(0),
            is_keyframe: std::mem::replace(&mut self.keyframe, false),
        })
    }

    /// Force-complete a unit (end of stream / remux).
    pub fn take(&mut self) -> Option<AccessUnit> {
        self.flush()
    }

    fn push_av1(&mut self, payload: &[u8]) {
        let Some((bytes, n)) = depacketize_av1(payload, &mut self.av1_fragment) else {
            return;
        };
        self.keyframe |= n || av1_has_sequence_header(&bytes);
        self.buf.extend_from_slice(&bytes);
    }

    fn push_vp8(&mut self, payload: &[u8]) {
        let Some((rest, _start, key)) = strip_vp8(payload) else {
            return;
        };
        self.keyframe |= key;
        self.buf.extend_from_slice(rest);
    }

    fn push_vp9(&mut self, payload: &[u8]) {
        let Some((rest, _begin, key)) = strip_vp9(payload) else {
            return;
        };
        self.keyframe |= key;
        self.buf.extend_from_slice(rest);
    }

    fn push_h264(&mut self, payload: &[u8]) {
        if payload.len() < 2 {
            return;
        }
        let nalu_type = payload[0] & 0x1f;
        match nalu_type {
            1..=23 => {
                self.keyframe |= matches!(nalu_type, 5 | 7);
                annexb(&mut self.buf, payload);
            }
            24 => {
                // STAP-A
                let mut off = 1;
                while off + 2 <= payload.len() {
                    let nalu_size = u16::from_be_bytes([payload[off], payload[off + 1]]) as usize;
                    off += 2;
                    if off + nalu_size > payload.len() {
                        break;
                    }
                    let nalu = &payload[off..off + nalu_size];
                    if !nalu.is_empty() {
                        let t = nalu[0] & 0x1f;
                        self.keyframe |= matches!(t, 5 | 7);
                        annexb(&mut self.buf, nalu);
                    }
                    off += nalu_size;
                }
            }
            28 => {
                // FU-A
                if payload.len() < 2 {
                    return;
                }
                let fu = payload[1];
                let start = fu & 0x80 != 0;
                let end = fu & 0x40 != 0;
                if start {
                    self.h26x_fu.clear();
                    let reconstructed = (payload[0] & 0xe0) | (fu & 0x1f);
                    self.h26x_fu.push(reconstructed);
                }
                if self.h26x_fu.is_empty() && !start {
                    return;
                }
                self.h26x_fu.extend_from_slice(&payload[2..]);
                if end {
                    let nalu = std::mem::take(&mut self.h26x_fu);
                    if let Some(&hdr) = nalu.first() {
                        self.keyframe |= matches!(hdr & 0x1f, 5 | 7);
                    }
                    annexb(&mut self.buf, &nalu);
                }
            }
            _ => {}
        }
    }

    fn push_h265(&mut self, payload: &[u8]) {
        if payload.len() < 2 {
            return;
        }
        let nalu_type = (payload[0] >> 1) & 0x3f;
        match nalu_type {
            1..=47 => {
                self.keyframe |= matches!(nalu_type, 19 | 20 | 21 | 32 | 33);
                annexb(&mut self.buf, payload);
            }
            48 => {
                // Aggregation packet
                let mut off = 2;
                while off + 2 <= payload.len() {
                    let nalu_size = u16::from_be_bytes([payload[off], payload[off + 1]]) as usize;
                    off += 2;
                    if off + nalu_size > payload.len() {
                        break;
                    }
                    let nalu = &payload[off..off + nalu_size];
                    if nalu.len() >= 2 {
                        let t = (nalu[0] >> 1) & 0x3f;
                        self.keyframe |= matches!(t, 19 | 20 | 21 | 32 | 33);
                        annexb(&mut self.buf, nalu);
                    }
                    off += nalu_size;
                }
            }
            49 => {
                // FU
                if payload.len() < 3 {
                    return;
                }
                let fu = payload[2];
                let start = fu & 0x80 != 0;
                let end = fu & 0x40 != 0;
                if start {
                    self.h26x_fu.clear();
                    let nal0 = (payload[0] & 0x81) | ((fu & 0x3f) << 1);
                    self.h26x_fu.push(nal0);
                    self.h26x_fu.push(payload[1]);
                }
                if self.h26x_fu.is_empty() && !start {
                    return;
                }
                self.h26x_fu.extend_from_slice(&payload[3..]);
                if end {
                    let nalu = std::mem::take(&mut self.h26x_fu);
                    if nalu.len() >= 2 {
                        let t = (nalu[0] >> 1) & 0x3f;
                        self.keyframe |= matches!(t, 19 | 20 | 21 | 32 | 33);
                    }
                    annexb(&mut self.buf, &nalu);
                }
            }
            _ => {}
        }
    }
}

fn annexb(out: &mut Vec<u8>, nalu: &[u8]) {
    out.extend_from_slice(&[0, 0, 0, 1]);
    out.extend_from_slice(nalu);
}

fn read_leb128(data: &[u8]) -> Option<(u32, usize)> {
    let mut value = 0u32;
    for (i, &b) in data.iter().enumerate().take(8) {
        value |= u32::from(b & 0x7f) << (7 * i);
        if b & 0x80 == 0 {
            return Some((value, i + 1));
        }
    }
    None
}

fn write_leb128(out: &mut Vec<u8>, mut val: u32) {
    loop {
        let mut b = (val & 0x7f) as u8;
        val >>= 7;
        if val != 0 {
            b |= 0x80;
            out.push(b);
        } else {
            out.push(b);
            break;
        }
    }
}

fn obu_type(header: u8) -> u8 {
    (header >> 3) & 0x0f
}

fn obu_has_extension(header: u8) -> bool {
    header & 0x04 != 0
}

fn emit_obu(out: &mut Vec<u8>, raw: &[u8]) {
    if raw.is_empty() {
        return;
    }
    let header = raw[0];
    let ty = obu_type(header);
    if matches!(ty, 2 | 8 | 15) {
        // temporal delimiter / tile list / padding — not needed in IVF
        return;
    }
    let header_len = 1 + usize::from(obu_has_extension(header));
    if raw.len() < header_len {
        return;
    }
    let payload = &raw[header_len..];
    out.push(header | 0x02);
    if obu_has_extension(header) {
        out.push(raw[1]);
    }
    write_leb128(out, payload.len() as u32);
    out.extend_from_slice(payload);
}

/// Strip the AV1 RTP aggregation header and rebuild OBUs with size fields.
fn depacketize_av1(payload: &[u8], fragment: &mut Vec<u8>) -> Option<(Vec<u8>, bool)> {
    if payload.is_empty() {
        return None;
    }
    let agg = payload[0];
    let z = agg & 0x80 != 0;
    let y = agg & 0x40 != 0;
    let w = (agg >> 4) & 0x03;
    let n = agg & 0x08 != 0;
    if n {
        fragment.clear();
    }

    let mut rest = &payload[1..];
    let mut elements: Vec<&[u8]> = Vec::new();
    if w > 0 {
        for i in 1..=w {
            if i == w {
                elements.push(rest);
                break;
            }
            let (sz, nread) = read_leb128(rest)?;
            rest = rest.get(nread..)?;
            let sz = sz as usize;
            if rest.len() < sz {
                return None;
            }
            elements.push(&rest[..sz]);
            rest = &rest[sz..];
        }
    } else {
        while !rest.is_empty() {
            let (sz, nread) = read_leb128(rest)?;
            rest = rest.get(nread..)?;
            let sz = sz as usize;
            if rest.len() < sz {
                return None;
            }
            elements.push(&rest[..sz]);
            rest = &rest[sz..];
        }
    }

    let mut out = Vec::new();
    let last = elements.len().saturating_sub(1);
    for (i, elem) in elements.iter().enumerate() {
        let continuation = i == 0 && z;
        let continues = i == last && y;
        if continuation {
            fragment.extend_from_slice(elem);
            if !continues {
                emit_obu(&mut out, fragment);
                fragment.clear();
            }
        } else if continues {
            fragment.clear();
            fragment.extend_from_slice(elem);
        } else {
            emit_obu(&mut out, elem);
        }
    }
    Some((out, n))
}

fn av1_has_sequence_header(buf: &[u8]) -> bool {
    let mut rest = buf;
    while rest.len() >= 2 {
        let header = rest[0];
        let header_len = 1 + usize::from(obu_has_extension(header));
        if rest.len() < header_len {
            break;
        }
        let after_hdr = &rest[header_len..];
        let Some((sz, nread)) = read_leb128(after_hdr) else {
            break;
        };
        if obu_type(header) == 1 {
            return true;
        }
        let skip = header_len + nread + sz as usize;
        if skip > rest.len() {
            break;
        }
        rest = &rest[skip..];
    }
    false
}

fn strip_vp8(payload: &[u8]) -> Option<(&[u8], bool, bool)> {
    if payload.is_empty() {
        return None;
    }
    let b0 = payload[0];
    let x = b0 & 0x80 != 0;
    let s = b0 & 0x10 != 0;
    let mut i = 1usize;
    let mut i_bit = false;
    let mut l_bit = false;
    let mut t_bit = false;
    let mut k_bit = false;
    if x {
        let b1 = *payload.get(i)?;
        i += 1;
        i_bit = b1 & 0x80 != 0;
        l_bit = b1 & 0x40 != 0;
        t_bit = b1 & 0x20 != 0;
        k_bit = b1 & 0x10 != 0;
    }
    if i_bit {
        let b = *payload.get(i)?;
        i += 1;
        if b & 0x80 != 0 {
            i += 1;
        }
    }
    if l_bit {
        i += 1;
    }
    if t_bit || k_bit {
        i += 1;
    }
    let rest = payload.get(i..)?;
    if rest.is_empty() {
        return None;
    }
    let key = s && (rest[0] & 0x01) == 0;
    Some((rest, s, key))
}

fn strip_vp9(payload: &[u8]) -> Option<(&[u8], bool, bool)> {
    if payload.is_empty() {
        return None;
    }
    let b = payload[0];
    let i_bit = b & 0x80 != 0;
    let p = b & 0x40 != 0;
    let l = b & 0x20 != 0;
    let f = b & 0x10 != 0;
    let begin = b & 0x08 != 0;
    let v = b & 0x02 != 0;
    let mut off = 1usize;
    if i_bit {
        let pid = *payload.get(off)?;
        off += 1;
        if pid & 0x80 != 0 {
            off += 1;
        }
    }
    if l {
        off += 1;
        if !f {
            off += 1;
        }
    }
    if f && p {
        loop {
            let b = *payload.get(off)?;
            off += 1;
            if b & 0x01 == 0 {
                break;
            }
        }
    }
    if v {
        let b = *payload.get(off)?;
        off += 1;
        let ns = ((b >> 5) + 1) as usize;
        let y = b & 0x10 != 0;
        let g = (b >> 1) & 0x07 != 0;
        if y {
            off += 4 * ns;
        }
        if g {
            let ng = *payload.get(off)? as usize;
            off += 1;
            for _ in 0..ng {
                let gb = *payload.get(off)?;
                off += 1;
                let r = gb & 0x0f;
                off += r as usize;
            }
        }
    }
    let rest = payload.get(off..)?;
    Some((rest, begin, begin && !p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::meets::sfu::codec::VideoCodec;

    fn agg(z: bool, y: bool, w: u8, n: bool) -> u8 {
        (u8::from(z) << 7) | (u8::from(y) << 6) | ((w & 3) << 4) | (u8::from(n) << 3)
    }

    #[test]
    fn av1_strips_aggregation_header() {
        // Sequence header OBU type 1, no size bit: 0x08, plus 4 payload bytes.
        let packet = [agg(false, false, 1, true), 0x08, 0xaa, 0xbb, 0xcc, 0xdd];
        let mut frag = Vec::new();
        let (out, n) = depacketize_av1(&packet, &mut frag).unwrap();
        assert!(n);
        assert_eq!(obu_type(out[0]), 1);
        assert!(out[0] & 0x02 != 0, "size bit set");
        assert_ne!(obu_type(out[0]), 13);
        assert!(av1_has_sequence_header(&out));
    }

    #[test]
    fn av1_does_not_emit_type_13_from_aggregation() {
        // 0x68 is Z=0 Y=1 W=2 N=1 — libdav1d reported this AH byte as OBU type 13
        // when it was left in the IVF payload.
        let packet = [0x68, 2, 0x08, 0xaa, 0x08, 0xbb];
        let mut frag = Vec::new();
        let (out, _) = depacketize_av1(&packet, &mut frag).unwrap();
        assert!(!out.is_empty());
        assert_ne!(obu_type(out[0]), 13);
    }

    #[test]
    fn av1_reassembles_fragmented_obu() {
        let mut asm = AccessUnitAssembler::new(true, VideoCodec::Av1);
        let first = [agg(false, true, 1, true), 0x08, 0x11, 0x22];
        let second = [agg(true, false, 1, false), 0x33, 0x44];
        assert!(asm.push(1, false, &first).is_empty());
        let units = asm.push(1, true, &second);
        assert_eq!(units.len(), 1);
        assert!(units[0].is_keyframe);
        assert!(av1_has_sequence_header(&units[0].payload));
        assert_eq!(obu_type(units[0].payload[0]), 1);
    }

    #[test]
    fn vp8_strips_descriptor_and_detects_keyframe() {
        // X=0,S=1, rest is a keyframe (P bit 0).
        let packet = [0x10, 0x00, 0x01, 0x02];
        let (rest, start, key) = strip_vp8(&packet).unwrap();
        assert!(start && key);
        assert_eq!(rest, &[0x00, 0x01, 0x02]);
    }

    #[test]
    fn opus_emits_each_packet() {
        let mut asm = AccessUnitAssembler::new(false, VideoCodec::Vp8);
        let units = asm.push(48000, false, &[1, 2, 3]);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].payload, vec![1, 2, 3]);
    }
}
