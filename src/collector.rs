use anyhow::{anyhow, Result};
use k8s_openapi::api::core::v1::Node as KubeNode;
use k8s_quantity_parser::QuantityParser;
use kube::core::params::ListParams;
use kube::{Api, Client as KubeClient};
use reqwest::Client as ReqwestClient;
use tokio::time::{sleep, Duration};
use tracing::warn;

use crate::env::{
    CPU_OVERCOMMIT_FACTOR, LXD_PROJECT, LXD_SERVER_URL, LXD_STORAGE_POOL_DRIVER,
    MEMORY_OVERCOMMIT_FACTOR,
};
use crate::model::{Node, Runtime, StoragePool};
use crate::operator_lxd::check_error;
use crate::storage::Storage;

pub struct Collector {
    storage: Storage,
    kube_client: Option<KubeClient>,
    lxd_client: Option<ReqwestClient>,
}

impl Collector {
    pub fn new(
        storage: Storage,
        kube_client: Option<KubeClient>,
        lxd_client: Option<ReqwestClient>,
    ) -> Self {
        Collector {
            storage,
            kube_client,
            lxd_client,
        }
    }

    pub async fn run(&self) {
        loop {
            self.run_once().await;
            sleep(Duration::from_secs(60)).await;
        }
    }

    async fn run_once(&self) {
        let mut nodes = Vec::new();
        if let Some(kube_client) = &self.kube_client {
            match self.collect_kube_nodes(kube_client).await {
                Ok(n) => nodes.extend(n),
                Err(e) => {
                    warn!("failed to collect kube nodes: {}", e);
                    return;
                }
            }
        }
        if let Some(lxd_client) = &self.lxd_client {
            match self.collect_lxd_nodes(lxd_client).await {
                Ok(n) => nodes.extend(n),
                Err(e) => {
                    warn!("failed to collect lxd nodes: {}", e);
                    return;
                }
            }
        }
        nodes.sort_by(|a, b| a.name.cmp(&b.name));

        let mut merged_nodes = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            let mut j = i;

            let mut runtimes: Vec<Runtime> = Vec::new();
            let mut storage_pools: Vec<StoragePool> = Vec::new();
            let mut cpu_total = 0;
            let mut memory_total = 0;
            while j < nodes.len() && nodes[i].name == nodes[j].name {
                for runtime in &nodes[j].runtimes {
                    if !runtimes.contains(runtime) {
                        runtimes.push(runtime.clone());
                    }
                }
                for storage_pool in &nodes[j].storage_pools {
                    if !storage_pools.iter().any(|s| s.name == storage_pool.name) {
                        storage_pools.push(storage_pool.clone());
                    }
                }
                if cpu_total == 0 || nodes[j].cpu_total > 0 && nodes[j].cpu_total < cpu_total {
                    cpu_total = nodes[j].cpu_total;
                }
                if memory_total == 0
                    || nodes[j].memory_total > 0 && nodes[j].memory_total < memory_total
                {
                    memory_total = nodes[j].memory_total;
                }
                j += 1;
            }

            let storage_total = storage_pools.iter().map(|s| s.total).sum();
            let storage_used = storage_pools.iter().map(|s| s.used).sum();

            merged_nodes.push(Node {
                name: nodes[i].name.clone(),
                runtimes,
                storage_pools,
                cpu_total: overcommit_cpu(cpu_total),
                cpu_allocated: 0,
                memory_total: overcommit_memory(memory_total),
                memory_allocated: 0,
                storage_total,
                storage_used,
                storage_allocated: 0,
            });
            i = j;
        }

        if let Err(e) = self
            .storage
            .read_write(|state| {
                state.nodes = merged_nodes.clone();
                true
            })
            .await
        {
            warn!("failed to read/write storage: {}", e);
        }
    }

    async fn collect_kube_nodes(&self, kube_client: &KubeClient) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();
        let kube_nodes: Api<KubeNode> = Api::all(kube_client.clone());
        for kube_node in kube_nodes.list(&ListParams::default()).await? {
            let name = kube_node.metadata.name.clone().unwrap();
            let cpu_total: usize = kube_node
                .status
                .as_ref()
                .and_then(|s| s.capacity.as_ref())
                .and_then(|c| {
                    c.get("cpu").map(|v| {
                        v.to_milli_cpus().ok().flatten().unwrap_or_default() as usize / 1000
                    })
                })
                .unwrap_or_default();
            let memory_total: usize = kube_node
                .status
                .as_ref()
                .and_then(|s| s.capacity.as_ref())
                .and_then(|c| {
                    c.get("memory")
                        .map(|v| v.to_bytes().ok().flatten().unwrap_or_default() as usize >> 30)
                })
                .unwrap_or_default();
            nodes.push(Node {
                name: name.clone(),
                storage_pools: Vec::new(),
                runtimes: vec![Runtime::Kata, Runtime::Runc],
                cpu_total,
                cpu_allocated: 0,
                memory_total,
                memory_allocated: 0,
                storage_total: 0,
                storage_used: 0,
                storage_allocated: 0,
            });
        }
        Ok(nodes)
    }

    async fn collect_lxd_nodes(&self, lxd_client: &ReqwestClient) -> Result<Vec<Node>> {
        let node_names = list_lxd_nodes(lxd_client).await?;
        let mut pool_names = Vec::new();
        for pool_name in list_lxd_storage_pools(lxd_client).await? {
            let driver = get_lxd_storage_pool_driver(lxd_client, &pool_name).await?;
            if driver == LXD_STORAGE_POOL_DRIVER.as_str() {
                pool_names.push(pool_name);
            }
        }
        let mut nodes = Vec::new();
        for node_name in &node_names {
            let (cpu_total, memory_total) = get_lxd_node_capacity(lxd_client, node_name).await?;
            let mut node = Node {
                name: node_name.clone(),
                storage_pools: Vec::new(),
                runtimes: vec![Runtime::Lxc, Runtime::Kvm],
                cpu_total,
                cpu_allocated: 0,
                memory_total,
                memory_allocated: 0,
                storage_total: 0,
                storage_used: 0,
                storage_allocated: 0,
            };
            for pool_name in &pool_names {
                let (total, used) =
                    get_lxd_storage_pool_usage(lxd_client, node_name, pool_name).await?;
                let storage_pool = StoragePool {
                    name: pool_name.clone(),
                    total,
                    used,
                    allocated: 0,
                };
                node.storage_pools.push(storage_pool);
            }
            nodes.push(node);
        }
        Ok(nodes)
    }
}

fn overcommit_cpu(cpu: usize) -> usize {
    (cpu as f64 * CPU_OVERCOMMIT_FACTOR.to_owned()) as usize
}

fn overcommit_memory(memory: usize) -> usize {
    (memory as f64 * MEMORY_OVERCOMMIT_FACTOR.to_owned()) as usize
}

async fn list_lxd_nodes(lxd_client: &ReqwestClient) -> Result<Vec<String>> {
    let url = format!("{}/1.0/cluster/members", LXD_SERVER_URL.as_str());
    let res: serde_json::Value = lxd_client.get(url).send().await?.json().await?;
    check_error(&res)?;
    // The response is like:
    // {
    //   "metadata": [
    //     "/1.0/cluster/members/lxd01",
    //     "/1.0/cluster/members/lxd02"
    //   ],
    //   "status": "Success",
    //   "status_code": 200,
    //   "type": "sync"
    // }
    let nodes: Vec<String> = res
        .get("metadata")
        .ok_or_else(|| anyhow!("no metadata"))?
        .as_array()
        .ok_or_else(|| anyhow!("no metadata array"))?
        .iter()
        .map(|n| {
            n.as_str()
                .unwrap()
                .strip_prefix("/1.0/cluster/members/")
                .unwrap()
                .to_owned()
        })
        .collect();
    Ok(nodes)
}

async fn list_lxd_storage_pools(lxd_client: &ReqwestClient) -> Result<Vec<String>> {
    let url = format!(
        "{}/1.0/storage-pools?project={}",
        LXD_SERVER_URL.as_str(),
        LXD_PROJECT.as_str()
    );
    let res: serde_json::Value = lxd_client.get(url).send().await?.json().await?;
    check_error(&res)?;
    // The response is like:
    // {
    //   "metadata": [
    //     "/1.0/storage-pools/local",
    //     "/1.0/storage-pools/remote"
    //   ],
    //   "status": "Success",
    //   "status_code": 200,
    //   "type": "sync"
    // }
    let pools: Vec<String> = res
        .get("metadata")
        .ok_or_else(|| anyhow!("no metadata"))?
        .as_array()
        .ok_or_else(|| anyhow!("no metadata array"))?
        .iter()
        .map(|p| {
            p.as_str()
                .unwrap()
                .strip_prefix("/1.0/storage-pools/")
                .unwrap()
                .to_owned()
        })
        .collect();
    Ok(pools)
}

async fn get_lxd_storage_pool_driver(
    lxd_client: &ReqwestClient,
    pool_name: &str,
) -> Result<String> {
    let url = format!(
        "{}/1.0/storage-pools/{}",
        LXD_SERVER_URL.as_str(),
        pool_name
    );
    let res: serde_json::Value = lxd_client.get(url).send().await?.json().await?;
    check_error(&res)?;
    // The response is like:
    // {
    //   "metadata": {
    //     "config": {
    //       "volume.block.filesystem": "ext4",
    //       "volume.size": "50GiB"
    //     },
    //     "description": "Local SSD pool",
    //     "driver": "zfs",
    //     "locations": [
    //       "lxd01",
    //       "lxd02",
    //       "lxd03"
    //     ],
    //     "name": "local",
    //     "status": "Created",
    //     "used_by": [
    //       "/1.0/profiles/default",
    //       "/1.0/instances/c1"
    //     ]
    //   },
    //   "status": "Success",
    //   "status_code": 200,
    //   "type": "sync"
    // }
    let driver = res
        .get("metadata")
        .ok_or_else(|| anyhow!("no metadata"))?
        .get("driver")
        .ok_or_else(|| anyhow!("no driver"))?
        .as_str()
        .ok_or_else(|| anyhow!("driver is not a string"))?
        .to_owned();
    Ok(driver)
}

async fn get_lxd_storage_pool_usage(
    lxd_client: &ReqwestClient,
    node_name: &str,
    pool_name: &str,
) -> Result<(usize, usize)> {
    let url = format!(
        "{}/1.0/storage-pools/{}/resources?target={}",
        LXD_SERVER_URL.as_str(),
        pool_name,
        node_name
    );
    let res: serde_json::Value = lxd_client.get(url).send().await?.json().await?;
    check_error(&res)?;
    // The response is like:
    // {
    //   "metadata": {
    //     "inodes": {
    //       "total": 30709993797,
    //       "used": 23937695
    //     },
    //     "space": {
    //       "total": 420100937728,
    //       "used": 343537419776
    //     }
    //   },
    //   "status": "Success",
    //   "status_code": 200,
    //   "type": "sync"
    // }
    let space = res
        .get("metadata")
        .ok_or_else(|| anyhow!("no metadata"))?
        .get("space")
        .ok_or_else(|| anyhow!("no space"))?;
    // The space is in bytes, but we want to return in GiB.
    let total = space.get("total").map_or(0, |v| v.as_u64().unwrap()) >> 30;
    let used = space.get("used").map_or(0, |v| v.as_u64().unwrap()) >> 30;
    Ok((total as usize, used as usize))
}

async fn get_lxd_node_capacity(
    lxd_client: &ReqwestClient,
    node_name: &str,
) -> Result<(usize, usize)> {
    let url = format!(
        "{}/1.0/resources?target={}",
        LXD_SERVER_URL.as_str(),
        node_name
    );
    let res: serde_json::Value = lxd_client.get(url).send().await?.json().await?;
    check_error(&res)?;
    // The response is like:
    // {
    //   "metadata": {
    //     "cpu": {
    //       "architecture": "x86_64",
    //       "sockets": [
    //         ...
    //       ],
    //       "total": 1
    //     },
    //     "memory": {
    //       "hugepages_size": 2097152,
    //       "hugepages_total": 429284917248,
    //       "hugepages_used": 429284917248,
    //       "nodes": null,
    //       "total": 687194767360,
    //       "used": 557450502144
    //     }
    //     ...
    //   },
    //   "status": "Success",
    //   "status_code": 200,
    //   "type": "sync"
    // }
    let cpu_total = res
        .get("metadata")
        .ok_or_else(|| anyhow!("no metadata"))?
        .get("cpu")
        .ok_or_else(|| anyhow!("no cpu"))?
        .get("total")
        .map_or(0, |v| v.as_u64().unwrap());
    let memory_total = res
        .get("metadata")
        .ok_or_else(|| anyhow!("no metadata"))?
        .get("memory")
        .ok_or_else(|| anyhow!("no memory"))?
        .get("total")
        .map_or(0, |v| v.as_u64().unwrap())
        >> 30;
    Ok((cpu_total as usize, memory_total as usize))
}
