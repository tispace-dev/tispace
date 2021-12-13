use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
    Json, Router,
};
use crypto::pbkdf2::pbkdf2_simple;

use crate::model::InstanceStatus;
use crate::storage::Storage;
use crate::{
    auth::UserClaims,
    dto::{ChangePasswordRequest, CreateInstanceRequest, ListInstancesResponse},
};
use crate::{
    error::{InstanceError, UserError},
    model::{Instance, InstanceStage},
};

pub fn protected_routes() -> Router {
    async fn change_password(
        user: UserClaims,
        Json(change): Json<ChangePasswordRequest>,
        Extension(storage): Extension<Storage>,
    ) -> Result<impl IntoResponse, UserError> {
        if change.new_password.is_empty() {
            return Err(UserError::EmptyNewPassword);
        }

        match storage
            .read_write(
                |state| match state.users.iter_mut().find(|u| u.username == user.sub) {
                    Some(u) => {
                        u.password_hash = pbkdf2_simple(&change.new_password, 1024).unwrap();
                        true
                    }
                    None => false,
                },
            )
            .await
        {
            Ok(_) => (),
            Err(_) => return Err(UserError::PasswordUpdateFailed),
        }
        Ok(StatusCode::NO_CONTENT)
    }

    async fn create_instance(
        user: UserClaims,
        Json(req): Json<CreateInstanceRequest>,
        Extension(storage): Extension<Storage>,
    ) -> Result<impl IntoResponse, InstanceError> {
        if req.name.is_empty() {
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
            .read_write(
                |state| match state.users.iter_mut().find(|u| u.username == user.sub) {
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
                            stage: InstanceStage::Pending,
                            hostname: format!(
                                "{}.{}.tispace.svc.cluster.local",
                                req.name, u.username
                            ),
                            status: InstanceStatus::Pending,
                        });
                        created = true;
                        created
                    }
                    None => false,
                },
            )
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
            .read_write(
                |state| match state.users.iter_mut().find(|u| u.username == user.sub) {
                    Some(u) => {
                        match u.instances.iter_mut().find(|instance| {
                            instance.name == instance_name
                                && instance.stage != InstanceStage::Deleting
                        }) {
                            Some(instance) => {
                                instance.stage = InstanceStage::Deleting;
                                true
                            }
                            None => false,
                        }
                    }
                    None => false,
                },
            )
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
                if let Some(u) = state.users.iter().find(|&u| u.username == user.sub) {
                    instances = u.instances.clone();
                }
            })
            .await;
        let resp = ListInstancesResponse { instances };
        Json(resp)
    }

    Router::new()
        .route("/user/password", put(change_password))
        .route("/instances", get(list_instances).post(create_instance))
        .route("/instances/:instance_name", delete(delete_instance))
}
