use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct UserLoginRequest {
    crate username: String,
    crate password: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct UserLoginResponse {
    crate token: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct ChangePasswordRequest {
    crate new_password: String,
}

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
crate struct CreateInstanceResponse {
    crate domain_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct Instance {
    crate name: String,
    crate cpu: usize,
    crate memory: usize,
    crate disk_size: usize,
    crate domain_name: String,
    crate status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
crate struct ListInstancesResponse {
    crate instances: Vec<Instance>,
}
