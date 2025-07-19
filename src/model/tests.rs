#[cfg(test)]
mod tests {
    use crate::model::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_phase_serialization() {
        let phase = Phase {
            kind: "Phase".to_string(),
            id: "test-phase".to_string(),
            spec: PhaseSpec {
                description: "Test phase description".to_string(),
                selector: Selector {
                    match_labels: {
                        let mut labels = HashMap::new();
                        labels.insert("phase".to_string(), "test".to_string());
                        labels
                    },
                },
                instance_mode: Some("parallel".to_string()),
                wait_for: Some(WaitFor {
                    phases: vec!["dep1".to_string(), "dep2".to_string()],
                    timeout: Some("30s".to_string()),
                }),
                retry: Some(Retry {
                    max_attempts: Some(3),
                }),
                on_failure: Some(Handler {
                    action: Some("continue".to_string()),
                    spec: Some(HandlerSpec {
                        message: vec!["Failure message".to_string()],
                        notify: Some(Notify {
                            email: Some("test@example.com".to_string()),
                            slack: Some("#alerts".to_string()),
                        }),
                        labels: Some({
                            let mut labels = HashMap::new();
                            labels.insert("status".to_string(), "failed".to_string());
                            labels
                        }),
                    }),
                }),
                on_success: None,
            },
        };

        let serialized = serde_json::to_string(&phase).unwrap();
        let deserialized: Phase = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(phase, deserialized);
    }

    #[test]
    fn test_phase_deserialization_minimal() {
        let json = json!({
            "Kind": "Phase",
            "Id": "minimal-phase",
            "Spec": {
                "description": "Minimal phase",
                "selector": {
                    "match_labels": {
                        "phase": "minimal"
                    }
                }
            }
        });

        let phase: Phase = serde_json::from_value(json).unwrap();
        
        assert_eq!(phase.kind, "Phase");
        assert_eq!(phase.id, "minimal-phase");
        assert_eq!(phase.spec.description, "Minimal phase");
        assert_eq!(phase.spec.selector.match_labels.get("phase"), Some(&"minimal".to_string()));
        assert!(phase.spec.wait_for.is_none());
        assert!(phase.spec.retry.is_none());
        assert!(phase.spec.on_failure.is_none());
        assert!(phase.spec.on_success.is_none());
    }

    #[test]
    fn test_phase_deserialization_full() {
        let json = json!({
            "Kind": "Phase",
            "Id": "full-phase",
            "Spec": {
                "description": "Full featured phase",
                "selector": {
                    "match_labels": {
                        "phase": "full",
                        "env": "test"
                    }
                },
                "instance_mode": "sequential",
                "wait_for": {
                    "phases": ["dep1", "dep2"],
                    "timeout": "1m"
                },
                "retry": {
                    "max_attempts": 5
                },
                "onFailure": {
                    "action": "abort",
                    "spec": {
                        "message": ["Phase failed", "Aborting execution"],
                        "notify": {
                            "email": "admin@example.com",
                            "slack": "#critical"
                        },
                        "labels": {
                            "severity": "high",
                            "status": "failed"
                        }
                    }
                },
                "onSuccess": {
                    "action": "log",
                    "spec": {
                        "message": ["Phase completed successfully"],
                        "labels": {
                            "status": "success"
                        }
                    }
                }
            }
        });

        let phase: Phase = serde_json::from_value(json).unwrap();
        
        assert_eq!(phase.kind, "Phase");
        assert_eq!(phase.id, "full-phase");
        assert_eq!(phase.spec.description, "Full featured phase");
        
        // Test selector
        assert_eq!(phase.spec.selector.match_labels.len(), 2);
        assert_eq!(phase.spec.selector.match_labels.get("phase"), Some(&"full".to_string()));
        assert_eq!(phase.spec.selector.match_labels.get("env"), Some(&"test".to_string()));
        
        // Test instance_mode
        assert_eq!(phase.spec.instance_mode, Some("sequential".to_string()));
        
        // Test wait_for
        let wait_for = phase.spec.wait_for.as_ref().unwrap();
        assert_eq!(wait_for.phases, vec!["dep1".to_string(), "dep2".to_string()]);
        assert_eq!(wait_for.timeout, Some("1m".to_string()));
        
        // Test retry
        let retry = phase.spec.retry.as_ref().unwrap();
        assert_eq!(retry.max_attempts, Some(5));
        
        // Test on_failure
        let on_failure = phase.spec.on_failure.as_ref().unwrap();
        assert_eq!(on_failure.action, Some("abort".to_string()));
        let failure_spec = on_failure.spec.as_ref().unwrap();
        assert_eq!(failure_spec.message, vec!["Phase failed".to_string(), "Aborting execution".to_string()]);
        let notify = failure_spec.notify.as_ref().unwrap();
        assert_eq!(notify.email, Some("admin@example.com".to_string()));
        assert_eq!(notify.slack, Some("#critical".to_string()));
        
        // Test on_success
        let on_success = phase.spec.on_success.as_ref().unwrap();
        assert_eq!(on_success.action, Some("log".to_string()));
        let success_spec = on_success.spec.as_ref().unwrap();
        assert_eq!(success_spec.message, vec!["Phase completed successfully".to_string()]);
    }

    #[test]
    fn test_wait_for_defaults() {
        let json = json!({
            "phases": ["dep1"]
        });

        let wait_for: WaitFor = serde_json::from_value(json).unwrap();
        assert_eq!(wait_for.phases, vec!["dep1".to_string()]);
        assert!(wait_for.timeout.is_none());
    }

    #[test]
    fn test_retry_defaults() {
        let json = json!({});

        let retry: Retry = serde_json::from_value(json).unwrap();
        assert!(retry.max_attempts.is_none());
    }

    #[test]
    fn test_handler_spec_defaults() {
        let json = json!({
            "message": ["test message"]
        });

        let handler_spec: HandlerSpec = serde_json::from_value(json).unwrap();
        assert_eq!(handler_spec.message, vec!["test message".to_string()]);
        assert!(handler_spec.notify.is_none());
        assert!(handler_spec.labels.is_none());
    }
}
