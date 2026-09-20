use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use sea_orm::DatabaseConnection;
use tokio::sync::{Mutex, mpsc};
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{
    MIME_TYPE_AV1, MIME_TYPE_H264, MIME_TYPE_OPUS, MIME_TYPE_VP8, MIME_TYPE_VP9, MediaEngine,
};
use webrtc::api::setting_engine::SettingEngine;
use webrtc::ice::udp_network::{EphemeralUDP, UDPNetwork};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::RTCRtpTransceiverInit;
use webrtc::rtp_transceiver::rtp_codec::{
    RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType,
};
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::rtp_transceiver::rtp_transceiver_direction::RTCRtpTransceiverDirection;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};
use webrtc::track::track_remote::TrackRemote;

use crate::plugins::filesystem::storage::DynFilestore;
use crate::plugins::meets::config::MeetsConfig;
use crate::plugins::meets::recording::Recorder;
use crate::plugins::meets::sfu::codec::{VideoCodec, negotiate, parse_codec_list};
use crate::plugins::meets::signaling::ServerMsg;

const MIME_TYPE_H265: &str = "video/H265";

struct Client {
    pc: Arc<RTCPeerConnection>,
    outbound: mpsc::UnboundedSender<ServerMsg>,
    codecs: HashSet<VideoCodec>,
}

struct RoomInner {
    clients: HashMap<i64, Arc<Client>>,
    video_codec: Option<VideoCodec>,
    recorder: Option<Arc<Recorder>>,
}

pub struct SfuRoom {
    code: String,
    config: MeetsConfig,
    db: DatabaseConnection,
    store: Arc<DynFilestore>,
    created_by_id: AtomicI64,
    inner: Mutex<RoomInner>,
}

impl SfuRoom {
    pub fn new(
        code: String,
        config: MeetsConfig,
        db: DatabaseConnection,
        store: Arc<DynFilestore>,
    ) -> Self {
        Self {
            code,
            config,
            db,
            store,
            created_by_id: AtomicI64::new(0),
            inner: Mutex::new(RoomInner {
                clients: HashMap::new(),
                video_codec: None,
                recorder: None,
            }),
        }
    }

    pub fn set_created_by(&self, id: i64) {
        self.created_by_id.store(id, Ordering::Relaxed);
    }

    pub async fn add_peer(
        self: &Arc<Self>,
        joined_user_id: i64,
        video_codecs: Vec<String>,
        outbound: mpsc::UnboundedSender<ServerMsg>,
    ) -> Result<(), String> {
        let caps = parse_codec_list(&video_codecs);
        if caps.is_empty() {
            return Err("no supported video codecs".into());
        }

        let (winner, codec_changed) = {
            let mut inner = self.inner.lock().await;
            let mut sets: Vec<HashSet<VideoCodec>> =
                inner.clients.values().map(|c| c.codecs.clone()).collect();
            sets.push(caps.clone());
            let Some(winner) = negotiate(&sets) else {
                return Err("no common video codec with current participants".into());
            };
            let codec_changed = inner.video_codec.map(|c| c != winner).unwrap_or(false);
            inner.video_codec = Some(winner);
            (winner, codec_changed)
        };

        let pc = new_peer_connection(&self.config)
            .await
            .map_err(|e| e.to_string())?;
        let pc = Arc::new(pc);

        let ice_tx = outbound.clone();
        pc.on_ice_candidate(Box::new(move |c| {
            let ice_tx = ice_tx.clone();
            Box::pin(async move {
                if let Some(c) = c
                    && let Ok(init) = c.to_json()
                {
                    let _ = ice_tx.send(ServerMsg::Ice {
                        candidate: init.candidate,
                        sdp_mid: init.sdp_mid,
                        sdp_mline_index: init.sdp_mline_index,
                    });
                }
            })
        }));

        let room = Arc::clone(self);
        pc.on_track(Box::new(move |track, _, _| {
            let room = Arc::clone(&room);
            Box::pin(async move {
                room.forward_track(joined_user_id, track).await;
            })
        }));

        let _ = pc
            .add_transceiver_from_kind(
                RTPCodecType::Audio,
                Some(RTCRtpTransceiverInit {
                    direction: RTCRtpTransceiverDirection::Recvonly,
                    send_encodings: vec![],
                }),
            )
            .await;
        let _ = pc
            .add_transceiver_from_kind(
                RTPCodecType::Video,
                Some(RTCRtpTransceiverInit {
                    direction: RTCRtpTransceiverDirection::Recvonly,
                    send_encodings: vec![],
                }),
            )
            .await;

        let offer = pc.create_offer(None).await.map_err(|e| e.to_string())?;
        pc.set_local_description(offer)
            .await
            .map_err(|e| e.to_string())?;
        wait_ice(&pc).await;
        let sdp = pc
            .local_description()
            .await
            .map(|d| d.sdp)
            .unwrap_or_default();

        let _ = outbound.send(ServerMsg::Codec {
            video: winner.as_sdp().to_string(),
            audio: "opus".into(),
        });
        let _ = outbound.send(ServerMsg::Offer { sdp });

        {
            let mut inner = self.inner.lock().await;
            for existing_id in inner.clients.keys().copied() {
                let _ = outbound.send(ServerMsg::ParticipantJoined {
                    joined_user_id: existing_id,
                });
            }
            for client in inner.clients.values() {
                let _ = client
                    .outbound
                    .send(ServerMsg::ParticipantJoined { joined_user_id });
            }
            inner.clients.insert(
                joined_user_id,
                Arc::new(Client {
                    pc,
                    outbound,
                    codecs: caps,
                }),
            );
        }

        if codec_changed {
            self.renegotiate_all().await;
        }
        Ok(())
    }

    pub async fn handle_answer(&self, joined_user_id: i64, sdp: String) -> Result<(), String> {
        let inner = self.inner.lock().await;
        let Some(client) = inner.clients.get(&joined_user_id) else {
            return Err("not in room".into());
        };
        let answer = RTCSessionDescription::answer(sdp).map_err(|e| e.to_string())?;
        client
            .pc
            .set_remote_description(answer)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn handle_ice(
        &self,
        joined_user_id: i64,
        msg: RTCIceCandidateInit,
    ) -> Result<(), String> {
        let inner = self.inner.lock().await;
        let Some(client) = inner.clients.get(&joined_user_id) else {
            return Err("not in room".into());
        };
        client
            .pc
            .add_ice_candidate(msg)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn remove_peer(&self, joined_user_id: i64) {
        let mut inner = self.inner.lock().await;
        if let Some(client) = inner.clients.remove(&joined_user_id) {
            let _ = client.pc.close().await;
            for other in inner.clients.values() {
                let _ = other
                    .outbound
                    .send(ServerMsg::ParticipantLeft { joined_user_id });
            }
        }
        if inner.clients.is_empty()
            && let Some(rec) = inner.recorder.take()
        {
            let db = self.db.clone();
            let store = Arc::clone(&self.store);
            let code = self.code.clone();
            tokio::spawn(async move {
                if let Err(e) = rec.finalize_vnode(&db, store.as_ref(), &code).await {
                    tracing::error!(error = %e, "finalize meeting recording");
                }
            });
        }
    }

    async fn renegotiate_all(&self) {
        let inner = self.inner.lock().await;
        let codec = inner.video_codec;
        let clients: Vec<_> = inner.clients.values().cloned().collect();
        drop(inner);
        if let Some(codec) = codec {
            for client in &clients {
                let _ = client.outbound.send(ServerMsg::Codec {
                    video: codec.as_sdp().to_string(),
                    audio: "opus".into(),
                });
                if let Ok(offer) = client.pc.create_offer(None).await
                    && client.pc.set_local_description(offer).await.is_ok()
                {
                    wait_ice(&client.pc).await;
                    if let Some(local) = client.pc.local_description().await {
                        let _ = client.outbound.send(ServerMsg::Offer { sdp: local.sdp });
                    }
                }
            }
        }
    }

    async fn forward_track(self: Arc<Self>, publisher_id: i64, track: Arc<TrackRemote>) {
        let is_video = track.kind() == RTPCodecType::Video;
        let codec_cap = track.codec().capability;
        let stream_id = track.stream_id();
        let track_id = track.id();
        let kind = if is_video { "video" } else { "audio" };

        let (codec, recorder, publisher_pc) = {
            let mut inner = self.inner.lock().await;
            let codec = inner.video_codec.unwrap_or(VideoCodec::Vp8);
            if inner.recorder.is_none() {
                let created_by = self.created_by_id.load(Ordering::Relaxed);
                match Recorder::start(&self.code, codec, created_by) {
                    Ok(rec) => inner.recorder = Some(Arc::new(rec)),
                    Err(e) => tracing::error!(error = %e, "start recorder"),
                }
            }
            for client in inner.clients.values() {
                let _ = client.outbound.send(ServerMsg::TracksChanged {
                    joined_user_id: publisher_id,
                    kind: kind.into(),
                });
            }
            let publisher_pc = inner.clients.get(&publisher_id).map(|c| Arc::clone(&c.pc));
            (codec, inner.recorder.clone(), publisher_pc)
        };

        let mut locals: Vec<Arc<TrackLocalStaticRTP>> = Vec::new();
        {
            let inner = self.inner.lock().await;
            for (id, client) in &inner.clients {
                if *id == publisher_id {
                    continue;
                }
                let local = Arc::new(TrackLocalStaticRTP::new(
                    codec_cap.clone(),
                    format!("{track_id}-{id}"),
                    stream_id.clone(),
                ));
                match client
                    .pc
                    .add_track(Arc::clone(&local) as Arc<dyn TrackLocal + Send + Sync>)
                    .await
                {
                    Ok(sender) => {
                        spawn_rtcp_reader(sender, publisher_pc.clone());
                        locals.push(local);
                        let outbound = client.outbound.clone();
                        let pc = Arc::clone(&client.pc);
                        tokio::spawn(async move {
                            if let Ok(offer) = pc.create_offer(None).await
                                && pc.set_local_description(offer).await.is_ok()
                            {
                                wait_ice(&pc).await;
                                if let Some(local) = pc.local_description().await {
                                    let _ = outbound.send(ServerMsg::Offer { sdp: local.sdp });
                                }
                            }
                        });
                    }
                    Err(e) => tracing::warn!(error = %e, "add_track"),
                }
            }
        }

        loop {
            match track.read_rtp().await {
                Ok((rtp, _)) => {
                    if let Some(rec) = &recorder {
                        let created = rec.push_rtp(
                            publisher_id,
                            is_video,
                            rtp.header.timestamp,
                            rtp.header.marker,
                            rtp.payload.as_ref(),
                        );
                        if created {
                            let rec = Arc::clone(rec);
                            tokio::spawn(async move {
                                if let Err(e) = rec.remux_now().await {
                                    tracing::warn!(error = %e, "remux after keyframe");
                                }
                            });
                        }
                    }
                    for local in &locals {
                        let _ = local.write_rtp(&rtp).await;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = codec;
    }
}

fn spawn_rtcp_reader(sender: Arc<RTCRtpSender>, publisher_pc: Option<Arc<RTCPeerConnection>>) {
    tokio::spawn(async move {
        loop {
            match sender.read_rtcp().await {
                Ok((packets, _)) => {
                    let mut forward = Vec::new();
                    for p in packets {
                        let any = p.as_any();
                        if any
                            .downcast_ref::<webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication>()
                            .is_some()
                            || any
                                .downcast_ref::<webrtc::rtcp::payload_feedbacks::full_intra_request::FullIntraRequest>()
                                .is_some()
                        {
                            forward.push(p);
                        }
                    }
                    if !forward.is_empty()
                        && let Some(pc) = &publisher_pc
                    {
                        let _ = pc.write_rtcp(&forward).await;
                    }
                }
                Err(_) => break,
            }
        }
    });
}

async fn wait_ice(pc: &RTCPeerConnection) {
    let mut complete = pc.gathering_complete_promise().await;
    let _ = complete.recv().await;
}

async fn new_peer_connection(config: &MeetsConfig) -> Result<RTCPeerConnection, webrtc::Error> {
    let mut m = MediaEngine::default();
    register_preferred_codecs(&mut m)?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m)?;

    let mut setting = SettingEngine::default();
    let bind_host = config.bind_host.clone();
    setting.set_ip_filter(Box::new(move |ip| allow_ice_ip(ip, &bind_host)));
    if let Some(ip) = config
        .advertised_ip
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        setting.set_nat_1to1_ips(vec![ip.to_string()], RTCIceCandidateType::Host);
        setting.set_lite(true);
    }
    let port = config.ice_udp_port.max(1);
    let max = port.saturating_add(64).max(port);
    if let Ok(ephemeral) = EphemeralUDP::new(port, max) {
        setting.set_udp_network(UDPNetwork::Ephemeral(ephemeral));
    }

    let api = APIBuilder::new()
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .with_setting_engine(setting)
        .build();

    let mut ice_servers = Vec::new();
    for url in &config.stun_servers {
        ice_servers.push(RTCIceServer {
            urls: vec![url.clone()],
            ..Default::default()
        });
    }
    for turn in &config.turn_servers {
        if turn.urls.is_empty() {
            continue;
        }
        ice_servers.push(RTCIceServer {
            urls: turn.urls.clone(),
            username: turn.username.clone(),
            credential: turn.credential.clone(),
            ..Default::default()
        });
    }

    api.new_peer_connection(RTCConfiguration {
        ice_servers,
        ..Default::default()
    })
    .await
}

fn register_preferred_codecs(m: &mut MediaEngine) -> Result<(), webrtc::Error> {
    let video = [
        (MIME_TYPE_AV1, 96u8, ""),
        (MIME_TYPE_H265, 97, ""),
        (MIME_TYPE_VP9, 98, ""),
        (MIME_TYPE_VP8, 99, ""),
        (
            MIME_TYPE_H264,
            102,
            "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f",
        ),
    ];
    for (mime, pt, fmtp) in video {
        m.register_codec(
            RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: mime.to_owned(),
                    clock_rate: 90000,
                    channels: 0,
                    sdp_fmtp_line: fmtp.to_owned(),
                    rtcp_feedback: vec![],
                },
                payload_type: pt,
                ..Default::default()
            },
            RTPCodecType::Video,
        )?;
    }
    m.register_codec(
        RTCRtpCodecParameters {
            capability: RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 111,
            ..Default::default()
        },
        RTPCodecType::Audio,
    )?;
    Ok(())
}

fn allow_ice_ip(ip: IpAddr, bind_host: &str) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return false;
    }
    match ip {
        IpAddr::V4(v4) if v4.is_link_local() => return false,
        IpAddr::V6(v6) if v6.is_unicast_link_local() => return false,
        _ => {}
    }
    if let Ok(bind) = bind_host.parse::<IpAddr>()
        && !bind.is_unspecified()
    {
        return ip == bind;
    }
    true
}

#[cfg(test)]
mod ice_ip_tests {
    use super::allow_ice_ip;
    use std::net::IpAddr;

    #[test]
    fn skips_link_local_ipv6() {
        let ip: IpAddr = "fe80::42ec:9a8c:8698:47e0".parse().unwrap();
        assert!(!allow_ice_ip(ip, "0.0.0.0"));
    }

    #[test]
    fn allows_global_ipv4() {
        let ip: IpAddr = "203.0.113.10".parse().unwrap();
        assert!(allow_ice_ip(ip, "0.0.0.0"));
    }

    #[test]
    fn restricts_to_concrete_bind_host() {
        let ip: IpAddr = "203.0.113.10".parse().unwrap();
        assert!(allow_ice_ip(ip, "203.0.113.10"));
        assert!(!allow_ice_ip(ip, "198.51.100.8"));
    }
}
