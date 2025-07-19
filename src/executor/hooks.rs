use crate::model::{Handler, Phase};

pub async fn handle_success(phase: &Phase) {
    if let Some(handler) = &phase.spec.on_success {
        run_handler("Success", handler).await;
    }
}

pub async fn handle_failure(phase: &Phase) {
    if let Some(handler) = &phase.spec.on_failure {
        run_handler("Failure", handler).await;
    }
}

async fn run_handler(label: &str, handler: &Handler) {
    println!("Running {} handler", label);
    if let Some(spec) = &handler.spec {
        for msg in &spec.message {
            println!("[{}] {}", label, msg);
        }

        if let Some(notify) = &spec.notify {
            if let Some(email) = &notify.email {
                println!("[Notify] email => {}", email);
            }
            if let Some(slack) = &notify.slack {
                println!("[Notify] slack => {}", slack);
            }
        }
    }
}

#[cfg(test)]
mod tests;
