use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use haikunator::Haikunator;
use nix::sys::wait::{WaitPidFlag, WaitStatus};
use nix::sys::wait::waitpid;
use nix::unistd::Pid;
use tokio::signal::unix::{signal, SignalKind};

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

    pub fn add_container(
        &mut self,
        pid: nix::unistd::Pid,
    ) -> Result<(), Box<dyn std::error::Error + '_>> {
        // TODO: Move this to utils?
        let haiku = Haikunator::default();
        let name = haiku.haikunate();

        let container_id = self.gen_id(&name);
        self.id_map.insert(
            container_id.clone(),
            Container {
                name,
                id: container_id.clone(),
                pid,
            },
        );
        self.pid_id_map.insert(pid, container_id.clone());
        Ok(())
    }

    fn gen_id(&self, input: &String) -> String {
        let hash = hmac_sha256::Hash::hash(input.as_bytes());
        format!("{:x?}", hash)
    }

    pub fn remove_container(&mut self, id: &String) -> Result<(), Box<dyn std::error::Error + '_>> {
        let container = self.id_map.remove(id);
        if let Some(container) = container {
            self.pid_id_map.remove(&container.pid);
        }
        Ok(())
    }
}

pub async fn signal_handler(state: Arc<Mutex<ContainerState>>) {
    let mut stream = signal(SignalKind::child()).unwrap();
    loop {
        stream.recv().await;
        let mut container_state = state.lock().unwrap();

        let clone = container_state.pid_id_map.clone();

        for (pid, id) in clone.iter() {
            debug!("checking {}", pid.as_raw());
            match waitpid(Some(*pid), Some(WaitPidFlag::WNOHANG)) {
                Ok(status) => {
                    match status {
                        WaitStatus::Exited(_, _) => {
                            if let Ok(_) = container_state.remove_container(id) {
                                info!("cleaned up dead container {}", pid.as_raw());
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    error!("couldn't check wait status of {}: {}", pid.as_raw(), e);
                }
            }
        }
    }
}
