use anyhow::{anyhow, Result};
use reqwest::Client;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::env::{EXTERNAL_IP_PREFIX_LENGTH, LXD_IMAGE_SERVER_URL, LXD_PROJECT, LXD_SERVER_URL};
use crate::model::{Image, Instance, InstanceStage, InstanceStatus, Runtime, User};
use crate::storage::Storage;

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
            self.run_once().await;
            sleep(Duration::from_secs(3)).await;
        }
    }

    async fn run_once(&self) {
        let state = self.storage.snapshot().await;
        for user in &state.users {
            for instance in &user.instances {
                if instance.runtime != Runtime::Lxc && instance.runtime != Runtime::Kvm {
                    continue;
                }
                // Wait for the scheduler to allocate an IP address and schedule node and storage pool for this instance.
                if instance.status == InstanceStatus::Creating
                    && (instance.external_ip.is_none()
                        || instance.node_name.is_none()
                        || instance.storage_pool.is_none())
                {
                    continue;
                }
                self.sync_instance(user, instance).await;
            }
        }
    }

    async fn sync_instance(&self, user: &User, instance: &Instance) {
        match instance.stage {
            InstanceStage::Stopped => {
                if instance.status != InstanceStatus::Stopped
                    && instance.status != InstanceStatus::Missing
                {
                    if let Err(e) = self.stop_instance(user, instance).await {
                        warn!(
                            username = user.username.as_str(),
                            instance = instance.name.as_str(),
                            runtime = instance.runtime.to_string().as_str(),
                            error = e.to_string().as_str(),
                            "stopping instance encountered error"
                        );
                    }
                }
            }
            InstanceStage::Running => {
                if instance.status != InstanceStatus::Running {
                    if instance.status == InstanceStatus::Creating {
                        if let Err(e) = self.create_instance(user, instance).await {
                            warn!(
                                username = user.username.as_str(),
                                instance = instance.name.as_str(),
                                runtime = instance.runtime.to_string().as_str(),
                                error = e.to_string().as_str(),
                                "creating instance encountered error"
                            );
                        }
                    } else if instance.status != InstanceStatus::Missing {
                        if let Err(e) = self.start_instance(user, instance).await {
                            warn!(
                                username = user.username.as_str(),
                                instance = instance.name.as_str(),
                                runtime = instance.runtime.to_string().as_str(),
                                error = e.to_string().as_str(),
                                "starting instance encountered error"
                            );
                        }
                    }
                }
            }
            InstanceStage::Deleted => {
                if instance.status != InstanceStatus::Deleting {
                    if let Err(e) = self.stop_instance(user, instance).await {
                        warn!(
                            username = user.username.as_str(),
                            instance = instance.name.as_str(),
                            runtime = instance.runtime.to_string().as_str(),
                            error = e.to_string().as_str(),
                            "stopping instance encountered error"
                        );
                    }
                } else if let Err(e) = self.delete_instance(user, instance).await {
                    warn!(
                        username = user.username.as_str(),
                        instance = instance.name.as_str(),
                        runtime = instance.runtime.to_string().as_str(),
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
                runtime = instance.runtime.to_string().as_str(),
                error = e.to_string().as_str(),
                "updating instance status encountered error"
            );
        }
    }

    async fn create_instance(&self, user: &User, instance: &Instance) -> Result<()> {
        info!(
            username = user.username.as_str(),
            instance = instance.name.as_str(),
            runtime = instance.runtime.to_string().as_str(),
            "creating instance"
        );
        let name = format!("{}-{}", user.username, instance.name);
        let url = format!(
            "{}/1.0/instances?project={}&target={}",
            LXD_SERVER_URL.as_str(),
            LXD_PROJECT.as_str(),
            instance.node_name.as_ref().unwrap()
        );

        let type_ = get_instance_type(&instance.runtime)?;

        let eip = format!(
            "{}/{}",
            instance.external_ip.as_ref().unwrap(),
            EXTERNAL_IP_PREFIX_LENGTH.to_owned()
        );

        let user_data = format!(
            r#"#cloud-config
hostname: {}
fqdn: {}
ssh_pwauth: true
disable_root: false
chpasswd:
  expire: false
  list:
  - root:{}
"#,
            instance.name, instance.name, instance.password
        );
        let network_config = match instance.image {
            Image::CentOS7 | Image::CentOS8 | Image::CentOS9Stream => {
                format!(
                    r#"network:
  version: 1
  config:
  - type: physical
    name: eth0
    subnets:
    - type: dhcp
  - type: physical
    name: eth1
    subnets:
    - type: static
      address: {}
"#,
                    eip
                )
            }
            Image::Ubuntu2004 | Image::Ubuntu2204 => {
                let mut eth0 = "eth0";
                let mut eth1 = "eth1";
                if instance.runtime == Runtime::Kvm {
                    eth0 = "enp5s0";
                    eth1 = "enp6s0";
                }
                format!(
                    r#"network:
  version: 2
  ethernets:
    eth0:
      match:
        name: {}
      dhcp4: true
      dhcp6: false
    eth1:
      match:
        name: {}
      dhcp4: false
      dhcp6: false
      addresses:
      - {}
"#,
                    eth0, eth1, eip
                )
            }
        };

        let res: serde_json::Value = self
            .client
            .post(url)
            .json(&serde_json::json!({
                "devices": {
                    "root": {
                        "path": "/",
                        "pool": instance.storage_pool.as_ref().unwrap(),
                        "size": format!("{}GiB",instance.disk_size),
                        "type":"disk"
                    }
                },
                "name": name,
                "source": {
                    "type": "image",
                    "alias": get_image_alias(&instance.image)?,
                    "protocol": "simplestreams",
                    "mode": "pull",
                    "server": LXD_IMAGE_SERVER_URL.as_str()
                },
                "config": {
                    "limits.cpu": instance.cpu.to_string(),
                    "limits.memory": format!("{}GiB", instance.memory),
                    "user.user-data": user_data,
                    "user.network-config": network_config
                },
                "type": type_
            }))
            .send()
            .await?
            .json()
            .await?;
        check_error(&res)
    }

    async fn delete_instance(&self, user: &User, instance: &Instance) -> Result<()> {
        info!(
            username = user.username.as_str(),
            instance = instance.name.as_str(),
            runtime = instance.runtime.to_string().as_str(),
            "deleting instance"
        );
        let name = format!("{}-{}", user.username, instance.name);
        let url = format!(
            "{}/1.0/instances/{}?project={}",
            LXD_SERVER_URL.as_str(),
            name,
            LXD_PROJECT.as_str(),
        );

        let res: serde_json::Value = self.client.delete(url).send().await?.json().await?;
        if is_not_found(&res) {
            return Ok(());
        }
        check_error(&res)
    }

    async fn start_instance(&self, user: &User, instance: &Instance) -> Result<()> {
        info!(
            username = user.username.as_str(),
            instance = instance.name.as_str(),
            runtime = instance.runtime.to_string().as_str(),
            "starting instance"
        );

        self.sync_instance_limits(user, instance).await?;

        let name = format!("{}-{}", user.username, instance.name);
        let url = format!(
            "{}/1.0/instances/{}/state?project={}",
            LXD_SERVER_URL.as_str(),
            name,
            LXD_PROJECT.as_str(),
        );

        let res: serde_json::Value = self
            .client
            .put(url)
            .json(&serde_json::json!({
               "action": "start"
            }))
            .send()
            .await?
            .json()
            .await?;
        check_error(&res)
    }

    async fn sync_instance_limits(&self, user: &User, instance: &Instance) -> Result<()> {
        let name = format!("{}-{}", user.username, instance.name);
        let url = format!(
            "{}/1.0/instances/{}?project={}",
            LXD_SERVER_URL.as_str(),
            name,
            LXD_PROJECT.as_str(),
        );
        let res: serde_json::Value = self.client.get(url.clone()).send().await?.json().await?;
        check_error(&res)?;

        if parse_instance_status(&res).unwrap_or_default() != "Stopped" {
            return Ok(());
        }

        let config = res
            .get("metadata")
            .and_then(|m| m.get("config"))
            .ok_or_else(|| anyhow!("cannot find instance config"))?;
        let cpu_limit = config
            .get("limits.cpu")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let memory_limit = config
            .get("limits.memory")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if cpu_limit != instance.cpu.to_string().as_str()
            || memory_limit != format!("{}GiB", instance.memory)
        {
            info!(
                username = user.username.as_str(),
                instance = instance.name.as_str(),
                runtime = instance.runtime.to_string().as_str(),
                cpu_limit = cpu_limit,
                memory_limit = memory_limit,
                new_cpu_limit = instance.cpu,
                new_memory_limit = format!("{}GiB", instance.memory).as_str(),
                "instance limits are chagned, updating"
            );

            let mut metadata = res.get("metadata").unwrap().clone();
            metadata
                .get_mut("config")
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(
                    "limits.cpu".to_string(),
                    serde_json::Value::String(instance.cpu.to_string()),
                );
            metadata
                .get_mut("config")
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(
                    "limits.memory".to_string(),
                    serde_json::Value::String(format!("{}GiB", instance.memory)),
                );

            let res = self
                .client
                .put(url)
                .json(&metadata)
                .send()
                .await?
                .json()
                .await?;
            check_error(&res)?;
        }
        Ok(())
    }

    async fn stop_instance(&self, user: &User, instance: &Instance) -> Result<()> {
        info!(
            username = user.username.as_str(),
            instance = instance.name.as_str(),
            runtime = instance.runtime.to_string().as_str(),
            "stopping instance"
        );
        let name = format!("{}-{}", user.username, instance.name);
        let url = format!(
            "{}/1.0/instances/{}/state?project={}",
            LXD_SERVER_URL.as_str(),
            name,
            LXD_PROJECT.as_str(),
        );

        let res: serde_json::Value = self
            .client
            .put(url)
            .json(&serde_json::json!({
               "action": "stop"
            }))
            .send()
            .await?
            .json()
            .await?;
        check_error(&res)
    }

    async fn update_instance_status(&self, user: &User, instance: &Instance) -> Result<()> {
        let name = format!("{}-{}", user.username, instance.name);
        let url = format!(
            "{}/1.0/instances/{}/state?project={}",
            LXD_SERVER_URL.as_str(),
            name,
            LXD_PROJECT.as_str(),
        );
        let res: serde_json::Value = self.client.get(url).send().await?.json().await?;
        if is_not_found(&res) {
            if instance.status == InstanceStatus::Creating {
                return Ok(());
            }
            return self
                .storage
                .read_write(|state| {
                    if let Some(i) = state
                        .find_mut_user(&user.username)
                        .and_then(|u| u.find_mut_instance(&instance.name))
                    {
                        if i.stage == InstanceStage::Deleted {
                            state
                                .find_mut_user(&user.username)
                                .unwrap()
                                .remove_instance(&instance.name);
                        } else {
                            i.status = InstanceStatus::Missing;
                            warn!(
                                username = user.username.as_str(),
                                instance = instance.name.as_str(),
                                runtime = instance.runtime.to_string().as_str(),
                                "instance is missing unexpectedly"
                            );
                        }
                    }
                    true
                })
                .await
                .map_err(|e| anyhow!(e));
        }
        check_error(&res)?;

        let status = parse_instance_status(&res).unwrap_or_default();
        let internal_ip = parse_internal_ip(&res);
        self.storage
            .read_write(|state| {
                if let Some(i) = state
                    .find_mut_user(&user.username)
                    .and_then(|u| u.find_mut_instance(&instance.name))
                {
                    match i.stage {
                        InstanceStage::Stopped => {
                            if status == "Stopped" {
                                i.status = InstanceStatus::Stopped;
                            }
                        }
                        InstanceStage::Running => {
                            if status == "Stopped" && i.status == InstanceStatus::Creating {
                                i.status = InstanceStatus::Starting;
                            } else if status == "Running" {
                                i.status = InstanceStatus::Running;
                            }
                            i.internal_ip = internal_ip.clone();
                        }
                        InstanceStage::Deleted => {
                            if status == "Stopped" {
                                i.status = InstanceStatus::Deleting;
                            }
                        }
                    }
                }
                true
            })
            .await
            .map_err(|e| anyhow!(e))
    }
}

fn get_image_alias(image: &Image) -> Result<String> {
    match image {
        Image::CentOS7 => Ok("centos/7/cloud".to_owned()),
        Image::CentOS9Stream => Ok("centos/9-Stream".to_owned()),
        Image::Ubuntu2004 => Ok("ubuntu/20.04/cloud".to_owned()),
        Image::Ubuntu2204 => Ok("ubuntu/22.04/cloud".to_owned()),
        _ => Err(anyhow!("invalid image {}", image)),
    }
}

fn get_instance_type(runtime: &Runtime) -> Result<String> {
    match runtime {
        Runtime::Lxc => Ok("container".to_owned()),
        Runtime::Kvm => Ok("virtual-machine".to_owned()),
        _ => Err(anyhow!("invalid runtime {}", runtime)),
    }
}

crate fn check_error(res: &serde_json::Value) -> Result<()> {
    let ec = res.get("error_code");
    if ec.is_none() {
        return Err(anyhow!("no error code"));
    }
    if let Some(0) = ec.unwrap().as_i64() {
        return Ok(());
    }
    res.get("error").map_or_else(
        || Err(anyhow!("no error message")),
        |e| Err(anyhow!(e.to_string())),
    )
}

fn is_not_found(res: &serde_json::Value) -> bool {
    matches!(res.get("error_code").and_then(|e| e.as_i64()), Some(404))
}

fn parse_instance_status(res: &serde_json::Value) -> Option<String> {
    res.get("metadata")
        .and_then(|v| v.get("status"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_owned())
}

fn parse_internal_ip(res: &serde_json::Value) -> Option<String> {
    let network = res.get("metadata").and_then(|v| v.get("network"))?;
    let eth = if network.get("eth0").is_some() {
        "eth0"
    } else {
        "enp5s0"
    };
    network
        .get(eth)
        .and_then(|v| v.get("addresses"))
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            for v in arr {
                let is_ipv4 = v.get("family").and_then(|f| f.as_str()).unwrap_or("") == "inet";
                let is_global = v.get("scope").and_then(|f| f.as_str()).unwrap_or("") == "global";
                if is_ipv4 && is_global {
                    return v
                        .get("address")
                        .and_then(|a| a.as_str())
                        .map(|a| a.to_owned());
                }
            }
            None
        })
}
