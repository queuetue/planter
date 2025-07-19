use crate::executor::driver;
use crate::executor::hooks;
use crate::model::Phase;
use crate::state::redis::RedisClient;
use tokio::time::sleep;

pub async fn run_phase(_client: &RedisClient, phase: &Phase) -> Result<(), String> {
    println!("Running phase: {}", phase.id);

    // Handle waitFor timeout
    if let Some(wait) = &phase.spec.wait_for {
        if let Some(timeout_str) = &wait.timeout {
            if let Ok(dur) = humantime::parse_duration(timeout_str) {
                println!("Waiting {:?} before executing {}", dur, phase.id);
                sleep(dur).await;
            }
        }
    }

    let mut attempts = 0;
    let max_attempts = phase
        .spec
        .retry
        .as_ref()
        .and_then(|r| r.max_attempts)
        .unwrap_or(1);

    while attempts < max_attempts {
        attempts += 1;
        println!("Attempt {} of {} for phase {}", attempts, max_attempts, phase.id);

        let result = driver::execute(&phase).await;

        match result {
            Ok(_) => {
                hooks::handle_success(&phase).await;
                return Ok(());
            }
            Err(err) => {
                eprintln!("Phase {} attempt {} failed: {}", phase.id, attempts, err);
                if attempts == max_attempts {
                    hooks::handle_failure(&phase).await;
                    return Err(err);
                }
            }
        }
    }

    Err(format!("Phase {} failed after {} attempts", phase.id, attempts))
}
