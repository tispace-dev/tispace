use rand::seq::SliceRandom;
use rand::thread_rng;
use std::cmp::Ordering;
use std::collections::HashSet;

use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::env::EXTERNAL_IP_POOL;
use crate::model::{InstanceStatus, Node, Runtime, State, StoragePool};
use crate::storage::Storage;

pub struct Scheduler {
    storage: Storage,
}

impl Scheduler {
    pub fn new(storage: Storage) -> Self {
        Scheduler { storage }
    }

    pub async fn run(&self) {
        loop {
            self.run_once().await;
            sleep(Duration::from_secs(3)).await;
        }
    }

    async fn run_once(&self) {
        if let Err(e) = self
            .storage
            .read_write(|state| {
                Scheduler::allocate_ip(state);
                Scheduler::schedule(state);
                true
            })
            .await
        {
            warn!("failed to read/write storage: {}", e);
        }
    }

    fn allocate_ip(state: &mut State) {
        let mut allocated_ips = HashSet::new();
        for u in &state.users {
            for i in &u.instances {
                if let Some(ip) = &i.external_ip {
                    allocated_ips.insert(ip.clone());
                }
            }
        }

        let mut ip_pool = EXTERNAL_IP_POOL.clone();
        ip_pool.shuffle(&mut thread_rng());

        for u in &mut state.users {
            for i in &mut u.instances {
                match i.runtime {
                    Runtime::Lxc | Runtime::Kvm => {
                        if i.external_ip.is_none() {
                            for ip in ip_pool.iter() {
                                if !allocated_ips.contains(ip) {
                                    i.external_ip = Some(ip.clone());
                                    allocated_ips.insert(ip.clone());
                                    break;
                                }
                            }
                            if i.external_ip.is_none() {
                                warn!("external IP pool is exhausted, no more IPs available");
                                return;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn schedule(state: &mut State) {
        let mut instances = Vec::new();
        for u in &mut state.users {
            for i in &mut u.instances {
                if i.status != InstanceStatus::Creating {
                    continue;
                }
                match i.runtime {
                    Runtime::Lxc | Runtime::Kvm => {
                        if i.external_ip.is_some()
                            && (i.node_name.is_none() || i.storage_pool.is_none())
                        {
                            instances.push(i);
                        }
                    }
                    Runtime::Runc | Runtime::Kata => {
                        if i.node_name.is_none() {
                            instances.push(i);
                        }
                    }
                }
            }
        }
        if instances.is_empty() {
            return;
        }

        for i in instances {
            let mut best_node: Option<&mut Node> = None;
            for n in &mut state.nodes {
                if let Some(node_name) = &i.node_name {
                    if node_name != &n.name {
                        continue;
                    }
                }
                if !n.runtimes.contains(&i.runtime) {
                    continue;
                }
                if i.cpu + n.cpu_allocated > n.cpu_total
                    || i.memory + n.memory_allocated > n.memory_total
                    || i.disk_size + n.storage_allocated > n.storage_total
                    || i.disk_size + n.storage_used > n.storage_total
                {
                    continue;
                }
                if !n.storage_pools.iter().any(|s| {
                    if let Some(storage_pool) = &i.storage_pool {
                        if storage_pool != &s.name {
                            return false;
                        }
                    }
                    s.allocated.max(s.used) + i.disk_size <= s.total
                }) {
                    continue;
                }

                if let Some(bn) = &best_node {
                    let a = (n.cpu_total - n.cpu_allocated).cmp(&(bn.cpu_total - bn.cpu_allocated));
                    let b = (n.memory_total - n.memory_allocated)
                        .cmp(&(bn.memory_total - bn.memory_allocated));
                    let c = (n.storage_total - n.storage_allocated.max(n.storage_used))
                        .cmp(&(bn.storage_total - bn.storage_allocated.max(bn.storage_used)));
                    if a == Ordering::Greater
                        || a == Ordering::Equal && b == Ordering::Greater
                        || a == Ordering::Equal && b == Ordering::Equal && c == Ordering::Greater
                    {
                        best_node = Some(n);
                    }
                } else {
                    best_node = Some(n);
                }
            }
            if best_node.is_none() {
                warn!(
                    "no node has enough resources to schedule instance {}",
                    i.name
                );
                continue;
            }

            let best_node = best_node.unwrap();
            let mut best_storage_pool: Option<&mut StoragePool> = None;
            for s in &mut best_node.storage_pools {
                if let Some(storage_pool) = &i.storage_pool {
                    if storage_pool != &s.name {
                        continue;
                    }
                }
                if let Some(bs) = &best_storage_pool {
                    if s.total - s.allocated.max(s.used) > bs.total - bs.allocated.max(bs.used) {
                        best_storage_pool = Some(s);
                    }
                } else {
                    best_storage_pool = Some(s);
                }
            }
            let best_storage_pool = best_storage_pool.unwrap();

            best_storage_pool.allocated += i.disk_size;
            best_node.cpu_allocated += i.cpu;
            best_node.memory_allocated += i.memory;
            best_node.storage_allocated += i.disk_size;
            i.node_name = Some(best_node.name.clone());

            match i.runtime {
                Runtime::Lxc | Runtime::Kvm => {
                    i.storage_pool = Some(best_storage_pool.name.clone());
                    info!(
                        "scheduled instance {} to node {} on storage pool {}",
                        i.name, best_node.name, best_storage_pool.name
                    );
                }
                Runtime::Runc | Runtime::Kata => {
                    // Runc and Kata doesn't support specifying storage pool.
                    info!("scheduled instance {} to node {}", i.name, best_node.name);
                }
            }
        }
    }
}
