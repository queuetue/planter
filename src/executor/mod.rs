pub mod driver;
pub mod runner;
pub mod hooks;

use crate::model::Phase;
use crate::state::tracker::store_applied_plan;
use crate::state::redis::RedisClient;

pub async fn execute_plan(client: &RedisClient, phases: &[Phase]) {
    for phase in phases {
        if let Err(e) = runner::run_phase(client, phase).await {
            eprintln!("Phase {} failed: {}", phase.id, e);
        }
    }

    store_applied_plan(client, phases).await;
}
