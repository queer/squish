use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use haikunator::Haikunator;
use nix::unistd::Pid;

#[derive(Debug)]
pub struct ContainerState {
    id_map: HashMap<String, Container>,
    pid_id_map: HashMap<Pid, String>,
}

#[derive(Debug)]
pub struct Container {
    pub name: String,
    pub pid: nix::unistd::Pid,
    pub id: String,
}

impl ContainerState {
    pub fn new() -> Self {
        ContainerState {
            id_map: HashMap::new(),
            pid_id_map: HashMap::new(),
        }
    }

    pub fn generate_id() -> (String, String) {
        let haiku = Haikunator::default();
        let name = haiku.haikunate();
        let hash = hmac_sha256::Hash::hash(name.as_bytes());
        let id = hex::encode(hash);
        (id, name)
    }

    pub fn add_container(
        &mut self,
        pid: nix::unistd::Pid,
        id: String,
        name: String,
    ) -> Result<(), Box<dyn std::error::Error + '_>> {
        self.id_map.insert(
            id.clone(),
            Container {
                name,
                id: id.clone(),
                pid,
            },
        );
        self.pid_id_map.insert(pid, id.clone());
        Ok(())
    }

    pub fn remove_container(&mut self, id: &String) -> Result<(), Box<dyn std::error::Error + '_>> {
        let container = self.id_map.remove(id);
        if let Some(container) = container {
            self.pid_id_map.remove(&container.pid);
        }
        Ok(())
    }
}

pub async fn reap_children(state: Arc<Mutex<ContainerState>>) {
    loop {
        sleep(Duration::from_millis(100));
        let mut container_state = state.lock().unwrap();

        let clone = container_state.pid_id_map.clone();

        for (pid, id) in clone.iter() {
            debug!("checking {}", pid.as_raw());
            let path = format!("/proc/{}", pid.as_raw());
            let path = Path::new(&path);
            if !path.exists() {
                if let Ok(_) = container_state.remove_container(id) {
                    info!("cleaned up dead container {}", pid.as_raw());
                }
            }
        }
    }
}
