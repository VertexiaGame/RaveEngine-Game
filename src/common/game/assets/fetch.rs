use std::time::Duration;

use bevy::prelude::*;

const ASSET_WORKERS: usize = 4;
const ASSET_QUEUE_CAP: usize = 64;

struct AssetWorkers {
    tx: crossbeam_channel::Sender<u32>,
    rx: std::sync::Mutex<crossbeam_channel::Receiver<(u32, Result<Vec<u8>, String>)>>,
}

impl AssetWorkers {
    fn start() -> Self {
        let (job_tx, job_rx) = crossbeam_channel::bounded::<u32>(ASSET_QUEUE_CAP);
        let (res_tx, res_rx) =
            crossbeam_channel::bounded::<(u32, Result<Vec<u8>, String>)>(ASSET_QUEUE_CAP);
        for _ in 0..ASSET_WORKERS {
            let job_rx = job_rx.clone();
            let res_tx = res_tx.clone();
            std::thread::spawn(move || {
                while let Ok(asset_id) = job_rx.recv() {
                    let result = fetch_asset_bytes(asset_id);
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

fn fetch_asset_bytes(asset_id: u32) -> Result<Vec<u8>, String> {
    let base = crate::common::net::api::api_base();
    let api_key = crate::common::net::api::gameserver_api_key()?;
    let url = format!("{base}/api/v1/assets/{asset_id}/file");
    let client = crate::common::net::api::blocking_client(Duration::from_secs(10))?;
    let resp = client
        .get(&url)
        .header("X-Gameserver-Key", api_key)
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

#[derive(Resource, Default)]
pub struct AssetFetchPool(Option<std::sync::Arc<AssetWorkers>>);

impl AssetFetchPool {
    pub fn ensure_started(&mut self) {
        if self.0.is_none() {
            self.0 = Some(std::sync::Arc::new(AssetWorkers::start()));
        }
    }

    pub fn submit(&self, asset_id: u32) -> bool {
        match &self.0 {
            Some(workers) => workers.tx.try_send(asset_id).is_ok(),
            None => false,
        }
    }

    pub fn drain_results(&self) -> Vec<(u32, Result<Vec<u8>, String>)> {
        match &self.0 {
            Some(workers) => {
                let rx = workers.rx.lock().unwrap();
                rx.try_iter().collect()
            }
            None => Vec::new(),
        }
    }
}