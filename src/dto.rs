use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct CreateInstanceRequest {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct Instance {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
    crate hostname: String,
    crate password: String,
    crate status: String,
}

impl From<&crate::model::Instance> for Instance {
    fn from(m: &crate::model::Instance) -> Self {
        Instance {
            name: m.name.clone(),
            cpu: m.cpu,
            memory: m.memory,
            disk_size: m.disk_size,
            hostname: m.hostname.clone(),
            password: m.password.clone(),
            status: m.status.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct ListInstancesResponse {
    crate instances: Vec<Instance>,
}
