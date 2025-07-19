#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::executor::hooks::{handle_success, handle_failure};
    use std::collections::HashMap;

    fn create_test_phase_with_handlers(id: &str) -> Phase {
        Phase {
            kind: "Phase".to_string(),
            id: id.to_string(),
            spec: PhaseSpec {
                description: format!("Test phase {}", id),
                selector: Selector {
                    match_labels: {
                        let mut labels = HashMap::new();
                        labels.insert("phase".to_string(), id.to_string());
                        labels
                    },
                },
                instance_mode: None,
                wait_for: None,
                retry: None,
                on_failure: Some(Handler {
                    action: Some("log".to_string()),
                    spec: Some(HandlerSpec {
                        message: vec![format!("Phase {} failed", id)],
                        notify: Some(Notify {
                            email: Some("admin@example.com".to_string()),
                            slack: Some("#alerts".to_string()),
                        }),
                        labels: Some({
                            let mut labels = HashMap::new();
                            labels.insert("status".to_string(), "failed".to_string());
                            labels
                        }),
                    }),
                }),
                on_success: Some(Handler {
                    action: Some("log".to_string()),
                    spec: Some(HandlerSpec {
                        message: vec![format!("Phase {} succeeded", id)],
                        notify: None,
                        labels: Some({
                            let mut labels = HashMap::new();
                            labels.insert("status".to_string(), "success".to_string());
                            labels
                        }),
                    }),
                }),
            },
        }
    }

    fn create_test_phase_no_handlers(id: &str) -> Phase {
        Phase {
            kind: "Phase".to_string(),
            id: id.to_string(),
            spec: PhaseSpec {
                description: format!("Test phase {}", id),
                selector: Selector {
                    match_labels: {
                        let mut labels = HashMap::new();
                        labels.insert("phase".to_string(), id.to_string());
                        labels
                    },
                },
                instance_mode: None,
                wait_for: None,
                retry: None,
                on_failure: None,
                on_success: None,
            },
        }
    }

    #[tokio::test]
    async fn test_handle_success_with_handler() {
        let phase = create_test_phase_with_handlers("test1");
        
        // This test mainly ensures the function doesn't panic
        // In a real implementation, we'd want to capture the output
        handle_success(&phase).await;
    }

    #[tokio::test]
    async fn test_handle_success_no_handler() {
        let phase = create_test_phase_no_handlers("test1");
        
        // This should do nothing and not panic
        handle_success(&phase).await;
    }

    #[tokio::test]
    async fn test_handle_failure_with_handler() {
        let phase = create_test_phase_with_handlers("test1");
        
        // This test mainly ensures the function doesn't panic
        handle_failure(&phase).await;
    }

    #[tokio::test]
    async fn test_handle_failure_no_handler() {
        let phase = create_test_phase_no_handlers("test1");
        
        // This should do nothing and not panic
        handle_failure(&phase).await;
    }

    #[tokio::test]
    async fn test_handler_with_minimal_spec() {
        let phase = Phase {
            kind: "Phase".to_string(),
            id: "minimal".to_string(),
            spec: PhaseSpec {
                description: "Minimal test".to_string(),
                selector: Selector {
                    match_labels: HashMap::new(),
                },
                instance_mode: None,
                wait_for: None,
                retry: None,
                on_failure: Some(Handler {
                    action: Some("continue".to_string()),
                    spec: None, // No spec
                }),
                on_success: None,
            },
        };
        
        // Should handle missing spec gracefully
        handle_failure(&phase).await;
    }
}
