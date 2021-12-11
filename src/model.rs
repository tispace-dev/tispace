use std::fmt;
use std::fmt::Formatter;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum InstanceStage {
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
pub enum InstanceStatus {
    Pending,
    Running,
    Deleting,
    Error,
}

impl fmt::Display for InstanceStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InstanceStatus::Pending => write!(f, "Pending"),
            InstanceStatus::Running => write!(f, "Running"),
            InstanceStatus::Deleting => write!(f, "Deleting"),
            InstanceStatus::Error => write!(f, "Error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub cpu: usize,
    pub memory: usize,
    pub disk_size: usize,
    pub domain_name: String,
    pub stage: InstanceStage,
    pub status: InstanceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub cpu_quota: usize,
    pub memory_quota: usize,
    pub disk_quota: usize,
    pub instance_quota: usize,
    pub instances: Vec<Instance>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct State {
    pub users: Vec<User>,
    pub secret: String,
}

impl State {
    pub fn new() -> Self {
        Default::default()
    }
}
