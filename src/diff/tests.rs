#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::collections::HashMap;

    fn create_test_phase(id: &str, description: &str) -> Phase {
        Phase {
            kind: "Phase".to_string(),
            id: id.to_string(),
            spec: PhaseSpec {
                description: description.to_string(),
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

    #[test]
    fn test_diff_empty_plans() {
        let current: Vec<Phase> = vec![];
        let incoming: Vec<Phase> = vec![];
        
        let diff = diff_plans(&current, &incoming);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_diff_add_phases() {
        let current: Vec<Phase> = vec![];
        let incoming = vec![
            create_test_phase("phase1", "First phase"),
            create_test_phase("phase2", "Second phase"),
        ];
        
        let diff = diff_plans(&current, &incoming);
        assert_eq!(diff.len(), 2);
        
        match &diff[0] {
            DiffResult::Add(phase) => assert_eq!(phase.id, "phase1"),
            _ => panic!("Expected Add operation"),
        }
        
        match &diff[1] {
            DiffResult::Add(phase) => assert_eq!(phase.id, "phase2"),
            _ => panic!("Expected Add operation"),
        }
    }

    #[test]
    fn test_diff_delete_phases() {
        let current = vec![
            create_test_phase("phase1", "First phase"),
            create_test_phase("phase2", "Second phase"),
        ];
        let incoming: Vec<Phase> = vec![];
        
        let diff = diff_plans(&current, &incoming);
        assert_eq!(diff.len(), 2);
        
        // Results could be in any order, so check both phases exist
        let deleted_ids: Vec<&str> = diff.iter().map(|d| match d {
            DiffResult::Delete(phase) => phase.id.as_str(),
            _ => panic!("Expected Delete operation"),
        }).collect();
        
        assert!(deleted_ids.contains(&"phase1"));
        assert!(deleted_ids.contains(&"phase2"));
    }

    #[test]
    fn test_diff_update_phases() {
        let current = vec![create_test_phase("phase1", "Original description")];
        let incoming = vec![create_test_phase("phase1", "Updated description")];
        
        let diff = diff_plans(&current, &incoming);
        assert_eq!(diff.len(), 1);
        
        match &diff[0] {
            DiffResult::Update { old, new } => {
                assert_eq!(old.id, "phase1");
                assert_eq!(old.spec.description, "Original description");
                assert_eq!(new.id, "phase1");
                assert_eq!(new.spec.description, "Updated description");
            }
            _ => panic!("Expected Update operation"),
        }
    }

    #[test]
    fn test_diff_no_change() {
        let current = vec![create_test_phase("phase1", "Same description")];
        let incoming = vec![create_test_phase("phase1", "Same description")];
        
        let diff = diff_plans(&current, &incoming);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_diff_mixed_operations() {
        let current = vec![
            create_test_phase("phase1", "Keep this"),
            create_test_phase("phase2", "Update this"),
            create_test_phase("phase3", "Delete this"),
        ];
        
        let incoming = vec![
            create_test_phase("phase1", "Keep this"), // No change
            create_test_phase("phase2", "Updated description"), // Update
            create_test_phase("phase4", "Add this"), // Add
        ];
        
        let diff = diff_plans(&current, &incoming);
        assert_eq!(diff.len(), 3); // 1 update, 1 delete, 1 add
        
        let mut has_update = false;
        let mut has_delete = false;
        let mut has_add = false;
        
        for result in &diff {
            match result {
                DiffResult::Update { old, new } => {
                    assert_eq!(old.id, "phase2");
                    assert_eq!(new.spec.description, "Updated description");
                    has_update = true;
                }
                DiffResult::Delete(phase) => {
                    assert_eq!(phase.id, "phase3");
                    has_delete = true;
                }
                DiffResult::Add(phase) => {
                    assert_eq!(phase.id, "phase4");
                    has_add = true;
                }
            }
        }
        
        assert!(has_update);
        assert!(has_delete);
        assert!(has_add);
    }

    #[test]
    fn test_diff_same_id_different_kind() {
        let current = vec![Phase {
            kind: "PhaseA".to_string(),
            id: "same-id".to_string(),
            spec: PhaseSpec {
                description: "Original".to_string(),
                selector: Selector {
                    match_labels: HashMap::new(),
                },
                instance_mode: None,
                wait_for: None,
                retry: None,
                on_failure: None,
                on_success: None,
            },
        }];
        
        let incoming = vec![Phase {
            kind: "PhaseB".to_string(),
            id: "same-id".to_string(),
            spec: PhaseSpec {
                description: "New".to_string(),
                selector: Selector {
                    match_labels: HashMap::new(),
                },
                instance_mode: None,
                wait_for: None,
                retry: None,
                on_failure: None,
                on_success: None,
            },
        }];
        
        let diff = diff_plans(&current, &incoming);
        assert_eq!(diff.len(), 2); // Delete old, Add new (different kind+id combination)
        
        let mut has_delete = false;
        let mut has_add = false;
        
        for result in &diff {
            match result {
                DiffResult::Delete(phase) => {
                    assert_eq!(phase.kind, "PhaseA");
                    assert_eq!(phase.id, "same-id");
                    has_delete = true;
                }
                DiffResult::Add(phase) => {
                    assert_eq!(phase.kind, "PhaseB");
                    assert_eq!(phase.id, "same-id");
                    has_add = true;
                }
                _ => panic!("Expected Delete and Add operations"),
            }
        }
        
        assert!(has_delete);
        assert!(has_add);
    }
}
