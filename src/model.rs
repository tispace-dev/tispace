use std::fmt;
use std::fmt::Formatter;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
crate enum InstanceStage {
    Pending,
    Running,
    Deleting,
}

impl fmt::Display for InstanceStage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InstanceStage::Pending => write!(f, "Pending"),
            InstanceStage::Running => write!(f, "Running"),
            InstanceStage::Deleting => write!(f, "Deleting"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
crate enum InstanceStatus {
    Pending,
    Running,
    Deleting,
    Error(String),
}

impl fmt::Display for InstanceStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InstanceStatus::Pending => write!(f, "Pending"),
            InstanceStatus::Running => write!(f, "Running"),
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
    crate stage: InstanceStage,
    crate status: InstanceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
crate struct User {
    crate username: String,
    crate password_hash: String,
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
