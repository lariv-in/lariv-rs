//! Subscribe to hang broadcasts in a live room and stage frames for recording.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use moq_relay::Cluster;
use sea_orm::DatabaseConnection;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::plugins::filesystem::storage::DynFilestore;

use super::Recorder;
use super::codec::VideoCodec;

pub struct RoomRecorder {
    code: String,
    recorder: Arc<Recorder>,
    tasks: Mutex<HashMap<String, JoinHandle<()>>>,
    active: Mutex<HashSet<String>>,
}

impl RoomRecorder {
    pub fn start(code: &str, created_by_id: i64) -> anyhow::Result<Arc<Self>> {
        let recorder = Recorder::start(code, VideoCodec::Vp8, created_by_id)?;
        Ok(Arc::new(Self {
            code: code.to_string(),
            recorder: Arc::new(recorder),
            tasks: Mutex::new(HashMap::new()),
            active: Mutex::new(HashSet::new()),
        }))
    }

    async fn run_watch(
        self: Arc<Self>,
        cluster: Cluster,
        db: DatabaseConnection,
        store: Arc<DynFilestore>,
    ) {
        let prefix = format!("meets/{}/", self.code);
        let consumer = cluster.origin.consume();
        let mut announced = consumer.announced();
        while let Some(update) = announced.next().await {
            let path = update.path.as_str();
            if !path.starts_with(&prefix) {
                continue;
            }
            let relative = path
                .strip_prefix(&prefix)
                .unwrap_or(path)
                .trim_end_matches(".hang");
            let Some(parsed) = moq_room::parse(relative) else {
                continue;
            };
            let joined_user_id = parsed.identity.as_str().parse::<i64>().unwrap_or(0);
            if joined_user_id == 0 {
                continue;
            }
            let task_key = format!("{}:{}", joined_user_id, parsed.kind.as_str());

            if let Some(broadcast) = update.broadcast {
                let mut tasks = self.tasks.lock().await;
                if tasks.contains_key(&task_key) {
                    continue;
                }
                let recorder = Arc::clone(&self.recorder);
                let handle = tokio::spawn(async move {
                    if let Err(err) = record_broadcast(joined_user_id, broadcast, recorder).await {
                        tracing::warn!(
                            joined_user_id,
                            error = %err,
                            "room recorder track ended"
                        );
                    }
                });
                tasks.insert(task_key.clone(), handle);
                self.active.lock().await.insert(task_key);
            } else {
                self.stop_track(&task_key).await;
                if self.active.lock().await.is_empty() {
                    break;
                }
            }
        }

        self.finalize(db, store).await;
    }

    async fn stop_track(&self, task_key: &str) {
        let mut tasks = self.tasks.lock().await;
        if let Some(handle) = tasks.remove(task_key) {
            handle.abort();
        }
        self.active.lock().await.remove(task_key);
    }

    async fn finalize(self: &Arc<Self>, db: DatabaseConnection, store: Arc<DynFilestore>) {
        let mut tasks = self.tasks.lock().await;
        for handle in tasks.values() {
            handle.abort();
        }
        tasks.clear();
        if let Err(err) = self
            .recorder
            .finalize_vnode(&db, store.as_ref(), &self.code)
            .await
        {
            tracing::error!(room_code = %self.code, error = %err, "finalize meeting recording");
        }
    }
}

async fn record_broadcast(
    joined_user_id: i64,
    broadcast: hang::moq_net::broadcast::Consumer,
    recorder: Arc<Recorder>,
) -> anyhow::Result<()> {
    let catalog_track = broadcast
        .track(hang::Catalog::DEFAULT_NAME)?
        .subscribe(hang::Catalog::default_subscription())
        .await?;
    let mut catalog = moq_mux::catalog::hang::Consumer::<()>::new(catalog_track);
    let info = catalog
        .next()
        .await?
        .ok_or_else(|| anyhow::anyhow!("no hang catalog"))?;

    if let Some((name, _)) = info.video.renditions.iter().next() {
        let track = broadcast
            .track(name)?
            .subscribe(hang::moq_net::track::Subscription::default().with_priority(1))
            .await?;
        let mut ordered =
            moq_mux::container::Consumer::new(track, moq_mux::catalog::hang::Container::Legacy)
                .with_latency(Duration::from_millis(500));
        while let Some(frame) = ordered.read().await? {
            let ts_us = u64::try_from(frame.timestamp.as_micros()).unwrap_or(u64::MAX);
            recorder.push_frame(joined_user_id, true, ts_us, frame.keyframe, &frame.payload);
        }
    }

    if let Some((name, _)) = info.audio.renditions.iter().next() {
        let track = broadcast
            .track(name)?
            .subscribe(hang::moq_net::track::Subscription::default().with_priority(1))
            .await?;
        let mut ordered =
            moq_mux::container::Consumer::new(track, moq_mux::catalog::hang::Container::Legacy)
                .with_latency(Duration::from_millis(500));
        while let Some(frame) = ordered.read().await? {
            let ts_us = u64::try_from(frame.timestamp.as_micros()).unwrap_or(u64::MAX);
            recorder.push_frame(joined_user_id, false, ts_us, frame.keyframe, &frame.payload);
        }
    }

    Ok(())
}

pub async fn ensure_room_recorder(
    recorders: &Arc<Mutex<HashMap<String, Arc<RoomRecorder>>>>,
    cluster: Option<Cluster>,
    db: DatabaseConnection,
    store: Arc<DynFilestore>,
    room_code: &str,
    created_by_id: i64,
) {
    if cluster.is_none() {
        return;
    }
    let cluster = cluster.expect("cluster present");
    {
        let guard = recorders.lock().await;
        if guard.contains_key(room_code) {
            return;
        }
    }
    let recorder = match RoomRecorder::start(room_code, created_by_id) {
        Ok(rec) => rec,
        Err(err) => {
            tracing::error!(room_code, error = %err, "start room recorder");
            return;
        }
    };
    recorders
        .lock()
        .await
        .insert(room_code.to_string(), Arc::clone(&recorder));
    let task = Arc::clone(&recorder);
    tokio::spawn(async move {
        task.run_watch(cluster, db, store).await;
    });
}

pub async fn stop_room_recorder(
    recorders: &Arc<Mutex<HashMap<String, Arc<RoomRecorder>>>>,
    room_code: &str,
) {
    recorders.lock().await.remove(room_code);
}
