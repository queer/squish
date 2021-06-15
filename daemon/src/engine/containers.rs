use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use haikunator::Haikunator;
use nix::sys::signal::kill;
use nix::sys::signal;
use nix::unistd::Pid;
use tokio::time::sleep;

#[derive(Debug)]
pub struct Container {
    pub name: String,
    pub pid: nix::unistd::Pid,
    pub slirp_pid: nix::unistd::Pid,
    pub id: String,
    pub created_at: u128,
}

impl Into<libsquish::RunningContainer> for &Container {
    fn into(self) -> libsquish::RunningContainer {
        libsquish::RunningContainer {
            id: self.id.clone(),
            name: self.name.clone(),
            pid: self.pid.as_raw(),
        }
    }
}

#[derive(Debug)]
pub struct ContainerState {
    id_map: HashMap<String, Container>,
    pid_id_map: HashMap<Pid, String>,
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
        slirp_pid: nix::unistd::Pid,
        id: String,
        name: String,
    ) -> Result<(), Box<dyn Error + '_>> {
        self.id_map.insert(
            id.clone(),
            Container {
                name,
                id: id.clone(),
                pid,
                slirp_pid,
                created_at: libsquish::now()?,
            },
        );
        self.pid_id_map.insert(pid, id.clone());
        Ok(())
    }

    pub fn remove_container(&mut self, id: &String) -> Result<(), Box<dyn Error + '_>> {
        let container = self.id_map.remove(id);
        if let Some(container) = container {
            self.pid_id_map.remove(&container.pid);
            kill(container.slirp_pid, signal::SIGTERM)?;
        }
        Ok(())
    }

    /// id <-> name
    pub fn running_containers(&self) -> Vec<libsquish::RunningContainer> {
        let mut out = vec![];
        for v in self.id_map.values() {
            out.push(v.into());
        }
        out
    }
}

pub async fn reap_children(state: Arc<Mutex<ContainerState>>) {
    loop {
        sleep(Duration::from_millis(100)).await;
        let mut container_state = state.lock().unwrap();

        let clone = container_state.pid_id_map.clone();

        for (pid, id) in clone.iter() {
            // debug!("checking {}", pid.as_raw());
            let path = format!("/proc/{}", pid.as_raw());
            let path = Path::new(&path);
            if !path.exists() {
                match cleanup_container(&mut container_state, id) {
                    Ok(_) => info!("cleaned up dead container {}", pid.as_raw()),
                    Err(e) => error!("error cleaning up dead container {}: {}", pid.as_raw(), e),
                }
            }
        }
    }
}

fn cleanup_container<'a>(state: &'a mut ContainerState, id: &'a String) -> Result<(), Box<dyn Error + 'a>> {
    state.remove_container(id)?;
    fs::remove_dir_all(path_to(id))?;
    Ok(())
}

pub fn path_to(id: &String) -> String {
    format!("container/{}", id)
}