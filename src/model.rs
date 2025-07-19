use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Phase {
    pub kind: String,
    pub id: String,
    pub spec: PhaseSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseSpec {
    pub description: String,
    pub selector: Selector,
    #[serde(default)]
    pub instance_mode: Option<String>,
    #[serde(default)]
    pub wait_for: Option<WaitFor>,
    #[serde(default)]
    pub retry: Option<Retry>,
    #[serde(default, rename = "onFailure")]
    pub on_failure: Option<Handler>,
    #[serde(default, rename = "onSuccess")]
    pub on_success: Option<Handler>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Selector {
    pub match_labels: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaitFor {
    #[serde(default)]
    pub phases: Vec<String>,
    #[serde(default)]
    pub timeout: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Retry {
    #[serde(default)]
    pub max_attempts: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Handler {
    pub action: Option<String>,
    #[serde(default)]
    pub spec: Option<HandlerSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandlerSpec {
    #[serde(default)]
    pub message: Vec<String>,
    #[serde(default)]
    pub notify: Option<Notify>,
    #[serde(default)]
    pub labels: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notify {
    pub email: Option<String>,
    pub slack: Option<String>,
}

#[cfg(test)]
mod tests;
