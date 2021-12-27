use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
    Json, Router,
};
use once_cell::sync::Lazy;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use regex::Regex;

use crate::model::InstanceStatus;
use crate::storage::Storage;
use crate::{
    auth::UserClaims,
    dto::{CreateInstanceRequest, Instance as InstanceDto, ListInstancesResponse},
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
        let mut already_exists = false;
        let mut quota_exceeded = false;
        let mut created = false;
        match storage
            .read_write(|state| {
                match state.users.iter_mut().find(|u| u.username == user.username) {
                    Some(u) => {
                        if u.instances.len() + 1 > u.instance_quota {
                            quota_exceeded = true;
                            return false;
                        }
                        let mut total_cpu = 0;
                        let mut total_memory = 0;
                        let mut total_disk_size = 0;
                        for instance in &mut u.instances {
                            if instance.name == req.name {
                                already_exists = true;
                                return false;
                            }
                            total_cpu += instance.cpu;
                            total_memory += instance.memory;
                            total_disk_size += instance.disk_size;
                        }
                        quota_exceeded = total_cpu + req.cpu > u.cpu_quota
                            || total_memory + req.memory > u.memory_quota
                            || total_disk_size + req.disk_size > u.disk_quota;
                        if quota_exceeded {
                            return false;
                        }

                        u.instances.push(Instance {
                            name: req.name.clone(),
                            cpu: req.cpu,
                            memory: req.memory,
                            disk_size: req.disk_size,
                            stage: InstanceStage::Running,
                            hostname: format!("{}.{}.tispace", req.name, u.username),
                            ssh_host: None,
                            ssh_port: None,
                            password: thread_rng()
                                .sample_iter(&Alphanumeric)
                                .take(16)
                                .map(char::from)
                                .collect(),
                            status: InstanceStatus::Starting,
                        });
                        created = true;
                        created
                    }
                    None => false,
                }
            })
            .await
        {
            Ok(_) => (),
            Err(_) => return Err(InstanceError::CreateFailed),
        }

        if already_exists {
            Err(InstanceError::AlreadyExists)
        } else if quota_exceeded {
            Err(InstanceError::QuotaExceeded)
        } else if created {
            Ok(StatusCode::CREATED)
        } else {
            Err(InstanceError::CreateFailed)
        }
    }

    async fn delete_instance(
        user: UserClaims,
        Path(instance_name): Path<String>,
        Extension(storage): Extension<Storage>,
    ) -> Result<impl IntoResponse, InstanceError> {
        match storage
            .read_write(|state| {
                match state.users.iter_mut().find(|u| u.username == user.username) {
                    Some(u) => {
                        match u.instances.iter_mut().find(|instance| {
                            instance.name == instance_name
                                && instance.stage != InstanceStage::Deleted
                        }) {
                            Some(instance) => {
                                instance.stage = InstanceStage::Deleted;
                                instance.status = InstanceStatus::Deleting;
                                true
                            }
                            None => false,
                        }
                    }
                    None => false,
                }
            })
            .await
        {
            Ok(_) => (),
            Err(_) => return Err(InstanceError::DeleteFailed),
        }
        Ok(StatusCode::NO_CONTENT)
    }

    async fn list_instances(
        user: UserClaims,
        Extension(storage): Extension<Storage>,
    ) -> impl IntoResponse {
        let mut instances = Vec::new();
        storage
            .read_only(|state| {
                if let Some(u) = state.users.iter().find(|&u| u.username == user.username) {
                    instances = u.instances.iter().map(InstanceDto::from).collect();
                }
            })
            .await;
        let resp = ListInstancesResponse { instances };
        Json(resp)
    }

    Router::new()
        .route("/instances", get(list_instances).post(create_instance))
        .route("/instances/:instance_name", delete(delete_instance))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_instance_name() {
        assert!(verify_instance_name("dev01"));
        assert!(verify_instance_name("dev-01"));
        assert!(!verify_instance_name(""));
        assert!(!verify_instance_name(
            std::iter::repeat('a').take(64).collect::<String>().as_str()
        ));
        assert!(!verify_instance_name("dev.01"));
        assert!(!verify_instance_name("dev@01"));
        assert!(!verify_instance_name("DEV01"));
        assert!(verify_instance_name("dev-new"));
        assert!(!verify_instance_name("01dev"));
    }
}
