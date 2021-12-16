use anyhow::{anyhow, Result};
use k8s_openapi::api::core::v1::{
    Container, EnvVar, PersistentVolumeClaim, PersistentVolumeClaimSpec,
    PersistentVolumeClaimVolumeSource, Pod, PodDNSConfig, PodSpec, ResourceRequirements,
    SecurityContext, Service, ServiceSpec, Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{DeleteParams, PostParams};
use kube::error::ErrorResponse;
use kube::{Api, Client};
use std::collections::BTreeMap;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::model::{Instance, InstanceStage, InstanceStatus, User};
use crate::storage::Storage;

const NAMESPACE: &str = "tispace";
const FAKE_IMAGE: &str = "k8s.gcr.io/pause:3.5";
const DEFAULT_BASE_IMAGE: &str = "tispace/ubuntu2004:latest";
const RBD_STORAGE_CLASS_NAME: &str = "rook-ceph-block";
const DEFAULT_RUNTIME_CLASS_NAME: &str = "kata";
const PASSWORD_ENV_KEY: &str = "PASSWORD";

fn build_container(pod_name: &str, cpu_limit: usize, memory_limit: usize) -> Container {
    let memory_limit_in_mb = (memory_limit + 1024 * 1024 - 1) / 1024 / 1024;
    Container {
        name: pod_name.to_owned(),
        command: Some(vec!["/sbin/init".to_owned()]),
        image: Some(FAKE_IMAGE.to_owned()),
        image_pull_policy: Some("IfNotPresent".to_owned()),
        security_context: Some(SecurityContext {
            privileged: Some(true),
            ..Default::default()
        }),
        volume_mounts: Some(vec![VolumeMount {
            name: "rootfs".to_owned(),
            mount_path: "/".to_owned(),
            ..Default::default()
        }]),
        resources: Some(ResourceRequirements {
            limits: Some(BTreeMap::from([
                ("cpu".to_owned(), Quantity(cpu_limit.to_string())),
                (
                    "memory".to_owned(),
                    Quantity(format!("{}Mi", memory_limit_in_mb)),
                ),
            ])),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_init_container(pod_name: &str, password: &str) -> Container {
    Container {
        name: format!("{}-init", pod_name),
        command: Some(vec!["/init-rootfs.sh".to_owned()]),
        image: Some(DEFAULT_BASE_IMAGE.to_owned()),
        image_pull_policy: Some("IfNotPresent".to_owned()),
        volume_mounts: Some(vec![VolumeMount {
            name: "rootfs".to_owned(),
            mount_path: "/tmp/rootfs".to_owned(),
            ..Default::default()
        }]),
        env: Some(vec![EnvVar {
            name: PASSWORD_ENV_KEY.to_owned(),
            value: Some(base64::encode(password)),
            ..Default::default()
        }]),
        ..Default::default()
    }
}

fn rootfs_name(pod_name: &str) -> String {
    format!("{}-rootfs", pod_name)
}

fn build_rootfs_pvc(pod_name: &str, disk_size: usize) -> PersistentVolumeClaim {
    let disk_size_in_gb = (disk_size + 1024 * 1024 * 1024 - 1) / 1024 / 1024 / 1024;
    PersistentVolumeClaim {
        metadata: ObjectMeta {
            name: Some(rootfs_name(pod_name)),
            namespace: Some(NAMESPACE.to_owned()),
            ..Default::default()
        },
        spec: Some(PersistentVolumeClaimSpec {
            access_modes: Some(vec!["ReadWriteOnce".to_owned()]),
            resources: Some(ResourceRequirements {
                requests: Some(BTreeMap::from([(
                    "storage".to_owned(),
                    Quantity(format!("{}Gi", disk_size_in_gb)),
                )])),
                ..Default::default()
            }),
            storage_class_name: Some(RBD_STORAGE_CLASS_NAME.to_owned()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_rootfs_volume(pod_name: &str) -> Volume {
    Volume {
        name: "rootfs".to_owned(),
        persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
            claim_name: rootfs_name(pod_name),
            read_only: Some(false),
        }),
        ..Default::default()
    }
}

fn build_service(subdomain: &str) -> Service {
    Service {
        metadata: ObjectMeta {
            name: Some(subdomain.to_owned()),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(BTreeMap::from([(
                "tispace/subdomain".to_owned(),
                subdomain.to_owned(),
            )])),
            cluster_ip: Some("None".to_owned()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_pod(
    pod_name: &str,
    cpu_limit: usize,
    memory_limit: usize,
    hostname: &str,
    subdomain: &str,
    password: &str,
) -> Pod {
    Pod {
        metadata: ObjectMeta {
            name: Some(pod_name.to_owned()),
            namespace: Some(NAMESPACE.to_owned()),
            labels: Some(BTreeMap::from([(
                "tispace/subdomain".to_owned(),
                subdomain.to_owned(),
            )])),
            ..Default::default()
        },
        spec: Some(PodSpec {
            hostname: Some(hostname.to_owned()),
            subdomain: Some(subdomain.to_owned()),
            automount_service_account_token: Some(false),
            containers: vec![build_container(pod_name, cpu_limit, memory_limit)],
            init_containers: Some(vec![build_init_container(pod_name, password)]),
            volumes: Some(vec![build_rootfs_volume(pod_name)]),
            restart_policy: Some("Always".to_owned()),
            dns_config: Some(PodDNSConfig {
                searches: Some(vec![format!("{}.tispace.svc.cluster.local", subdomain)]),
                ..Default::default()
            }),
            runtime_class_name: Some(DEFAULT_RUNTIME_CLASS_NAME.to_owned()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub struct Operator {
    client: Client,
    storage: Storage,
}

impl Operator {
    pub fn new(client: Client, storage: Storage) -> Self {
        Operator { client, storage }
    }

    pub async fn run(&self) {
        loop {
            let state = self.storage.snapshot().await;
            for user in &state.users {
                for instance in &user.instances {
                    if let Err(e) = match instance.stage {
                        InstanceStage::Pending | InstanceStage::Running => {
                            self.ensure_instance_is_running(user, instance).await
                        }
                        InstanceStage::Deleting => {
                            self.ensure_instance_is_deleted(user, instance).await
                        }
                    } {
                        warn!(
                            username = user.username.as_str(),
                            instance = instance.name.as_str(),
                            error = e.to_string().as_str(),
                            "Failed to sync instance status"
                        );
                    }
                }
                // If a user has no instance, delete the Service.
                if user.instances.is_empty() {
                    let subdomain = user.username.as_str();
                    if let Err(e) = self.remove_orphan_service(subdomain).await {
                        warn!(
                            subdomain = subdomain,
                            error = e.to_string().as_str(),
                            "Failed to remove orphan service"
                        );
                    }
                }
            }
            sleep(Duration::from_secs(3)).await;
        }
    }

    crate async fn ensure_instance_is_running(
        &self,
        user: &User,
        instance: &Instance,
    ) -> Result<()> {
        // 1. Ensure Service is created.
        let hostname = instance.name.clone();
        let subdomain = user.username.clone();
        let services: Api<Service> = Api::namespaced(self.client.clone(), NAMESPACE);
        match services.get(&subdomain).await {
            Ok(_) => {}
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {
                info!(
                    username = user.username.as_str(),
                    instance = instance.name.as_str(),
                    subdomain = subdomain.as_str(),
                    "Creating Service"
                );
                let service = build_service(&subdomain);
                services.create(&PostParams::default(), &service).await?;
            }
            Err(e) => {
                return Err(anyhow!(e));
            }
        }

        // 2. Ensure PersistentVolumeClaim is created.
        let pod_name = format!("{}-{}", user.username, instance.name);
        let pvc_name = rootfs_name(&pod_name);
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), NAMESPACE);
        match pvcs.get(&pvc_name).await {
            Ok(_) => {}
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {
                info!(
                    username = user.username.as_str(),
                    instance = instance.name.as_str(),
                    "Creating PersistentVolumeClaim"
                );
                let pvc = build_rootfs_pvc(&pod_name, instance.disk_size);
                pvcs.create(&PostParams::default(), &pvc).await?;
            }
            Err(e) => {
                return Err(anyhow!(e));
            }
        }

        // 3. Ensure Pod is running.
        let mut new_stage = instance.stage.clone();
        let mut new_status = instance.status.clone();
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), NAMESPACE);
        match pods.get(&pod_name).await {
            Ok(pod) => {
                let pod_status = pod
                    .status
                    .map(|s| s.phase.unwrap_or_default())
                    .unwrap_or_default();
                if pod_status == "Running" {
                    new_stage = InstanceStage::Running;
                    new_status = InstanceStatus::Running;
                } else if instance.stage == InstanceStage::Running {
                    new_status = InstanceStatus::Error(format!("Pod is {}", pod_status));
                    warn!(
                        username = user.username.as_str(),
                        instance = instance.name.as_str(),
                        pod_status = pod_status.as_str(),
                        "Pod status is abnormal"
                    );
                }
            }
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {
                if instance.stage == InstanceStage::Running {
                    new_status = InstanceStatus::Error("Pod is missing".to_owned());
                    warn!(
                        username = user.username.as_str(),
                        instance = instance.name.as_str(),
                        "Pod is missing"
                    );
                }
                info!(
                    username = user.username.as_str(),
                    instance = instance.name.as_str(),
                    "Creating Pod"
                );
                let pod = build_pod(
                    &pod_name,
                    instance.cpu,
                    instance.memory,
                    &hostname,
                    &subdomain,
                    &instance.password,
                );
                pods.create(&PostParams::default(), &pod).await?;
            }
            Err(e) => {
                return Err(anyhow!(e));
            }
        }

        // 4. Update instance status.
        if new_stage != instance.stage || new_status != instance.status {
            self.storage
                .read_write(|state| {
                    if let Some(u) = state.users.iter_mut().find(|u| u.username == user.username) {
                        for i in 0..u.instances.len() {
                            if u.instances[i].name == instance.name {
                                u.instances[i].stage = new_stage.clone();
                                u.instances[i].status = new_status.clone();
                                return true;
                            }
                        }
                    }
                    false
                })
                .await
                .unwrap();
        }
        Ok(())
    }

    crate async fn ensure_instance_is_deleted(
        &self,
        user: &User,
        instance: &Instance,
    ) -> Result<()> {
        let mut deleted = true;

        // 1. Try to delete the Pod.
        let pod_name = format!("{}-{}", user.username, instance.name);
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), NAMESPACE);
        match pods.get(&pod_name).await {
            Ok(_pod) => {
                deleted = false;
                info!(
                    username = user.username.as_str(),
                    instance = instance.name.as_str(),
                    "Deleting Pod"
                );
                pods.delete(&pod_name, &DeleteParams::default()).await?;
            }
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {}
            Err(e) => {
                return Err(anyhow!(e));
            }
        }

        // 2. Try to delete the PersistentVolumeClaim.
        let pvc_name = rootfs_name(&pod_name);
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), NAMESPACE);
        match pvcs.get(&pvc_name).await {
            Ok(_) => {
                deleted = false;
                info!(
                    username = user.username.as_str(),
                    instance = instance.name.as_str(),
                    "Deleting PersistentVolumeClaim"
                );
                pvcs.delete(&pvc_name, &DeleteParams::default()).await?;
            }
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {}
            Err(e) => {
                return Err(anyhow!(e));
            }
        }

        // 3. If both Pod and PersistentVolumeClaim are deleted, remove the instance from storage state.
        if deleted {
            self.storage
                .read_write(|state| {
                    if let Some(u) = state.users.iter_mut().find(|u| u.username == user.username) {
                        for i in 0..u.instances.len() {
                            if u.instances[i].name == instance.name {
                                u.instances.remove(i);
                                return true;
                            }
                        }
                    }
                    false
                })
                .await
                .unwrap();
            info!(
                username = user.username.as_str(),
                instance = instance.name.as_str(),
                "Instance is deleted"
            );
        }

        Ok(())
    }

    async fn remove_orphan_service(&self, subdomain: &str) -> Result<()> {
        let services: Api<Service> = Api::namespaced(self.client.clone(), NAMESPACE);
        match services.get(subdomain).await {
            Ok(_) => {
                info!(subdomain = subdomain, "Deleting Service");
                services.delete(subdomain, &DeleteParams::default()).await?;
            }
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {}
            Err(e) => {
                return Err(anyhow!(e));
            }
        }
        Ok(())
    }
}
