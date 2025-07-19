use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    PhaseReceived(String),
    PhaseExecuted { id: String, success: bool },
    DiffComputed { adds: usize, updates: usize, deletes: usize },
    PlanSubmitted { plan_id: String, phases_count: usize },
    PlanApplied { plan_id: String },
    DiffResult { plan_id: String, changes: Vec<String> },
    Error(String),
}

pub trait EventBus: Send + Sync {
    fn publish(&self, event: Event);
}

#[derive(Clone)]
pub struct DefaultBus;

impl EventBus for DefaultBus {
    fn publish(&self, event: Event) {
        println!("[event] {:?}", event);
    }
}

lazy_static::lazy_static! {
    static ref GLOBAL_BUS: Arc<Mutex<Box<dyn EventBus>>> =
        Arc::new(Mutex::new(Box::new(DefaultBus)));
}

pub fn set_bus(bus: Box<dyn EventBus>) {
    let mut global = GLOBAL_BUS.lock().unwrap();
    *global = bus;
}

pub fn get_bus() -> Arc<Mutex<Box<dyn EventBus>>> {
    Arc::clone(&GLOBAL_BUS)
}
