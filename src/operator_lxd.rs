use crate::storage::Storage;

pub struct Operator {
    _storage: Storage,
}

impl Operator {
    pub fn new(storage: Storage) -> Self {
        Operator { _storage: storage }
    }

    pub async fn run(&self) {}
}
