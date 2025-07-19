use crate::model::Phase;
use std::collections::HashMap;

#[derive(Debug)]
pub enum DiffResult {
    Add(Phase),
    Update { old: Phase, new: Phase },
    Delete(Phase),
}

/// Compute the difference between two sets of phases.
/// `current` is the last applied state.
/// `incoming` is the new plan being submitted.
pub fn diff_plans(
    current: &[Phase],
    incoming: &[Phase],
) -> Vec<DiffResult> {
    let mut results = vec![];

    let current_map: HashMap<_, _> = current.iter()
        .map(|p| ((p.kind.clone(), p.id.clone()), p))
        .collect();

    let incoming_map: HashMap<_, _> = incoming.iter()
        .map(|p| ((p.kind.clone(), p.id.clone()), p))
        .collect();

    // Detect additions and updates
    for (key, new_phase) in &incoming_map {
        match current_map.get(key) {
            None => {
                results.push(DiffResult::Add((*new_phase).clone()));
            }
            Some(old_phase) => {
                if old_phase.spec != new_phase.spec {
                    results.push(DiffResult::Update {
                        old: (*old_phase).clone(),
                        new: (*new_phase).clone(),
                    });
                }
            }
        }
    }

    // Detect deletions
    for (key, old_phase) in &current_map {
        if !incoming_map.contains_key(key) {
            results.push(DiffResult::Delete((*old_phase).clone()));
        }
    }

    results
}

#[cfg(test)]
mod tests;
