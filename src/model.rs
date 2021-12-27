use std::fmt;
use std::fmt::Formatter;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
crate enum InstanceStage {
    Stopped,
    Running,
    Deleted,
}

impl fmt::Display for InstanceStage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InstanceStage::Stopped => write!(f, "Stopped"),
            InstanceStage::Running => write!(f, "Running"),
            InstanceStage::Deleted => write!(f, "Deleted"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
crate enum InstanceStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Deleting,
    Error(String),
}

impl fmt::Display for InstanceStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InstanceStatus::Starting => write!(f, "Starting"),
            InstanceStatus::Running => write!(f, "Running"),
            InstanceStatus::Stopping => write!(f, "Stopping"),
            InstanceStatus::Stopped => write!(f, "Stopped"),
            InstanceStatus::Deleting => write!(f, "Deleting"),
            InstanceStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
crate struct Instance {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
    crate hostname: String,
    crate ssh_host: Option<String>,
    crate ssh_port: Option<i32>,
    crate password: String,
    crate stage: InstanceStage,
    crate status: InstanceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
crate struct User {
    crate username: String,
    crate cpu_quota: usize,
    crate memory_quota: usize,
    crate disk_quota: usize,
    crate instance_quota: usize,
    crate instances: Vec<Instance>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
crate struct State {
    crate users: Vec<User>,
}

impl State {
    crate fn new() -> Self {
        Default::default()
    }
}
