pub mod vrtx;
pub mod performance;
#[cfg(feature = "bench")]
pub mod bench;

use bevy::prelude::*;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(performance::PerformancePlugin);
        #[cfg(feature = "bench")]
        {
            app.init_resource::<bench::BenchStats>();
            app.add_systems(First, bench::bench_begin_frame)
                .add_systems(Last, bench::bench_finish_frame);
        }
    }
}
