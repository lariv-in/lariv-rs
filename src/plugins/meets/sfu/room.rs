use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use rtc::interceptor::Registry;
use rtc::media_stream::MediaStreamTrack;
use rtc::peer_connection::configuration::RTCConfigurationBuilder;
use rtc::peer_connection::configuration::interceptor_registry::register_default_interceptors;
use rtc::peer_connection::configuration::media_engine::{
    MIME_TYPE_AV1, MIME_TYPE_H264, MIME_TYPE_OPUS, MIME_TYPE_VP8, MIME_TYPE_VP9, MediaEngine,
};
use rtc::peer_connection::configuration::setting_engine::SettingEngineBuilder;
use rtc::peer_connection::sdp::RTCSessionDescription;
use rtc::peer_connection::transport::{RTCIceCandidateType, RTCIceCandidateInit, RTCIceServer};
use rtc::rtcp::payload_feedbacks::full_intra_request::FullIntraRequest;
use rtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use rtc::rtp_transceiver::rtp_sender::{
    RTCRtpCodec, RTCRtpCodecParameters, RTCRtpCodingParameters, RTCRtpEncodingParameters,
    RtpCodecKind,
};
use rtc::rtp_transceiver::{RTCRtpTransceiverDirection, RTCRtpTransceiverInit};
use rand::RngExt;
use sea_orm::DatabaseConnection;
use tokio::sync::{Mutex, mpsc};
use webrtc::error::{Error, Result as WebRtcResult};
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
use webrtc::media_stream::track_local::TrackLocalEvent;
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};
use webrtc::peer_connection::{
    PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler, RTCIceGatheringState,
    RTCPeerConnectionIceEvent,
};
use webrtc::runtime::{Sender, channel};

use crate::plugins::filesystem::storage::DynFilestore;
use crate::plugins::meets::config::MeetsConfig;
use crate::plugins::meets::recording::Recorder;
use crate::plugins::meets::sfu::codec::{VideoCodec, negotiate, parse_codec_list};
use crate::plugins::meets::signaling::ServerMsg;

const MIME_TYPE_H265: &str = "video/H265";

struct Client {
    pc: Arc<dyn PeerConnection>,
    outbound: mpsc::UnboundedSender<ServerMsg>,
    codecs: HashSet<VideoCodec>,
}

struct RoomInner {
    clients: HashMap<i64, Arc<Client>>,
    video_codec: Option<VideoCodec>,
    recorder: Option<Arc<Recorder>>,
}

#[derive(Clone)]
struct PeerHandler {
    outbound: mpsc::UnboundedSender<ServerMsg>,
    gather_complete_tx: Sender<()>,
    joined_user_id: i64,
    room: Arc<SfuRoom>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for PeerHandler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        if let Ok(init) = event.candidate.to_json() {
            if !init.candidate.is_empty()
                && !allow_ice_candidate(&init.candidate, &self.room.config.bind_host)
            {
                return;
            }
            if let Err(err) = self.outbound.send(ServerMsg::Ice {
                candidate: init.candidate,
                sdp_mid: init.sdp_mid,
                sdp_mline_index: init.sdp_mline_index,
            }) {
                tracing::debug!(error = %err, "send ice candidate");
            }
        }
    }

    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        if state == RTCIceGatheringState::Complete {
            let _ = self.gather_complete_tx.try_send(());
        }
    }

    async fn on_track(&self, track: Arc<dyn TrackRemote>) {
        let room = Arc::clone(&self.room);
        let joined_user_id = self.joined_user_id;
        tokio::spawn(async move {
            room.forward_track(joined_user_id, track).await;
        });
    }
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

        let (gather_complete_tx, mut gather_complete_rx) = channel::<()>(1);
        let handler = Arc::new(PeerHandler {
            outbound: outbound.clone(),
            gather_complete_tx,
            joined_user_id,
            room: Arc::clone(self),
        });

        let pc = new_peer_connection(&self.config, handler)
            .await
            .map_err(|e| e.to_string())?;

        let _ = pc
            .add_transceiver_from_kind(
                RtpCodecKind::Audio,
                Some(RTCRtpTransceiverInit {
                    direction: RTCRtpTransceiverDirection::Recvonly,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| e.to_string())?;
        let _ = pc
            .add_transceiver_from_kind(
                RtpCodecKind::Video,
                Some(RTCRtpTransceiverInit {
                    direction: RTCRtpTransceiverDirection::Recvonly,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| e.to_string())?;

        let offer = pc.create_offer(None).await.map_err(|e| e.to_string())?;
        pc.set_local_description(offer)
            .await
            .map_err(|e| e.to_string())?;
        let _ = gather_complete_rx.recv().await;
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
                    if let Some(local) = client.pc.local_description().await {
                        let _ = client.outbound.send(ServerMsg::Offer { sdp: local.sdp });
                    }
                }
            }
        }
    }

    async fn forward_track(self: Arc<Self>, publisher_id: i64, track: Arc<dyn TrackRemote>) {
        let kind = track.kind().await;
        let is_video = kind == RtpCodecKind::Video;
        let media_ssrc = track.ssrcs().await.first().copied().unwrap_or(0);
        let codec = track.codec(media_ssrc).await.unwrap_or_default();
        let stream_id = track.stream_id().await;
        let track_id = track.track_id().await;
        let kind_label = if is_video { "video" } else { "audio" };

        let (video_codec, recorder) = {
            let mut inner = self.inner.lock().await;
            let video_codec = inner.video_codec.unwrap_or(VideoCodec::Vp8);
            if inner.recorder.is_none() {
                let created_by = self.created_by_id.load(Ordering::Relaxed);
                match Recorder::start(&self.code, video_codec, created_by) {
                    Ok(rec) => inner.recorder = Some(Arc::new(rec)),
                    Err(e) => tracing::error!(error = %e, "start recorder"),
                }
            }
            for client in inner.clients.values() {
                let _ = client.outbound.send(ServerMsg::TracksChanged {
                    joined_user_id: publisher_id,
                    kind: kind_label.into(),
                });
            }
            (video_codec, inner.recorder.clone())
        };

        let mut locals: Vec<Arc<TrackLocalStaticRTP>> = Vec::new();
        {
            let inner = self.inner.lock().await;
            for (id, client) in &inner.clients {
                if *id == publisher_id {
                    continue;
                }
                let local_ssrc = rand::rng().random::<u32>();
                let local = Arc::new(TrackLocalStaticRTP::new(MediaStreamTrack::new(
                    stream_id.clone(),
                    format!("{track_id}-{id}"),
                    format!("{track_id}-{id}"),
                    kind,
                    vec![RTCRtpEncodingParameters {
                        rtp_coding_parameters: RTCRtpCodingParameters {
                            ssrc: Some(local_ssrc),
                            ..Default::default()
                        },
                        codec: codec.clone(),
                        ..Default::default()
                    }],
                )));
                match client
                    .pc
                    .add_track(Arc::clone(&local) as Arc<dyn TrackLocal>)
                    .await
                {
                    Ok(_sender) => {
                        spawn_rtcp_reader(Arc::clone(&local), Some(Arc::clone(&track)));
                        locals.push(local);
                        let outbound = client.outbound.clone();
                        let pc = Arc::clone(&client.pc);
                        tokio::spawn(async move {
                            if let Ok(offer) = pc.create_offer(None).await
                                && pc.set_local_description(offer).await.is_ok()
                                && let Some(local) = pc.local_description().await
                            {
                                let _ = outbound.send(ServerMsg::Offer { sdp: local.sdp });
                            }
                        });
                    }
                    Err(e) => tracing::warn!(error = %e, "add_track"),
                }
            }
        }

        while let Some(evt) = track.poll().await {
            match evt {
                TrackRemoteEvent::OnRtpPacket(rtp) => {
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
                        let _ = local.write_rtp(rtp.clone()).await;
                    }
                }
                TrackRemoteEvent::OnEnded => break,
                _ => {}
            }
        }
        let _ = video_codec;
    }
}

fn spawn_rtcp_reader(
    local: Arc<TrackLocalStaticRTP>,
    publisher_track: Option<Arc<dyn TrackRemote>>,
) {
    tokio::spawn(async move {
        while let Some(evt) = local.poll().await {
            if let TrackLocalEvent::OnRtcpPacket(packets) = evt {
                let forward = packets
                    .iter()
                    .filter(|p| {
                        let any = p.as_any();
                        any.is::<PictureLossIndication>() || any.is::<FullIntraRequest>()
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if !forward.is_empty()
                    && let Some(track) = &publisher_track
                {
                    let _ = track.write_rtcp(forward).await;
                }
            }
        }
    });
}

async fn new_peer_connection(
    config: &MeetsConfig,
    handler: Arc<dyn PeerConnectionEventHandler>,
) -> Result<Arc<dyn PeerConnection>, Error> {
    let mut media_engine = MediaEngine::default();
    register_preferred_codecs(&mut media_engine)?;

    let registry = register_default_interceptors(Registry::new(), &mut media_engine)?;

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

    let mut setting_builder = SettingEngineBuilder::new();
    if let Some(ip) = config
        .advertised_ip
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        setting_builder = setting_builder
            .with_nat_1to1_ips(vec![ip.to_string()], RTCIceCandidateType::Host)
            .with_lite(true);
    }
    let setting_engine = setting_builder.build();

    let rtc_config = RTCConfigurationBuilder::new()
        .with_ice_servers(ice_servers)
        .build();

    let bind_addr = if config.bind_host.trim().is_empty() || config.bind_host == "0.0.0.0" {
        "0.0.0.0:0".to_string()
    } else {
        format!("{}:0", config.bind_host)
    };

    let pc = PeerConnectionBuilder::new()
        .with_configuration(rtc_config)
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .with_setting_engine(setting_engine)
        .with_handler(handler)
        .with_udp_addrs(vec![bind_addr])
        .build()
        .await?;

    Ok(Arc::new(pc))
}

fn register_preferred_codecs(m: &mut MediaEngine) -> WebRtcResult<()> {
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
                rtp_codec: RTCRtpCodec {
                    mime_type: mime.to_owned(),
                    clock_rate: 90000,
                    channels: 0,
                    sdp_fmtp_line: fmtp.to_owned(),
                    rtcp_feedback: vec![],
                },
                payload_type: pt,
                ..Default::default()
            },
            RtpCodecKind::Video,
        )?;
    }
    m.register_codec(
        RTCRtpCodecParameters {
            rtp_codec: RTCRtpCodec {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
                rtcp_feedback: vec![],
            },
            payload_type: 111,
            ..Default::default()
        },
        RtpCodecKind::Audio,
    )?;
    Ok(())
}

fn ice_candidate_address(candidate: &str) -> Option<IpAddr> {
    let rest = candidate.strip_prefix("candidate:")?;
    rest.split_whitespace().nth(4)?.parse().ok()
}

fn allow_ice_candidate(candidate: &str, bind_host: &str) -> bool {
    let Some(ip) = ice_candidate_address(candidate) else {
        return true;
    };
    allow_ice_ip(ip, bind_host)
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
