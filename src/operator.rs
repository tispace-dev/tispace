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
            // For each instance.
            // 1. Ensure namespace is created.
            // 2. Ensure rootfs is initialized.
            // 3. Ensure pod is running.
        }
    }
}
