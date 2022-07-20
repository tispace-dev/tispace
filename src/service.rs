use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use prometheus::{Encoder, GaugeVec, Opts, Registry, TextEncoder};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use regex::Regex;
use std::str::FromStr;
use tracing::warn;

use crate::model::{Image, InstanceStatus, Runtime};
use crate::storage::Storage;
use crate::{
    auth::UserClaims,
    dto::{
        CreateInstanceRequest, Instance as InstanceDto, ListInstancesResponse,
        UpdateInstanceRequest,
    },
};
use crate::{
    error::InstanceError,
    model::{Instance, InstanceStage},
};

static INSTANCE_NAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-z]([-a-z0-9]{0,61}[a-z0-9])?$").unwrap());

/// Returns true if and only if the name is a valid instance name.
///
/// Instance name will be used as kubernetes's resource names, such as pod names, label names,
/// hostnames and so on. So the same naming constraints should be applied to the instance name.
/// See: https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#names.
fn verify_instance_name(name: &str) -> bool {
    INSTANCE_NAME_REGEX.is_match(name)
}

pub fn protected_routes() -> Router {
    async fn create_instance(
        user: UserClaims,
        Json(req): Json<CreateInstanceRequest>,
        Extension(storage): Extension<Storage>,
    ) -> Result<impl IntoResponse, InstanceError> {
        if !verify_instance_name(req.name.as_str()) {
            return Err(InstanceError::InvalidArgs("name".to_string()));
        }
        if req.cpu == 0 {
            return Err(InstanceError::InvalidArgs("cpu".to_string()));
        }
        if req.memory == 0 {
            return Err(InstanceError::InvalidArgs("memory".to_string()));
        }
        if req.disk_size == 0 {
            return Err(InstanceError::InvalidArgs("disk_size".to_string()));
        }
        if req.image.is_empty() {
            return Err(InstanceError::InvalidArgs("image".to_string()));
        }
        if req.runtime.is_empty() {
            return Err(InstanceError::InvalidArgs("runtime".to_string()));
        }
        let image: Image = req
            .image
            .parse()
            .map_err(|_| InstanceError::InvalidArgs("image".to_string()))?;
        let runtime: Runtime = req
            .runtime
            .parse()
            .map_err(|_| InstanceError::InvalidArgs("runtime".to_owned()))?;
        if !runtime.supported_images().contains(&image) {
            return Err(InstanceError::ImageUnavailable {
                image: image.to_string(),
                runtime: runtime.to_string(),
            });
        }
        if !req.storage_pool.is_empty() && (runtime == Runtime::Kata || runtime == Runtime::Runc) {
            return Err(InstanceError::StoragePoolCannotBeSpecified {
                runtime: runtime.to_string(),
            });
        }

        let mut user_err = None;
        match storage
            .read_write(|state| {
                let mut node_exists = false;
                let mut storage_pool_exists = false;
                if !state.nodes.iter().any(|n| {
                    if !req.node_name.is_empty() && req.node_name != n.name {
                        return false;
                    }
                    node_exists = true;

                    if !req.storage_pool.is_empty()
                        && !n.storage_pools.iter().any(|p| p.name == req.storage_pool)
                    {
                        return false;
                    }
                    storage_pool_exists = true;

                    if req.cpu + n.cpu_allocated > n.cpu_total {
                        return false;
                    }
                    if req.memory + n.memory_allocated > n.memory_total {
                        return false;
                    }
                    if req.disk_size + n.storage_allocated.max(n.storage_used) > n.storage_total {
                        return false;
                    }

                    n.storage_pools.iter().any(|p| {
                        if !req.storage_pool.is_empty() && req.storage_pool != p.name {
                            return false;
                        }
                        if req.disk_size + p.allocated.max(p.used) > p.total {
                            return false;
                        }
                        true
                    })
                }) {
                    if !req.node_name.is_empty() && !node_exists {
                        user_err = Some(InstanceError::UnknownNode(req.node_name.clone()));
                    } else if !req.storage_pool.is_empty() && !storage_pool_exists {
                        user_err =
                            Some(InstanceError::UnknownStoragePool(req.storage_pool.clone()));
                    } else {
                        user_err = Some(InstanceError::ResourceExhausted);
                    }
                    return false;
                }

                match state.find_mut_user(&user.username) {
                    Some(u) => {
                        if u.instances.len() + 1 > u.instance_quota {
                            user_err = Some(InstanceError::QuotaExceeded {
                                resource: "Instance".to_string(),
                                quota: u.instance_quota,
                                remaining: u.instance_quota - u.instances.len(),
                                requested: 1,
                                unit: "".to_string(),
                            });
                            return false;
                        }
                        let mut total_cpu = 0;
                        let mut total_memory = 0;
                        let mut total_disk_size = 0;
                        for instance in &u.instances {
                            if instance.name == req.name {
                                user_err = Some(InstanceError::AlreadyExists);
                                return false;
                            }
                            total_cpu += instance.cpu;
                            total_memory += instance.memory;
                            total_disk_size += instance.disk_size;
                        }
                        if total_cpu + req.cpu > u.cpu_quota {
                            user_err = Some(InstanceError::QuotaExceeded {
                                resource: "CPU".to_string(),
                                quota: u.cpu_quota,
                                remaining: u.cpu_quota - total_cpu,
                                requested: req.cpu,
                                unit: "C".to_string(),
                            });
                            return false;
                        }
                        if total_memory + req.memory > u.memory_quota {
                            user_err = Some(InstanceError::QuotaExceeded {
                                resource: "Memory".to_string(),
                                quota: u.memory_quota,
                                remaining: u.memory_quota - total_memory,
                                requested: req.memory,
                                unit: "GiB".to_string(),
                            });
                            return false;
                        }
                        if total_disk_size + req.disk_size > u.disk_quota {
                            user_err = Some(InstanceError::QuotaExceeded {
                                resource: "Disk size".to_string(),
                                quota: u.disk_quota,
                                remaining: u.disk_quota - total_disk_size,
                                requested: req.disk_size,
                                unit: "GiB".to_string(),
                            });
                            return false;
                        }

                        u.instances.push(Instance {
                            name: req.name.clone(),
                            image: image.clone(),
                            cpu: req.cpu,
                            memory: req.memory,
                            disk_size: req.disk_size,
                            stage: InstanceStage::Running,
                            hostname: req.name.clone(),
                            ssh_host: None,
                            ssh_port: None,
                            password: thread_rng()
                                .sample_iter(&Alphanumeric)
                                .take(16)
                                .map(char::from)
                                .collect(),
                            status: InstanceStatus::Creating,
                            internal_ip: None,
                            external_ip: None,
                            runtime: runtime.clone(),
                            node_name: if req.node_name.is_empty() {
                                None
                            } else {
                                Some(req.node_name.clone())
                            },
                            storage_pool: if req.storage_pool.is_empty() {
                                None
                            } else {
                                Some(req.storage_pool.clone())
                            },
                        });
                        true
                    }
                    None => false,
                }
            })
            .await
        {
            Ok(_) => (),
            Err(e) => {
                warn!(
                    username = user.username.as_str(),
                    instance = req.name.as_str(),
                    error = e.to_string().as_str(),
                    "create instance encountered error"
                );
                return Err(InstanceError::CreateFailed);
            }
        }

        match user_err {
            Some(e) => Err(e),
            None => Ok(StatusCode::CREATED),
        }
    }

    async fn delete_instance(
        user: UserClaims,
        Path(instance_name): Path<String>,
        Extension(storage): Extension<Storage>,
    ) -> Result<impl IntoResponse, InstanceError> {
        match storage
            .read_write(|state| {
                match state
                    .find_mut_user(&user.username)
                    .and_then(|u| u.find_mut_instance(&instance_name))
                {
                    Some(instance) if instance.stage != InstanceStage::Deleted => {
                        instance.stage = InstanceStage::Deleted;
                        match instance.runtime {
                            Runtime::Kata | Runtime::Runc => {
                                instance.status = InstanceStatus::Deleting;
                            }
                            Runtime::Lxc | Runtime::Kvm => {
                                instance.status = InstanceStatus::Stopping;
                            }
                        }

                        true
                    }
                    _ => false,
                }
            })
            .await
        {
            Ok(_) => (),
            Err(e) => {
                warn!(
                    username = user.username.as_str(),
                    instance = instance_name.as_str(),
                    error = e.to_string().as_str(),
                    "delete instance encountered error"
                );
                return Err(InstanceError::DeleteFailed);
            }
        }
        Ok(StatusCode::NO_CONTENT)
    }

    async fn update_instance(
        user: UserClaims,
        Path(instance_name): Path<String>,
        Json(req): Json<UpdateInstanceRequest>,
        Extension(storage): Extension<Storage>,
    ) -> Result<impl IntoResponse, InstanceError> {
        if let Some(0) = req.cpu {
            return Err(InstanceError::InvalidArgs("cpu".to_string()));
        }
        if let Some(0) = req.memory {
            return Err(InstanceError::InvalidArgs("memory".to_string()));
        }
        if let Some(runtime) = &req.runtime {
            let _ = Runtime::from_str(runtime)
                .map_err(|_| InstanceError::InvalidArgs(runtime.to_owned()))?;
        }
        let mut user_err = None;
        match storage
            .read_write(|state| match state.find_mut_user(&user.username) {
                Some(u) => {
                    let mut total_cpu = 0;
                    let mut total_memory = 0;
                    for instance in &u.instances {
                        if instance.name != instance_name {
                            total_cpu += instance.cpu;
                            total_memory += instance.memory;
                        }
                    }
                    match u
                        .instances
                        .iter_mut()
                        .find(|instance| instance.name == instance_name)
                    {
                        Some(instance) => {
                            if instance.stage == InstanceStage::Deleted {
                                user_err = Some(InstanceError::AlreadyDeleted);
                                return false;
                            }
                            if instance.status != InstanceStatus::Stopped {
                                user_err = Some(InstanceError::NotYetStopped);
                                return false;
                            }
                            if let Some(cpu) = req.cpu {
                                if total_cpu + cpu > u.cpu_quota {
                                    user_err = Some(InstanceError::QuotaExceeded {
                                        resource: "CPU".to_string(),
                                        quota: u.cpu_quota,
                                        remaining: u.cpu_quota - total_cpu,
                                        requested: cpu,
                                        unit: "C".to_string(),
                                    });
                                    return false;
                                }
                                instance.cpu = cpu;
                            }
                            if let Some(memory) = req.memory {
                                if total_memory + memory > u.memory_quota {
                                    user_err = Some(InstanceError::QuotaExceeded {
                                        resource: "Memory".to_string(),
                                        quota: u.memory_quota,
                                        remaining: u.memory_quota - total_memory,
                                        requested: memory,
                                        unit: "GiB".to_string(),
                                    });
                                    return false;
                                }
                                instance.memory = memory;
                            }
                            if let Some(runtime) = &req.runtime {
                                let runtime = Runtime::from_str(runtime).unwrap();
                                if instance.runtime.compatiable_with(&runtime) {
                                    instance.runtime = runtime;
                                } else {
                                    user_err = Some(InstanceError::RuntimeIncompatible {
                                        current: instance.runtime.to_string(),
                                        target: runtime.to_string(),
                                    });
                                    return false;
                                }
                            }
                            true
                        }
                        None => false,
                    }
                }
                None => false,
            })
            .await
        {
            Ok(_) => (),
            Err(e) => {
                warn!(
                    username = user.username.as_str(),
                    instance = instance_name.as_str(),
                    error = e.to_string().as_str(),
                    "update instance encountered error"
                );
                return Err(InstanceError::UpdateFailed);
            }
        }

        match user_err {
            Some(e) => Err(e),
            None => Ok(StatusCode::NO_CONTENT),
        }
    }

    async fn start_instance(
        user: UserClaims,
        Path(instance_name): Path<String>,
        Extension(storage): Extension<Storage>,
    ) -> Result<impl IntoResponse, InstanceError> {
        let mut user_err = None;
        match storage
            .read_write(|state| {
                match state
                    .find_mut_user(&user.username)
                    .and_then(|u| u.find_mut_instance(&instance_name))
                {
                    Some(instance) => {
                        if instance.stage == InstanceStage::Deleted {
                            user_err = Some(InstanceError::AlreadyDeleted);
                            return false;
                        }
                        if instance.stage != InstanceStage::Running {
                            instance.stage = InstanceStage::Running;
                            instance.status = InstanceStatus::Starting;
                            true
                        } else {
                            false
                        }
                    }
                    None => false,
                }
            })
            .await
        {
            Ok(_) => (),
            Err(_) => return Err(InstanceError::StartFailed),
        }
        match user_err {
            Some(e) => Err(e),
            None => Ok(StatusCode::NO_CONTENT),
        }
    }

    async fn stop_instance(
        user: UserClaims,
        Path(instance_name): Path<String>,
        Extension(storage): Extension<Storage>,
    ) -> Result<impl IntoResponse, InstanceError> {
        let mut user_err = None;
        match storage
            .read_write(|state| {
                match state
                    .find_mut_user(&user.username)
                    .and_then(|u| u.find_mut_instance(&instance_name))
                {
                    Some(instance) => {
                        if instance.stage == InstanceStage::Deleted {
                            user_err = Some(InstanceError::AlreadyDeleted);
                            return false;
                        }
                        if instance.stage != InstanceStage::Stopped {
                            instance.stage = InstanceStage::Stopped;
                            instance.status = InstanceStatus::Stopping;
                            true
                        } else {
                            false
                        }
                    }
                    None => false,
                }
            })
            .await
        {
            Ok(_) => (),
            Err(_) => return Err(InstanceError::StopFailed),
        }
        match user_err {
            Some(e) => Err(e),
            None => Ok(StatusCode::NO_CONTENT),
        }
    }

    async fn list_instances(
        user: UserClaims,
        Extension(storage): Extension<Storage>,
    ) -> impl IntoResponse {
        let mut instances = Vec::new();
        storage
            .read_only(|state| {
                if let Some(u) = state.find_user(&user.username) {
                    instances = u.instances.iter().map(InstanceDto::from).collect();
                }
            })
            .await;
        let resp = ListInstancesResponse { instances };
        Json(resp)
    }

    Router::new()
        .route("/instances", get(list_instances).post(create_instance))
        .route(
            "/instances/:instance_name",
            delete(delete_instance).patch(update_instance),
        )
        .route("/instances/:instance_name/start", post(start_instance))
        .route("/instances/:instance_name/stop", post(stop_instance))
}

pub fn metrics_routes() -> Router {
    async fn metrics(Extension(storage): Extension<Storage>) -> impl IntoResponse {
        let cpu_allocated = GaugeVec::new(
            Opts::new("cpu_allocated", "Total cpu allocated").namespace("tispace"),
            &["node_name"],
        )
        .unwrap();
        let memory_allocated = GaugeVec::new(
            Opts::new("memory_allocated", "Total memory allocated").namespace("tispace"),
            &["node_name"],
        )
        .unwrap();
        let storage_total = GaugeVec::new(
            Opts::new("storage_total", "Total storage").namespace("tispace"),
            &["node_name", "storage_pool"],
        )
        .unwrap();
        let storage_allocated = GaugeVec::new(
            Opts::new("storage_allocated", "Total storage allocated").namespace("tispace"),
            &["node_name", "storage_pool"],
        )
        .unwrap();
        let storage_used = GaugeVec::new(
            Opts::new("storage_used", "Total storage used").namespace("tispace"),
            &["node_name", "storage_pool"],
        )
        .unwrap();
        let instance_status = GaugeVec::new(
            Opts::new("instance_status", "Instance status").namespace("tispace"),
            &["node_name", "storage_pool", "runtime", "status"],
        )
        .unwrap();

        let snapshot = storage.snapshot().await;
        for node in &snapshot.nodes {
            cpu_allocated
                .with_label_values(&[node.name.as_str()])
                .add(node.cpu_allocated as f64);
            memory_allocated
                .with_label_values(&[node.name.as_str()])
                .add(node.memory_allocated as f64);
            for pool in &node.storage_pools {
                storage_total
                    .with_label_values(&[node.name.as_str(), pool.name.as_str()])
                    .add(pool.total as f64);
                storage_allocated
                    .with_label_values(&[node.name.as_str(), pool.name.as_str()])
                    .add(pool.allocated as f64);
                storage_used
                    .with_label_values(&[node.name.as_str(), pool.name.as_str()])
                    .add(pool.used as f64);
            }
        }

        for instance in snapshot.users.iter().flat_map(|u| u.instances.iter()) {
            let mut status = instance.status.to_string();
            if status.starts_with("Error:") {
                status = "Error".to_owned();
            }

            let node_name = instance.node_name.clone().unwrap_or_default();
            let storage_pool = instance.storage_pool.clone().unwrap_or_default();

            instance_status
                .with_label_values(&[
                    node_name.as_str(),
                    storage_pool.as_str(),
                    instance.runtime.to_string().as_str(),
                    status.as_str(),
                ])
                .inc();
        }

        let r = Registry::new();
        r.register(Box::new(cpu_allocated)).unwrap();
        r.register(Box::new(memory_allocated)).unwrap();
        r.register(Box::new(storage_total)).unwrap();
        r.register(Box::new(storage_used)).unwrap();
        r.register(Box::new(storage_allocated)).unwrap();
        r.register(Box::new(instance_status)).unwrap();

        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = r.gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }

    Router::new().route("/metrics", get(metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_instance_name() {
        assert!(verify_instance_name("dev01"));
        assert!(verify_instance_name("dev-01"));
        assert!(!verify_instance_name(""));
        assert!(!verify_instance_name("a".repeat(64).as_str()));
        assert!(!verify_instance_name("dev.01"));
        assert!(!verify_instance_name("dev@01"));
        assert!(!verify_instance_name("DEV01"));
        assert!(verify_instance_name("dev-new"));
        assert!(!verify_instance_name("01dev"));
    }
}
