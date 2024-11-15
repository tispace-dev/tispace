use std::collections::HashMap;
use std::fmt::Formatter;
use std::{fmt, str::FromStr};

use anyhow::{anyhow, Error, Result};
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

#[derive(Debug, Clone, Eq, PartialEq)]
crate enum InstanceStatus {
    Creating,
    Starting,
    Running,
    Stopping,
    Stopped,
    Deleting,
    Missing,
    Error(String),
}

impl fmt::Display for InstanceStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InstanceStatus::Creating => write!(f, "Creating"),
            InstanceStatus::Starting => write!(f, "Starting"),
            InstanceStatus::Running => write!(f, "Running"),
            InstanceStatus::Stopping => write!(f, "Stopping"),
            InstanceStatus::Stopped => write!(f, "Stopped"),
            InstanceStatus::Deleting => write!(f, "Deleting"),
            InstanceStatus::Missing => write!(f, "Missing"),
            InstanceStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl Serialize for InstanceStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for InstanceStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Creating" => Ok(InstanceStatus::Creating),
            "Starting" => Ok(InstanceStatus::Starting),
            "Running" => Ok(InstanceStatus::Running),
            "Stopping" => Ok(InstanceStatus::Stopping),
            "Stopped" => Ok(InstanceStatus::Stopped),
            "Deleting" => Ok(InstanceStatus::Deleting),
            "Missing" => Ok(InstanceStatus::Missing),
            _ if s.starts_with("Error:") => {
                let e = s.strip_prefix("Error:").unwrap().trim();
                Ok(InstanceStatus::Error(e.to_string()))
            }
            _ => Err(SerdeError::custom(format!(
                "invalid instance status: {}",
                s
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
crate enum Runtime {
    Kata,
    Runc,
    Lxc,
    Kvm,
}

impl fmt::Display for Runtime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Runtime::Kata => write!(f, "kata"),
            Runtime::Runc => write!(f, "runc"),
            Runtime::Lxc => write!(f, "lxc"),
            Runtime::Kvm => write!(f, "kvm"),
        }
    }
}

impl FromStr for Runtime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "kata" => Ok(Self::Kata),
            "runc" => Ok(Self::Runc),
            "lxc" => Ok(Self::Lxc),
            "kvm" => Ok(Self::Kvm),
            _ => Err(anyhow!("invalid runtime {}", s)),
        }
    }
}

impl<'de> Deserialize<'de> for Runtime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        Runtime::from_str(&s).map_err(|_| SerdeError::custom(format!("invalid runtime {}", s)))
    }
}

impl Runtime {
    crate fn supported_images(&self) -> Vec<Image> {
        match self {
            Runtime::Kata => Vec::new(),
            Runtime::Runc => Vec::new(),
            Runtime::Lxc => vec![
                Image::CentOS7,
                Image::CentOS9Stream,
                Image::Ubuntu2004,
                Image::Ubuntu2204,
            ],
            Runtime::Kvm => vec![
                Image::CentOS7,
                Image::CentOS9Stream,
                Image::Ubuntu2004,
                Image::Ubuntu2204,
            ],
        }
    }

    crate fn compatiable_with(&self, other: &Runtime) -> bool {
        if self == other {
            return true;
        }
        matches!((self, other), |(Runtime::Kata, Runtime::Runc)| (
            Runtime::Runc,
            Runtime::Kata
        ))
    }
}

#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
crate enum Image {
    CentOS7,
    CentOS8,
    CentOS9Stream,
    Ubuntu2004,
    Ubuntu2204,
}

impl fmt::Display for Image {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Image::CentOS7 => write!(f, "centos:7"),
            Image::CentOS8 => write!(f, "centos:8"),
            Image::CentOS9Stream => write!(f, "centos:9-Stream"),
            Image::Ubuntu2004 => write!(f, "ubuntu:20.04"),
            Image::Ubuntu2204 => write!(f, "ubuntu:22.04"),
        }
    }
}

impl FromStr for Image {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        if lower.starts_with("tispace/centos7:") {
            return Ok(Self::CentOS7);
        }
        if lower.starts_with("tispace/centos8:") {
            return Ok(Self::CentOS8);
        }
        if lower.starts_with("tispace/centos9-stream:") {
            return Ok(Self::CentOS9Stream);
        }
        if lower.starts_with("tispace/ubuntu2004:") {
            return Ok(Self::Ubuntu2004);
        }
        return match lower.as_str() {
            "tispace/centos7" | "centos7" | "centos:7" => Ok(Self::CentOS7),
            "tispace/centos8" | "centos8" | "centos:8" => Ok(Self::CentOS8),
            "tispace/centos9-stream" | "centos9-stream" | "centos:9-stream" => {
                Ok(Self::CentOS9Stream)
            }
            "tispace/ubuntu2004" | "ubuntu2004" | "ubuntu:20.04" => Ok(Self::Ubuntu2004),
            "ubuntu2204" | "ubuntu:22.04" => Ok(Self::Ubuntu2204),
            _ => Err(anyhow!("invalid image {}", s)),
        };
    }
}

impl<'de> Deserialize<'de> for Image {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Image::from_str(&s).map_err(|_| SerdeError::custom(format!("invalid image {}", s)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
crate struct Instance {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
    crate image: Image,
    // Deprecated: hostname is now the same as name.
    crate hostname: String,
    // Deprecated: use external_ip instead.
    crate ssh_host: Option<String>,
    // Deprecated: use 22 instead.
    crate ssh_port: Option<i32>,
    crate password: String,
    crate stage: InstanceStage,
    crate status: InstanceStatus,
    crate internal_ip: Option<String>,
    crate external_ip: Option<String>,
    crate runtime: Runtime,
    crate node_name: Option<String>,
    crate storage_pool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
crate struct User {
    crate username: String,
    crate cpu_quota: usize,
    crate memory_quota: usize,
    crate disk_quota: usize,
    crate instance_quota: usize,
    crate instances: Vec<Instance>,
}

impl User {
    #[allow(dead_code)]
    crate fn find_instance(&self, name: &str) -> Option<&Instance> {
        self.instances.iter().find(|i| i.name == name)
    }

    crate fn find_mut_instance(&mut self, name: &str) -> Option<&mut Instance> {
        self.instances.iter_mut().find(|i| i.name == name)
    }

    crate fn remove_instance(&mut self, name: &str) {
        self.instances
            .iter_mut()
            .position(|i| i.name == name)
            .map(|i| self.instances.remove(i));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
crate struct Node {
    crate name: String,
    crate storage_pools: Vec<StoragePool>,
    crate runtimes: Vec<Runtime>,
    crate cpu_total: usize,
    crate cpu_allocated: usize,
    crate memory_total: usize,
    crate memory_allocated: usize,
    crate storage_total: usize,
    crate storage_used: usize,
    crate storage_allocated: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
crate struct StoragePool {
    crate name: String,
    crate total: usize,
    crate used: usize,
    crate allocated: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Eq, PartialEq)]
crate struct State {
    crate users: Vec<User>,
    #[serde(default)]
    crate nodes: Vec<Node>,
}

impl State {
    crate fn find_user(&self, username: &str) -> Option<&User> {
        self.users.iter().find(|u| u.username == username)
    }

    crate fn find_mut_user(&mut self, username: &str) -> Option<&mut User> {
        self.users.iter_mut().find(|u| u.username == username)
    }

    crate fn sync_allocated_resources(&mut self) {
        let mut cpu_allocated: HashMap<String, usize> = HashMap::new();
        let mut memory_allocated: HashMap<String, usize> = HashMap::new();
        // Map of (node_name, storage_pool) to the allocated capacity of each storage pool.
        let mut storage_allocated: HashMap<(String, String), usize> = HashMap::new();
        // Map of node name to total allocated capacity of all storage pools on each node.
        let mut node_storage_allocated_total: HashMap<String, usize> = HashMap::new();

        for u in &mut self.users {
            for i in &mut u.instances {
                if let Some(node_name) = &i.node_name {
                    *cpu_allocated.entry(node_name.clone()).or_default() += i.cpu;
                    *memory_allocated.entry(node_name.clone()).or_default() += i.memory;
                    if let Some(storage_pool) = &i.storage_pool {
                        *storage_allocated
                            .entry((node_name.clone(), storage_pool.clone()))
                            .or_default() += i.disk_size;
                    }
                    *node_storage_allocated_total
                        .entry(node_name.clone())
                        .or_default() += i.disk_size;
                }
            }
        }

        for node in &mut self.nodes {
            node.cpu_allocated = cpu_allocated.get(&node.name).cloned().unwrap_or_default();
            node.memory_allocated = memory_allocated
                .get(&node.name)
                .cloned()
                .unwrap_or_default();
            node.storage_allocated = node_storage_allocated_total
                .get(&node.name)
                .cloned()
                .unwrap_or_default();
            for s in &mut node.storage_pools {
                s.allocated = storage_allocated
                    .get(&(node.name.clone(), s.name.clone()))
                    .cloned()
                    .unwrap_or_default();
            }
        }
    }
}

impl State {
    crate fn new() -> Self {
        Default::default()
    }
}
