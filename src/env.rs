use std::collections::HashMap;
use std::net::Ipv4Addr;

use once_cell::sync::Lazy;

crate static GOOGLE_CLIENT_ID: Lazy<String> =
    Lazy::new(|| std::env::var("GOOGLE_CLIENT_ID").unwrap());

crate static STORAGE_CLASS_NAME: Lazy<String> =
    Lazy::new(|| std::env::var("STORAGE_CLASS_NAME").unwrap_or_else(|_| "openebs-lvm".to_owned()));

crate static DEFAULT_ROOTFS_IMAGE_TAG: Lazy<String> =
    Lazy::new(|| std::env::var("DEFAULT_ROOTFS_IMAGE_TAG").unwrap_or_else(|_| "latest".to_owned()));

crate static LXD_PROJECT: Lazy<String> =
    Lazy::new(|| std::env::var("LXD_PROJECT").unwrap_or_else(|_| "tispace".to_owned()));

pub static LXD_CLIENT_CERT: Lazy<String> =
    Lazy::new(|| std::env::var("LXD_CLIENT_CERT").unwrap_or_default());

crate static LXD_SERVER_URL: Lazy<String> = Lazy::new(|| std::env::var("LXD_SERVER_URL").unwrap());

crate static LXD_IMAGE_SERVER_URL: Lazy<String> = Lazy::new(|| {
    std::env::var("LXD_IMAGE_SERVER_URL")
        .unwrap_or_else(|_| "https://mirrors.tuna.tsinghua.edu.cn/lxc-images".to_owned())
});

crate static LXD_STORAGE_POOL_DRIVER: Lazy<String> =
    Lazy::new(|| std::env::var("LXD_STORAGE_DRIVER").unwrap_or_else(|_| "lvm".to_owned()));

// Kubernetes cluster and LXD cluster may share the same storage pool but with different names.
// LXD_STORAGE_MAPPING is a map from openebs volume name to LXD storage pool name.
crate static LXD_STORAGE_POOL_MAPPING: Lazy<HashMap<String, String>> = Lazy::new(|| {
    if let Ok(s) = std::env::var("LXD_STORAGE_POOL_MAPPING") {
        let mut m = HashMap::new();
        for s in s.split(',') {
            let mut parts = s.splitn(2, '=');
            let vg_name = parts.next().unwrap();
            let storage_pool = parts.next().unwrap();
            m.insert(vg_name.to_owned(), storage_pool.to_owned());
        }
        m
    } else {
        HashMap::new()
    }
});

// A list of IP addresses for instances exposed outside of the cluster.
// The value of the environment variable is a comma-separated list of IP ranges.
// Each IP range is an explicit inclusive start-end ip address. For example:
// EXTERNAL_IP_POOL=192.168.100.1-192.168.100.254,192.168.101.1-192.168.101.254.
// Please note that the IP addresses must be in the same subnet with same prefix length.
// The prefix length is configured by variable EXTERNAL_IP_PREFIX_LENGTH.
crate static EXTERNAL_IP_POOL: Lazy<Vec<String>> = Lazy::new(|| {
    if let Ok(s) = std::env::var("EXTERNAL_IP_POOL") {
        s.split(',')
            .flat_map(|s| {
                let mut parts = s.splitn(2, '-');
                let start = parts
                    .next()
                    .unwrap()
                    .parse::<Ipv4Addr>()
                    .unwrap()
                    .octets()
                    .into_iter()
                    .fold(0, |a, b| (a << 8) + b as u32);
                let end = parts
                    .next()
                    .unwrap()
                    .parse::<Ipv4Addr>()
                    .unwrap()
                    .octets()
                    .into_iter()
                    .fold(0, |a, b| (a << 8) + b as u32);
                (start..=end)
                    .into_iter()
                    .map(Ipv4Addr::from)
                    .map(|a| a.to_string())
            })
            .collect()
    } else {
        Vec::new()
    }
});

// The prefix length of the IP addresses in the EXTERNAL_IP_POOL.
crate static EXTERNAL_IP_PREFIX_LENGTH: Lazy<u8> = Lazy::new(|| {
    if let Ok(s) = std::env::var("EXTERNAL_IP_PREFIX_LENGTH") {
        s.parse::<u8>().unwrap()
    } else {
        32
    }
});

crate static CPU_OVERCOMMIT_FACTOR: Lazy<f64> = Lazy::new(|| {
    if let Ok(s) = std::env::var("CPU_OVERCOMMIT_FACTOR") {
        s.parse::<f64>().unwrap()
    } else {
        1.0
    }
});

crate static MEMORY_OVERCOMMIT_FACTOR: Lazy<f64> = Lazy::new(|| {
    if let Ok(s) = std::env::var("MEMORY_OVERCOMMIT_FACTOR") {
        s.parse::<f64>().unwrap()
    } else {
        1.0
    }
});
