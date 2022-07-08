use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct CreateInstanceRequest {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
    crate image: String,
    crate runtime: String,
    #[serde(default)]
    crate node_name: String,
    #[serde(default)]
    crate storage_pool: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct UpdateInstanceRequest {
    crate cpu: Option<usize>,
    crate memory: Option<usize>,
    crate runtime: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct Instance {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
    crate hostname: String,
    // Deprecated: use external_ip instead.
    crate ssh_host: Option<String>,
    // Deprecated: use 22 instead.
    crate ssh_port: Option<i32>,
    crate password: String,
    crate status: String,
    crate image: String,
    crate internal_ip: Option<String>,
    crate external_ip: Option<String>,
    crate runtime: String,
    crate node_name: Option<String>,
    crate storage_pool: Option<String>,
}

impl From<&crate::model::Instance> for Instance {
    fn from(m: &crate::model::Instance) -> Self {
        Instance {
            name: m.name.clone(),
            cpu: m.cpu,
            memory: m.memory,
            disk_size: m.disk_size,
            hostname: m.name.clone(),
            ssh_host: m.ssh_host.clone(),
            ssh_port: m.ssh_port,
            password: m.password.clone(),
            status: m.status.to_string(),
            image: m.image.to_string(),
            internal_ip: m.internal_ip.clone(),
            external_ip: m.external_ip.clone(),
            runtime: m.runtime.to_string(),
            node_name: m.node_name.clone(),
            storage_pool: m.storage_pool.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct ListInstancesResponse {
    crate instances: Vec<Instance>,
}
