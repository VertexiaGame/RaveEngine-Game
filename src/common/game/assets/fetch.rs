use std::time::Duration;

use bevy::prelude::*;

const ASSET_WORKERS: usize = 4;
const ASSET_QUEUE_CAP: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Image,
    Mesh,
    MeshTexture,
    Sound,
}

struct AssetWorkers {
    tx: crossbeam_channel::Sender<(u32, AssetKind, Option<String>)>,
    rx: std::sync::Mutex<crossbeam_channel::Receiver<(u32, AssetKind, Result<Vec<u8>, String>)>>,
}

impl AssetWorkers {
    fn start() -> Self {
        let (job_tx, job_rx) = crossbeam_channel::bounded::<(u32, AssetKind, Option<String>)>(ASSET_QUEUE_CAP);
        let (res_tx, res_rx) =
            crossbeam_channel::bounded::<(u32, AssetKind, Result<Vec<u8>, String>)>(
                ASSET_QUEUE_CAP,
            );
        for _ in 0..ASSET_WORKERS {
            let job_rx = job_rx.clone();
            let res_tx = res_tx.clone();
            std::thread::spawn(move || {
                while let Ok((asset_id, kind, user_ukey)) = job_rx.recv() {
                    let result = match kind {
                        AssetKind::MeshTexture => {
                            fetch_texture_bytes(asset_id, user_ukey.as_deref())
                        }
                        _ => fetch_asset_bytes(asset_id, user_ukey.as_deref()),
                    };
                    let _ = res_tx.send((asset_id, kind, result));
                }
            });
        }
        Self {
            tx: job_tx,
            rx: std::sync::Mutex::new(res_rx),
        }
    }
}

fn with_asset_auth(
    req: reqwest::blocking::RequestBuilder,
    user_ukey: Option<&str>,
) -> Result<reqwest::blocking::RequestBuilder, String> {
    if let Ok(api_key) = crate::common::net::api::gameserver_api_key() {
        if !api_key.is_empty() {
            return Ok(req.header("X-Gameserver-Key", api_key));
        }
    }
    match user_ukey.filter(|ukey| !ukey.is_empty()) {
        Some(ukey) => Ok(req.query(&[("ukey", ukey)])),
        None => Err("GAMESERVER_API_KEY environment variable is not configured and no user ukey is available".to_string()),
    }
}

fn fetch_asset_bytes(asset_id: u32, user_ukey: Option<&str>) -> Result<Vec<u8>, String> {
    let base = crate::common::net::api::api_base();
    let url = format!("{base}/api/v1/assets/{asset_id}/file");
    let client = crate::common::net::api::blocking_client(Duration::from_secs(10))?;
    let resp = with_asset_auth(client.get(&url), user_ukey)?
        .send()
        .map_err(|e| format!("asset request failed: {e}"))?;
    let status = resp.status();
    let bytes = resp
        .bytes()
        .map_err(|e| format!("failed to read asset response: {e}"))?;
    if !status.is_success() {
        return Err(format!("asset server returned error: {status}"));
    }
    Ok(bytes.to_vec())
}

fn fetch_texture_bytes(asset_id: u32, user_ukey: Option<&str>) -> Result<Vec<u8>, String> {
    let base = crate::common::net::api::api_base();
    let url = format!("{base}/api/v1/assets/{asset_id}/texture");
    let client = crate::common::net::api::blocking_client(Duration::from_secs(10))?;
    let resp = with_asset_auth(client.get(&url), user_ukey)?
        .send()
        .map_err(|e| format!("texture request failed: {e}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    let bytes = resp
        .bytes()
        .map_err(|e| format!("failed to read texture response: {e}"))?;
    if !status.is_success() {
        return Err(format!("texture server returned error: {status}"));
    }
    Ok(bytes.to_vec())
}

#[derive(Resource, Default)]
pub struct AssetFetchPool(Option<std::sync::Arc<AssetWorkers>>);

impl AssetFetchPool {
    pub fn ensure_started(&mut self) {
        if self.0.is_none() {
            self.0 = Some(std::sync::Arc::new(AssetWorkers::start()));
        }
    }

    pub fn submit(&self, asset_id: u32, user_ukey: Option<String>) -> bool {
        self.submit_kind(asset_id, AssetKind::Image, user_ukey)
    }

    pub fn submit_kind(
        &self,
        asset_id: u32,
        kind: AssetKind,
        user_ukey: Option<String>,
    ) -> bool {
        match &self.0 {
            Some(workers) => workers.tx.try_send((asset_id, kind, user_ukey)).is_ok(),
            None => false,
        }
    }

    pub fn drain_results(&self) -> Vec<(u32, AssetKind, Result<Vec<u8>, String>)> {
        match &self.0 {
            Some(workers) => {
                let rx = workers.rx.lock().unwrap();
                rx.try_iter().collect()
            }
            None => Vec::new(),
        }
    }
}
