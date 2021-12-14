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
    crate status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct ListInstancesResponse {
    crate instances: Vec<Instance>,
}
