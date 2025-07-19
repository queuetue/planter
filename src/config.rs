use std::path::PathBuf;

pub fn planter_root() -> PathBuf {
    match std::env::var("PLANTER_ROOT") {
        Ok(s) if !s.is_empty() => PathBuf::from(s),
        _ => PathBuf::from("/etc/planter"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_planter_root_default() {
        // Set empty override to trigger default
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("PLANTER_ROOT", "");
        let root = planter_root();
        assert_eq!(root, PathBuf::from("/etc/planter"));
    }

    #[test]
    fn test_planter_root_override() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("PLANTER_ROOT", "/tmp/custom");
        let root = planter_root();
        assert_eq!(root, PathBuf::from("/tmp/custom"));
    }

    #[test]
    fn test_state_file_path() {
        let _guard = ENV_MUTEX.lock().unwrap();
        env::set_var("PLANTER_ROOT", "/tmp/custom2");
        let path = state_file_path();
        // Ends with state/state.json
        let path_str = path.to_string_lossy();
        assert!(path_str.ends_with("state/state.json"));
        assert!(path_str.starts_with("/tmp/custom2"));
    }
}

pub fn state_file_path() -> PathBuf {
    let mut root = planter_root();
    root.push("state");
    std::fs::create_dir_all(&root).ok();
    root.push("state.json");
    root
}
use std::env;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub redis_url: Option<String>,
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 3030,
            redis_url: None,
            log_level: "info".to_string(),
        }
    }
}

#[allow(dead_code)]
impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3030),
            redis_url: env::var("REDIS_URL").ok(),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        }
    }
}