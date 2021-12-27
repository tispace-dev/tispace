use anyhow::{anyhow, Result};
use either::Either;
use k8s_openapi::api::core::v1::{
    Container, EnvVar, PersistentVolumeClaim, PersistentVolumeClaimSpec,
    PersistentVolumeClaimVolumeSource, Pod, PodDNSConfig, PodSpec, ResourceRequirements,
    SecurityContext, Service, ServicePort, ServiceSpec, Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
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
                ("memory".to_owned(), Quantity(format!("{}Gi", memory_limit))),
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
            value: Some(password.to_owned()),
            ..Default::default()
        }]),
        ..Default::default()
    }
}

fn rootfs_name(pod_name: &str) -> String {
    format!("{}-rootfs", pod_name)
}

fn build_rootfs_pvc(pod_name: &str, disk_size: usize) -> PersistentVolumeClaim {
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
                    Quantity(format!("{}Gi", disk_size)),
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

fn build_subdomain_service(subdomain: &str) -> Service {
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

fn build_pod_service(pod_name: &str) -> Service {
    Service {
        metadata: ObjectMeta {
            name: Some(pod_name.to_owned()),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            selector: Some(BTreeMap::from([(
                "tispace/instance".to_owned(),
                pod_name.to_owned(),
            )])),
            ports: Some(vec![ServicePort {
                name: Some("ssh".to_owned()),
                port: 22,
                target_port: Some(IntOrString::Int(22)),
                ..Default::default()
            }]),
            type_: Some("NodePort".to_owned()),
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
            labels: Some(BTreeMap::from([
                ("tispace/subdomain".to_owned(), subdomain.to_owned()),
                ("tispace/instance".to_owned(), pod_name.to_owned()),
            ])),
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

fn get_ssh_port(svc: &Service) -> Option<i32> {
    svc.spec
        .as_ref()
        .and_then(|spec| spec.ports.as_ref())
        .and_then(|ports| {
            ports
                .iter()
                .find(|port| matches!(port.name.as_deref(), Some("ssh")))
                .and_then(|port| port.node_port)
        })
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
                    match instance.stage {
                        InstanceStage::Stopped => {
                            if instance.status != InstanceStatus::Stopped {
                                info!(
                                    username = user.username.as_str(),
                                    instance = instance.name.as_str(),
                                    "stopping instance"
                                );
                                if let Err(e) = self.stop_instance(user, instance).await {
                                    warn!(
                                        username = user.username.as_str(),
                                        instance = instance.name.as_str(),
                                        error = e.to_string().as_str(),
                                        "stopping instance encountered error"
                                    );
                                }
                            }
                        }
                        InstanceStage::Running => {
                            if instance.status != InstanceStatus::Running {
                                info!(
                                    username = user.username.as_str(),
                                    instance = instance.name.as_str(),
                                    "starting instance"
                                );
                                if let Err(e) = self.start_instance(user, instance).await {
                                    warn!(
                                        username = user.username.as_str(),
                                        instance = instance.name.as_str(),
                                        error = e.to_string().as_str(),
                                        "starting instance encountered error"
                                    );
                                }
                            }
                        }
                        InstanceStage::Deleted => {
                            info!(
                                username = user.username.as_str(),
                                instance = instance.name.as_str(),
                                "deleting instance"
                            );
                            if let Err(e) = self.delete_instance(user, instance).await {
                                warn!(
                                    username = user.username.as_str(),
                                    instance = instance.name.as_str(),
                                    error = e.to_string().as_str(),
                                    "deleting instance encountered error"
                                );
                            }
                        }
                    }
                    if let Err(e) = self.update_instance_status(user, instance).await {
                        warn!(
                            username = user.username.as_str(),
                            instance = instance.name.as_str(),
                            error = e.to_string().as_str(),
                            "updating instance status encountered error"
                        );
                    }
                }
                // If a user has no instance, delete the Service.
                if user.instances.is_empty() {
                    let subdomain = user.username.as_str();
                    if let Err(e) = self.delete_service(subdomain).await {
                        warn!(
                            username = user.username.as_str(),
                            error = e.to_string().as_str(),
                            "deleting service encountered error"
                        );
                    }
                }
            }
            sleep(Duration::from_secs(3)).await;
        }
    }

    async fn delete_pod(&self, pod_name: &str) -> Result<()> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), NAMESPACE);
        match pods.delete(pod_name, &DeleteParams::default()).await {
            Ok(Either::Left(_)) => {
                info!("deleting pod {}", pod_name);
                Ok(())
            }
            Ok(Either::Right(_)) => {
                info!("deleted pod {}", pod_name);
                Ok(())
            }
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => Ok(()),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn delete_service(&self, svc_name: &str) -> Result<()> {
        let services: Api<Service> = Api::namespaced(self.client.clone(), NAMESPACE);
        match services.delete(svc_name, &DeleteParams::default()).await {
            Ok(Either::Left(_)) => {
                info!("deleting service {}", svc_name);
                Ok(())
            }
            Ok(Either::Right(_)) => {
                info!("deleted service {}", svc_name);
                Ok(())
            }
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => Ok(()),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn delete_pvc(&self, pvc_name: &str) -> Result<()> {
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), NAMESPACE);
        match pvcs.delete(pvc_name, &DeleteParams::default()).await {
            Ok(Either::Left(_)) => {
                info!("deleting serpersistentvolumeclaimvice {}", pvc_name);
                Ok(())
            }
            Ok(Either::Right(_)) => {
                info!("deleted persistentvolumeclaim {}", pvc_name);
                Ok(())
            }
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => Ok(()),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn stop_instance(&self, user: &User, instance: &Instance) -> Result<()> {
        let pod_name = format!("{}-{}", user.username, instance.name);
        info!("deleting pod {}", pod_name);
        self.delete_pod(&pod_name).await
    }

    async fn start_instance(&self, user: &User, instance: &Instance) -> Result<()> {
        let pod_name = format!("{}-{}", user.username, instance.name);

        // 1. Ensure sudomain service is created.
        let hostname = instance.name.clone();
        let subdomain = user.username.clone();
        let services: Api<Service> = Api::namespaced(self.client.clone(), NAMESPACE);
        match services.get(&subdomain).await {
            Ok(_) => {}
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {
                info!("creating service {}", subdomain);
                let service = build_subdomain_service(&subdomain);
                services.create(&PostParams::default(), &service).await?;
            }
            Err(e) => {
                return Err(anyhow!(e));
            }
        }

        // 2. Ensure pod service is created.
        match services.get(&pod_name).await {
            Ok(_) => {}
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {
                info!("creating service {}", pod_name);
                let service = build_pod_service(&pod_name);
                services.create(&PostParams::default(), &service).await?;
            }
            Err(e) => {
                return Err(anyhow!(e));
            }
        }

        // 3. Ensure PersistentVolumeClaim is created.
        let pvc_name = rootfs_name(&pod_name);
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), NAMESPACE);
        match pvcs.get(&pvc_name).await {
            Ok(_) => {}
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {
                info!("creating persistentvolumeclaim {}", pvc_name);
                let pvc = build_rootfs_pvc(&pod_name, instance.disk_size);
                pvcs.create(&PostParams::default(), &pvc).await?;
            }
            Err(e) => {
                return Err(anyhow!(e));
            }
        }

        // 4. Ensure Pod is created.
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), NAMESPACE);
        match pods.get(&pod_name).await {
            Ok(_) => {}
            Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {
                info!("creating pod {}", pod_name);
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
        Ok(())
    }

    async fn delete_instance(&self, user: &User, instance: &Instance) -> Result<()> {
        let pod_name = format!("{}-{}", user.username, instance.name);
        let pvc_name = rootfs_name(&pod_name);
        self.delete_pod(&pod_name).await?;
        self.delete_pvc(&pvc_name).await?;
        self.delete_service(&pod_name).await?;
        Ok(())
    }

    async fn update_instance_status(&self, user: &User, instance: &Instance) -> Result<()> {
        let pod_name = format!("{}-{}", user.username, instance.name);
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), NAMESPACE);
        let pvc_name = rootfs_name(&pod_name);
        let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), NAMESPACE);
        let services: Api<Service> = Api::namespaced(self.client.clone(), NAMESPACE);
        let mut new_status = instance.status.clone();
        let mut ssh_address = instance.ssh_address.clone();
        let mut deleted = false;
        match instance.stage {
            InstanceStage::Stopped => match pods.get(&pod_name).await {
                Ok(_) => {}
                Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {
                    new_status = InstanceStatus::Stopped;
                }
                Err(e) => {
                    return Err(anyhow!(e));
                }
            },
            InstanceStage::Running => {
                match pods.get(&pod_name).await {
                    Ok(pod) => {
                        let pod_status = pod
                            .status
                            .as_ref()
                            .map(|s| s.phase.clone().unwrap_or_default())
                            .unwrap_or_default();
                        if pod_status == "Running" {
                            new_status = InstanceStatus::Running;
                        } else {
                            match instance.status {
                                InstanceStatus::Running | InstanceStatus::Error(_) => {
                                    new_status =
                                        InstanceStatus::Error(format!("Pod is {}", pod_status));
                                    warn!(
                                        username = user.username.as_str(),
                                        instance = instance.name.as_str(),
                                        pod_status = pod_status.as_str(),
                                        "pod status is abnormal"
                                    );
                                }
                                _ => {}
                            }
                        }
                        let host_ip = pod
                            .status
                            .as_ref()
                            .map(|s| s.host_ip.clone().unwrap_or_default())
                            .unwrap_or_default();
                        if !host_ip.is_empty() {
                            match services.get(&pod_name).await {
                                Ok(svc) => {
                                    if let Some(port) = get_ssh_port(&svc) {
                                        ssh_address = format!("{}:{}", host_ip, port);
                                    }
                                }
                                Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {}
                                Err(e) => {
                                    return Err(anyhow!(e));
                                }
                            };
                        }
                    }
                    Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {
                        match instance.status {
                            InstanceStatus::Running | InstanceStatus::Error(_) => {
                                new_status = InstanceStatus::Error("Pod is missing".to_owned());
                                warn!(
                                    username = user.username.as_str(),
                                    instance = instance.name.as_str(),
                                    "pod is missing"
                                );
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        return Err(anyhow!(e));
                    }
                };
            }
            InstanceStage::Deleted => {
                deleted = true;
                match pods.get(&pod_name).await {
                    Ok(_) => {
                        deleted = false;
                    }
                    Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {}
                    Err(e) => {
                        return Err(anyhow!(e));
                    }
                };
                match pvcs.get(&pvc_name).await {
                    Ok(_) => {
                        deleted = false;
                    }
                    Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {}
                    Err(e) => {
                        return Err(anyhow!(e));
                    }
                }
                match services.get(&pod_name).await {
                    Ok(_) => {
                        deleted = false;
                    }
                    Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => {}
                    Err(e) => {
                        return Err(anyhow!(e));
                    }
                }
            }
        }
        // Status is unchanged, skip writing storage.
        if !deleted && new_status == instance.status && ssh_address == instance.ssh_address {
            return Ok(());
        }
        self.storage
            .read_write(|state| {
                if let Some(u) = state.users.iter_mut().find(|u| u.username == user.username) {
                    for i in 0..u.instances.len() {
                        if u.instances[i].name == instance.name
                            && u.instances[i].stage == instance.stage
                        {
                            if deleted {
                                u.instances.remove(i);
                            } else {
                                u.instances[i].ssh_address = ssh_address.clone();
                                u.instances[i].status = new_status.clone();
                            }
                            return true;
                        }
                    }
                }
                false
            })
            .await
            .map_err(|e| anyhow!(e))
    }
}
