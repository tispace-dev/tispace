use std::io::ErrorKind;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{error::*, model::State};

#[derive(Clone)]
pub struct Storage {
    path: String,
    state: Arc<RwLock<State>>,
}

impl Storage {
    pub async fn load(path: &str) -> Result<Self> {
        let mut state = State::new();
        match tokio::fs::read(path).await {
            Ok(contents) => {
                state = serde_json::from_slice(&contents)?;
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(Box::new(e)),
        }
        Ok(Storage {
            path: path.to_string(),
            state: Arc::new(RwLock::new(state)),
        })
    }

    crate async fn read_only<F>(&self, mut f: F)
    where
        F: FnMut(&State),
    {
        f(&*self.state.read().await)
    }

    crate async fn read_write<F>(&self, mut f: F) -> Result<()>
    where
        F: FnMut(&mut State) -> bool,
    {
        let state = &mut *self.state.write().await;
        let mut new_state = state.clone();
        if f(&mut new_state) {
            let data = serde_json::to_vec(&new_state).unwrap();
            let tmp_path = format!("{}.tmp", self.path);
            tokio::fs::write(&tmp_path, data).await?;
            tokio::fs::rename(&tmp_path, &self.path).await?;
            *state = new_state;
        }
        Ok(())
    }

    crate async fn snapshot(&self) -> State {
        let state = &*self.state.read().await;
        state.clone()
    }
}
