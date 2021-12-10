use tokio::time::{sleep, Duration};

use crate::storage::Storage;

pub struct Operator {
    storage: Storage,
}

impl Operator {
    pub fn new(storage: Storage) -> Self {
        Operator { storage }
    }

    pub async fn run(&self) {
        loop {
            let _state = self.storage.snapshot();
            // For each instance (stage != Deleting)
            // 1. Ensure namespace is created.
            // 2. Ensure rootfs is initialized.
            // 3. Ensure service is created.
            // 4. Ensure pod is running.
            sleep(Duration::from_secs(1)).await;
        }
    }
}
