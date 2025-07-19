mod log;
mod model;
mod diff;
mod executor;
mod state;
mod config;
mod nats;

use axum::{routing::{get, post}, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use routes::plan::{submit_plan, AppState};
use log::{init_logger, LoggingService};
use sha2::{Sha256, Digest};

mod routes;
#[tokio::main]
async fn main() {
    use axum::extract::State;
    use axum::response::IntoResponse;
    use axum::Json;
    use std::sync::Arc;
    use tokio::sync::Notify;
    use crate::state::tracker::{save_state_file, load_state_file};

    let shutdown_notify = Arc::new(Notify::new());

    // Initialize Redis client if configured
    let redis_client = if let Ok(redis_url) = std::env::var("REDIS_URL") {
        println!("Connecting to Redis at {}", redis_url);
        match state::redis::connect(&redis_url).await {
            Ok(client) => {
                println!("Redis client initialized successfully");
                Some(std::sync::Arc::new(client))
            }
            Err(e) => {
                eprintln!("Failed to connect to Redis: {}", e);
                None
            }
        }
    } else {
        println!("No REDIS_URL configured - running in stateless mode");
        None
    };

    // Initialize NATS client if configured
    let nats_client = if let Ok(nats_url) = std::env::var("NATS_URL") {
        println!("Connecting to NATS at {}", nats_url);
        match crate::nats::client::NatsClient::connect(&nats_url).await {
            Ok(client) => {
                println!("Connected to NATS successfully");
                Some(std::sync::Arc::new(client))
            }
            Err(e) => {
                eprintln!("Failed to connect to NATS: {}", e);
                None
            }
        }
    } else {
        println!("No NATS_URL configured - skipping NATS integration");
        None
    };

    // On startup, try to load state from file or sync from remote
    let mut initial_state = load_state_file();

    // If PLANTER_SYNC_FROM is set, fetch state from remote Planter
    let sync_url = std::env::var("PLANTER_SYNC_FROM").ok();
    if initial_state.is_none() {
        if let Some(ref sync_url) = sync_url {
            println!("Syncing initial state from remote Planter at {}", sync_url);
            match reqwest::get(format!("{}/state", sync_url)).await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<serde_json::Value>().await {
                            Ok(json) => {
                                if let Some(phases) = json.get("phases").and_then(|p| serde_json::from_value::<Vec<model::Phase>>(p.clone()).ok()) {
                                    initial_state = Some(phases);
                                    println!("Loaded {} phases from remote Planter", json.get("phases").and_then(|p| p.as_array()).map_or(0, |arr| arr.len()));
                                } else {
                                    println!("Remote state response missing 'phases' field");
                                }
                            }
                            Err(e) => println!("Failed to parse remote state JSON: {}", e),
                        }
                    } else {
                        println!("Remote Planter /state returned status: {}", resp.status());
                    }
                }
                Err(e) => println!("Failed to fetch remote state: {}", e),
            }
        }
    }

    if let Some(phases) = initial_state {
        if let Some(redis_client) = &redis_client {
            crate::state::tracker::store_current_plan(redis_client, &phases).await;
        }
    }

    // If PLANTER_SYNC_INTERVAL is set, periodically poll remote Planter and update state
    if let (Some(sync_url), Ok(interval_str)) = (sync_url, std::env::var("PLANTER_SYNC_INTERVAL")) {
        if let Ok(interval) = interval_str.parse::<u64>() {
            let redis_client = redis_client.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                    println!("Periodic sync: fetching state from {}", sync_url);
                    match reqwest::get(format!("{}/state", sync_url)).await {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                match resp.json::<serde_json::Value>().await {
                                    Ok(json) => {
                                        if let Some(phases) = json.get("phases").and_then(|p| serde_json::from_value::<Vec<model::Phase>>(p.clone()).ok()) {
                                            if let Some(redis_client) = &redis_client {
                                                crate::state::tracker::store_current_plan(redis_client, &phases).await;
                                                println!("Periodic sync: updated local state with {} phases", json.get("phases").and_then(|p| p.as_array()).map_or(0, |arr| arr.len()));
                                            }
                                        } else {
                                            println!("Periodic sync: remote state missing 'phases' field");
                                        }
                                    }
                                    Err(e) => println!("Periodic sync: failed to parse remote state JSON: {}", e),
                                }
                            } else {
                                println!("Periodic sync: remote Planter /state returned status: {}", resp.status());
                            }
                        }
                        Err(e) => println!("Periodic sync: failed to fetch remote state: {}", e),
                    }
                }
            });
        }
    }

    // /STOP endpoint handler
    let shutdown_notify_clone = shutdown_notify.clone();
    let stop_handler = move |State(app_state): State<AppState>| {
        let shutdown_notify_clone = shutdown_notify_clone.clone();
        async move {
            // Save state to file
            if let Some(redis_client) = &app_state.redis_client {
                if let Some(phases) = crate::state::tracker::load_current_plan(redis_client).await {
                    let _ = save_state_file(&phases);
                }
            }
            shutdown_notify_clone.notify_waiters();
            Json(serde_json::json!({ "status": "stopping", "message": "State saved, shutting down" }))
        }
    };

    // /RELOAD endpoint handler
    async fn reload_handler(State(app_state): State<AppState>) -> impl IntoResponse {
        let loaded = load_state_file();
        if let Some(phases) = loaded {
            if let Some(redis_client) = &app_state.redis_client {
                crate::state::tracker::store_current_plan(redis_client, &phases).await;
                return Json(serde_json::json!({ "status": "reloaded", "count": phases.len() }));
            }
        }
        Json(serde_json::json!({ "status": "no_state", "message": "No state file found or failed to load" }))
    }
    init_logger();

    // Initialize Redis client if configured
    let redis_client = if let Ok(redis_url) = std::env::var("REDIS_URL") {
        println!("Connecting to Redis at {}", redis_url);
        match state::redis::connect(&redis_url).await {
            Ok(client) => {
                println!("Redis client initialized successfully");
                Some(std::sync::Arc::new(client))
            }
            Err(e) => {
                eprintln!("Failed to connect to Redis: {}", e);
                None
            }
        }
    } else {
        println!("No REDIS_URL configured - running in stateless mode");
        None
    };

    // Initialize NATS client if configured
    let nats_client = if let Ok(nats_url) = std::env::var("NATS_URL") {
        println!("Connecting to NATS at {}", nats_url);
        match crate::nats::client::NatsClient::connect(&nats_url).await {
            Ok(client) => {
                println!("Connected to NATS successfully");
                Some(std::sync::Arc::new(client))
            }
            Err(e) => {
                eprintln!("Failed to connect to NATS: {}", e);
                None
            }
        }
    } else {
        println!("No NATS_URL configured - skipping NATS integration");
        None
    };

    // Get prefix from environment variable
    let prefix = std::env::var("PLANTER_PREFIX").unwrap_or_else(|_| "".to_string());
    let prefix = prefix.trim_end_matches('/').to_string();

    // Compute tenant namespace key by hashing the API prefix
    let tenant_key = if prefix.is_empty() {
        "global".to_string()
    } else {
        // Use SHA-256 and hex encoding for obfuscated namespace
        let mut hasher = Sha256::new();
        hasher.update(prefix.as_bytes());
        hex::encode(hasher.finalize())
    };
    // Expose tenant key for tracker and storage
    std::env::set_var("TENANT_KEY", &tenant_key);
    let app_state = AppState {
        redis_client: redis_client.clone(),
        nats_client: nats_client.clone(),
        logging_service: LoggingService::new(redis_client.clone().map(|c| (*c).clone())),
        tenant_key: tenant_key.clone(),
    };

    // Helper to prepend prefix to a route
    let route = |path: &str| {
        if prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", prefix, path.trim_start_matches('/'))
        }
    };

    let app = Router::new()
        .route(&route("/plan"), post(submit_plan))
        .route(&route("/manifest"), post(routes::manifest::submit_manifest))
        .route(&route("/state"), get(routes::state::get_state))
        .route(&route("/diff"), get(routes::diff::get_diff))
        .route(&route("/logs"), get(routes::logs::get_logs))
        .route(&route("/phases/:id"), get(routes::phases::get_phase))
        .route(&route("/apply"), post(routes::apply::apply_plan))
        .route(&route("/health"), get(routes::health::health_check))
        .route(&route("/ready"), get(routes::health::readiness_check))
        .route(&route("/metrics"), get(routes::health::metrics))
        .route(&route("/STOP"), post(stop_handler))
        .route(&route("/RELOAD"), post(reload_handler))
        .with_state(app_state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3030));
    println!("Listening on {}", addr);

    let listener = TcpListener::bind(&addr).await.unwrap();
    // Handle signals for graceful shutdown and reload
    let shutdown_notify_signal = shutdown_notify.clone();
    let shutdown_fut = async move {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sighup = signal(SignalKind::hangup()).unwrap();
        tokio::select! {
            _ = sigterm.recv() => {
                println!("Received SIGTERM, saving state and shutting down...");
                if let Some(redis_client) = &app_state.redis_client {
                    if let Some(phases) = crate::state::tracker::load_current_plan(redis_client).await {
                        let _ = save_state_file(&phases);
                    }
                }
                shutdown_notify_signal.notify_waiters();
            }
            _ = sigint.recv() => {
                println!("Received SIGINT, saving state and shutting down...");
                if let Some(redis_client) = &app_state.redis_client {
                    if let Some(phases) = crate::state::tracker::load_current_plan(redis_client).await {
                        let _ = save_state_file(&phases);
                    }
                }
                shutdown_notify_signal.notify_waiters();
            }
            _ = sighup.recv() => {
                println!("Received SIGHUP, reloading state from file...");
                let loaded = load_state_file();
                if let Some(phases) = loaded {
                    if let Some(redis_client) = &app_state.redis_client {
                        crate::state::tracker::store_current_plan(redis_client, &phases).await;
                    }
                }
            }
            _ = shutdown_notify.notified() => {
                println!("Shutdown requested via /STOP endpoint");
            }
        }
    };

    tokio::select! {
        _ = axum::serve(listener, app) => {},
        _ = shutdown_fut => {},
    }
}
