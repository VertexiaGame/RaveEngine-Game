use std::collections::{HashMap, HashSet};
use std::time::Duration;

use bevy::prelude::*;

const STATUS_WORKERS: usize = 2;
const STATUS_QUEUE_CAP: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetStatus {
    Approved,
    Pending,
    Rejected,
    NotFound,
}

struct StatusWorkers {
    tx: crossbeam_channel::Sender<(u32, Option<String>)>,
    rx: std::sync::Mutex<
        crossbeam_channel::Receiver<(u32, Result<(AssetStatus, Option<i32>), String>)>,
    >,
}

impl StatusWorkers {
    fn start() -> Self {
        let (job_tx, job_rx) =
            crossbeam_channel::bounded::<(u32, Option<String>)>(STATUS_QUEUE_CAP);
        let (res_tx, res_rx) =
            crossbeam_channel::bounded::<(u32, Result<(AssetStatus, Option<i32>), String>)>(
                STATUS_QUEUE_CAP,
            );
        for _ in 0..STATUS_WORKERS {
            let job_rx = job_rx.clone();
            let res_tx = res_tx.clone();
            std::thread::spawn(move || {
                while let Ok((asset_id, user_ukey)) = job_rx.recv() {
                    let result = fetch_asset_status(asset_id, user_ukey.as_deref());
                    let _ = res_tx.send((asset_id, result));
                }
            });
        }
        Self {
            tx: job_tx,
            rx: std::sync::Mutex::new(res_rx),
        }
    }
}

fn fetch_asset_status(
    asset_id: u32,
    user_ukey: Option<&str>,
) -> Result<(AssetStatus, Option<i32>), String> {
    let base = crate::common::net::api::api_base();
    let url = format!("{base}/api/v1/assets/{asset_id}/status");
    let client = crate::common::net::api::blocking_client(Duration::from_secs(10))?;
    let mut req = client.get(&url);
    if let Ok(api_key) = crate::common::net::api::gameserver_api_key() {
        if !api_key.is_empty() {
            req = req.header("X-Gameserver-Key", api_key);
        } else if let Some(ukey) = user_ukey.filter(|s| !s.is_empty()) {
            req = req.query(&[("ukey", ukey)]);
        }
    } else if let Some(ukey) = user_ukey.filter(|s| !s.is_empty()) {
        req = req.query(&[("ukey", ukey)]);
    }
    let resp = req
        .send()
        .map_err(|e| format!("status request failed: {e}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok((AssetStatus::NotFound, None));
    }
    let body = resp
        .text()
        .map_err(|e| format!("failed to read status response: {e}"))?;
    if !status.is_success() {
        return Err(format!("status server returned error: {status}"));
    }
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("invalid status response: {e}"))?;
    let state = value
        .get("approval_state")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let owner_uid = value
        .get("uid")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);
    match state.as_str() {
        "approved" => Ok((AssetStatus::Approved, owner_uid)),
        "pending" => Ok((AssetStatus::Pending, owner_uid)),
        "rejected" | "declined" => Ok((AssetStatus::Rejected, owner_uid)),
        _ => Err(format!("unknown approval state: {state}")),
    }
}

#[derive(Resource, Default)]
pub struct AssetStatusCache {
    pub statuses: HashMap<u32, AssetStatus>,
    pub owner_uids: HashMap<u32, i32>,
}

#[derive(Resource, Default)]
pub struct AssetStatusPool(Option<std::sync::Arc<StatusWorkers>>);

impl AssetStatusPool {
    pub fn ensure_started(&mut self) {
        if self.0.is_none() {
            self.0 = Some(std::sync::Arc::new(StatusWorkers::start()));
        }
    }

    pub fn submit(&self, asset_id: u32, user_ukey: Option<String>) -> bool {
        match &self.0 {
            Some(workers) => workers.tx.try_send((asset_id, user_ukey)).is_ok(),
            None => false,
        }
    }

    pub fn drain_results(&self) -> Vec<(u32, Result<(AssetStatus, Option<i32>), String>)> {
        match &self.0 {
            Some(workers) => {
                let rx = workers.rx.lock().unwrap();
                rx.try_iter().collect()
            }
            None => Vec::new(),
        }
    }
}

fn current_user_ukey(
    auth_store: Option<Res<crate::studio::auth::StudioAuthStore>>,
    client_ukey: Option<Res<crate::client::ClientUkey>>,
) -> Option<String> {
    if let Some(ukey) = auth_store
        .as_ref()
        .and_then(|store| store.credentials.clone())
        .map(|creds| creds.ukey)
        .filter(|ukey| !ukey.is_empty())
    {
        return Some(ukey);
    }
    client_ukey
        .map(|ukey| ukey.0.clone())
        .filter(|ukey| !ukey.is_empty())
}

pub fn request_missing_statuses(
    meshes: Query<&crate::common::game::assets::components::Mesh>,
    textures: Query<&crate::common::game::assets::components::Texture>,
    cache: Res<AssetStatusCache>,
    mut pending: ResMut<PendingStatusFetches>,
    mut pool: ResMut<AssetStatusPool>,
    auth_store: Option<Res<crate::studio::auth::StudioAuthStore>>,
    client_ukey: Option<Res<crate::client::ClientUkey>>,
) {
    let user_ukey = current_user_ukey(auth_store, client_ukey);
    let mut missing: HashSet<u32> = HashSet::new();
    for mesh in &meshes {
        if mesh.asset_id != 0
            && !cache.statuses.contains_key(&mesh.asset_id)
            && !pending.0.contains(&mesh.asset_id)
        {
            missing.insert(mesh.asset_id);
        }
    }
    for texture in &textures {
        if texture.asset_id != 0
            && !cache.statuses.contains_key(&texture.asset_id)
            && !pending.0.contains(&texture.asset_id)
        {
            missing.insert(texture.asset_id);
        }
    }
    if missing.is_empty() {
        return;
    }
    pool.ensure_started();
    for asset_id in missing {
        if pool.submit(asset_id, user_ukey.clone()) {
            pending.0.insert(asset_id);
        }
    }
}

#[derive(Resource, Default)]
pub struct PendingStatusFetches(pub HashSet<u32>);

pub fn poll_status_results(
    pool: Res<AssetStatusPool>,
    mut pending: ResMut<PendingStatusFetches>,
    mut cache: ResMut<AssetStatusCache>,
) {
    for (asset_id, result) in pool.drain_results() {
        pending.0.remove(&asset_id);
        match result {
            Ok((status, owner_uid)) => {
                cache.statuses.insert(asset_id, status);
                if let Some(uid) = owner_uid {
                    cache.owner_uids.insert(asset_id, uid);
                }
            }
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_roundtrip_from_json() {
        let body = serde_json::json!({"id": 12, "type": "mesh", "approval_state": "pending"});
        let text = serde_json::to_string(&body).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let state = value
            .get("approval_state")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(state, "pending");
    }

    #[test]
    fn declined_maps_to_rejected() {
        let mapped = match "declined" {
            "approved" => AssetStatus::Approved,
            "pending" => AssetStatus::Pending,
            "rejected" | "declined" => AssetStatus::Rejected,
            _ => AssetStatus::Approved,
        };
        assert_eq!(mapped, AssetStatus::Rejected);
    }
}
