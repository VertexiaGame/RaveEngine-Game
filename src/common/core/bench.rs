use bevy::prelude::*;
use std::time::Instant;

#[derive(Resource, Default)]
pub struct BenchStats {
    scenario: String,
    warmup_frames: u64,
    target_frames: u64,
    seen_frames: u64,
    frame_start: Option<Instant>,
    frame_ns: Vec<u128>,
    finished: bool,
    mesh_assets: usize,
    material_assets: usize,
}

impl BenchStats {
    pub fn configure(&mut self, scenario: impl Into<String>, warmup_frames: u64, target_frames: u64) {
        assert!(target_frames > 0, "benchmark frames must be greater than zero");
        self.scenario = scenario.into();
        self.warmup_frames = warmup_frames;
        self.target_frames = target_frames;
        self.seen_frames = 0;
        self.frame_start = None;
        self.frame_ns.clear();
        self.finished = false;
        self.mesh_assets = 0;
        self.material_assets = 0;
    }

    pub fn set_asset_counts(&mut self, mesh_assets: usize, material_assets: usize) {
        self.mesh_assets = mesh_assets;
        self.material_assets = material_assets;
    }

    fn begin_frame(&mut self, now: Instant) {
        if self.target_frames > 0 && self.seen_frames >= self.warmup_frames && !self.finished {
            self.frame_start = Some(now);
        }
    }

    fn finish_frame(&mut self, now: Instant) -> bool {
        if self.target_frames == 0 || self.finished {
            return false;
        }
        if let Some(start) = self.frame_start.take() {
            self.frame_ns.push(now.duration_since(start).as_nanos());
        }
        self.seen_frames += 1;
        self.finished = self.frame_ns.len() as u64 >= self.target_frames;
        self.finished
    }

    fn percentile(&self, percentile: f64) -> u128 {
        percentile_ns(&self.frame_ns, percentile)
    }

    fn dump_json(&self) {
        let elapsed_ns: u128 = self.frame_ns.iter().sum();
        let avg_frame_ns = elapsed_ns / self.frame_ns.len() as u128;
        println!(
            r#"{{"scenario":"{}","warmup_frames":{},"total_frames":{},"elapsed_ns":{},"avg_frame_ns":{},"median_frame_ns":{},"p95_frame_ns":{},"mesh_assets":{},"material_assets":{}}}"#,
            self.scenario,
            self.warmup_frames,
            self.frame_ns.len(),
            elapsed_ns,
            avg_frame_ns,
            self.percentile(0.5),
            self.percentile(0.95),
            self.mesh_assets,
            self.material_assets,
        );
    }
}

fn percentile_ns(samples: &[u128], percentile: f64) -> u128 {
    if samples.is_empty() {
        return 0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index]
}

pub fn bench_begin_frame(mut stats: ResMut<BenchStats>) {
    stats.begin_frame(Instant::now());
}

pub fn bench_finish_frame(mut stats: ResMut<BenchStats>, mut exit: MessageWriter<AppExit>) {
    if stats.finish_frame(Instant::now()) {
        stats.dump_json();
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn calculates_percentiles() {
        assert_eq!(percentile_ns(&[40, 10, 30, 20], 0.5), 30);
        assert_eq!(percentile_ns(&[40, 10, 30, 20], 0.95), 40);
        assert_eq!(percentile_ns(&[], 0.95), 0);
    }

    #[test]
    fn excludes_warmup_frames() {
        let mut stats = BenchStats::default();
        stats.configure("server", 2, 2);
        let start = Instant::now();

        stats.begin_frame(start);
        assert!(!stats.finish_frame(start + Duration::from_nanos(10)));
        stats.begin_frame(start + Duration::from_nanos(20));
        assert!(!stats.finish_frame(start + Duration::from_nanos(30)));
        stats.begin_frame(start + Duration::from_nanos(40));
        assert!(!stats.finish_frame(start + Duration::from_nanos(50)));
        stats.begin_frame(start + Duration::from_nanos(60));
        assert!(stats.finish_frame(start + Duration::from_nanos(80)));

        assert_eq!(stats.frame_ns, vec![10, 20]);
    }

    #[test]
    #[should_panic(expected = "benchmark frames must be greater than zero")]
    fn rejects_zero_frames() {
        BenchStats::default().configure("server", 0, 0);
    }
}
