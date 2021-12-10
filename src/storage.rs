use std::fmt;
use std::fmt::Formatter;
use std::io::ErrorKind;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::error::*;

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
    pub password: String,
    pub cpu_quota: usize,
    pub memory_quota: usize,
    pub disk_quota: usize,
    pub instances: Vec<Instance>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct State {
    pub users: Vec<User>,
}

impl State {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, Clone)]
pub struct Storage {
    path: String,
    state: Arc<RwLock<State>>,
}

impl Storage {
    pub async fn from_path(path: &str) -> Result<Self> {
        let mut state = State::new();
        match tokio::fs::read(path).await {
            Ok(contents) => {
                state = serde_json::from_slice(&contents)?;
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(Box::new(e)),
        }
        Ok(Storage {
            path: path.to_string(),
            state: Arc::new(RwLock::new(state)),
        })
    }

    pub fn read_only<F>(&self, mut f: F)
    where
        F: FnMut(&State),
    {
        f(&*self.state.read().unwrap())
    }

    pub async fn read_write<F>(&self, mut f: F) -> Result<()>
    where
        F: FnMut(&mut State) -> bool,
    {
        if f(&mut *self.state.write().unwrap()) {
            let state = self.dump();
            let data = serde_json::to_vec(&state).unwrap();
            let tmp_path = format!("{}.tmp", self.path);
            tokio::fs::write(&tmp_path, data).await?;
            tokio::fs::rename(&tmp_path, &self.path).await?;
        }
        Ok(())
    }

    pub fn dump(&self) -> State {
        let state = &*self.state.read().unwrap();
        state.clone()
    }
}
