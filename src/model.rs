use std::fmt::Formatter;
use std::{fmt, str::FromStr};

use crate::error::InstanceError;
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer, Serialize};

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
    type Err = InstanceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_ascii_lowercase();
        match lower.as_str() {
            "kata" => Ok(Self::Kata),
            "runc" => Ok(Self::Runc),
            "lxc" => Ok(Self::Lxc),
            "kvm" => Ok(Self::Kvm),
            _ => Err(InstanceError::UnsupportedRuntime),
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
            Runtime::Kata => vec![Image::CentOS7, Image::Ubuntu2004],
            Runtime::Runc => vec![Image::CentOS7, Image::Ubuntu2004],
            Runtime::Lxc => vec![Image::CentOS7, Image::Ubuntu2004, Image::Ubuntu2204],
            Runtime::Kvm => vec![Image::CentOS7, Image::Ubuntu2004, Image::Ubuntu2204],
        }
    }

    crate fn compatiable_with(&self, other: &Runtime) -> bool {
        matches!(
            (self, other),
            (Runtime::Kata, Runtime::Kata)
                | (Runtime::Kata, Runtime::Runc)
                | (Runtime::Runc, Runtime::Kata)
                | (Runtime::Runc, Runtime::Runc)
        )
    }
}

#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
crate enum Image {
    CentOS7,
    CentOS8,
    Ubuntu2004,
    Ubuntu2204,
}

impl fmt::Display for Image {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Image::CentOS7 => write!(f, "centos:7"),
            Image::CentOS8 => write!(f, "centos:8"),
            Image::Ubuntu2004 => write!(f, "ubuntu:20.04"),
            Image::Ubuntu2204 => write!(f, "ubuntu:22.04"),
        }
    }
}

impl FromStr for Image {
    type Err = InstanceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        if lower.starts_with("tispace/centos7:") {
            return Ok(Self::CentOS7);
        }
        if lower.starts_with("tispace/centos8:") {
            return Ok(Self::CentOS8);
        }
        if lower.starts_with("tispace/ubuntu2004:") {
            return Ok(Self::Ubuntu2004);
        }
        return match lower.as_str() {
            "tispace/centos7" | "centos7" | "centos:7" => Ok(Self::CentOS7),
            "tispace/centos8" | "centos8" | "centos:8" => Ok(Self::CentOS8),
            "tispace/ubuntu2004" | "ubuntu2004" | "ubuntu:20.04" => Ok(Self::Ubuntu2004),
            "ubuntu:22.04" => Ok(Self::Ubuntu2204),
            _ => Err(InstanceError::UnsupportedImage),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
crate struct Instance {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
    crate image: Image,
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
