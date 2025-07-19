use crate::log::event_bus::{get_bus, Event};

pub fn log_event(event: Event) {
    let bus = get_bus();
    let bus = bus.lock().unwrap();
    bus.publish(event);
}

pub fn init_logger() {
    // Extend this to support env-configured backends or log levels
    log_event(Event::PhaseReceived("Logger initialized".into()));
}
