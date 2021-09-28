use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use haikunator::Haikunator;
use nix::sys::signal;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use tokio::time::sleep;

/// A squish container. A container is basically just a tracked pid, that has a
/// hexadecimal id and a name attached to it. Containers also contain a pid for
/// their respective slirp4netns instances, as well as a timestamp for when
/// they were created.
#[derive(Debug, Clone)]
pub struct Container {
    pub name: String,
    pub pid: nix::unistd::Pid, // TODO: Support multi-pid
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

/// The global state of the daemon. To avoid constant locking, this is kept
/// fairly small. It contains a mapping from container ids to `Container`
/// structs, as well as a mapping from container pids to container ids. This
/// data can be relied on to always be up to date.
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

    /// Generates a (id, name) tuple. The id is a SHA256 hash of the name.
    pub fn generate_id() -> (String, String) {
        let haiku = Haikunator::default();
        let name = haiku.haikunate();
        let hash = hmac_sha256::Hash::hash(name.as_bytes());
        let id = hex::encode(hash);
        (id, name)
    }

    /// Add a container to the global container state.
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

    /// Remove a container or set of containers based on "fuzzy" matching of
    /// container names or ids. This partially matches the container name or id
    /// based on starting characters. That is, a container is removed if its
    /// name or its id starts with the partial value passed in. This is not a
    /// general substring match.
    pub fn fuzzy_remove_container(
        &mut self,
        partial_id_or_name: &String,
    ) -> Result<Vec<String>, Box<dyn Error + '_>> {
        let matches: Vec<&Container> = (&self.id_map)
            .into_iter()
            .filter(|(id, container)| {
                id.starts_with(partial_id_or_name) || container.name.starts_with(partial_id_or_name)
            })
            .map(|(_id, container)| container)
            .collect();
        let mut matched_ids = vec![];
        for container in matches {
            matched_ids.push(container.id.clone());
        }
        self.remove_all_containers(matched_ids.clone())?;
        Ok(matched_ids)
    }

    /// Remove the container with the given id.
    pub fn remove_container(&mut self, id: &String) -> Result<(), Box<dyn Error + '_>> {
        self.remove_all_containers(vec![id.clone()])?;
        Ok(())
    }

    /// Remove all containers matching the ids in the list. This will kill the
    /// container and slirp4netns instances as a side effect.
    pub fn remove_all_containers(&mut self, ids: Vec<String>) -> Result<(), Box<dyn Error + '_>> {
        for id in ids {
            let container = self.id_map.remove(&id);
            if let Some(container) = container {
                self.pid_id_map.remove(&container.pid);
                // // TODO: Wait and SIGKILL the container as needed
                // kill(container.pid, signal::SIGTERM)?;
                kill(container.slirp_pid, signal::SIGTERM)?;
            }
        }
        Ok(())
    }

    /// Returns a list of all currently-running containers. This is guaranteed
    /// to never contain state of currently-stopped containers.
    pub fn running_containers(&self) -> Vec<libsquish::RunningContainer> {
        let mut out = vec![];
        for v in self.id_map.values() {
            out.push(v.into());
        }
        out
    }
}

/// A background task for reaping dead containers. This checks the global state
/// 10 times per second, removing all container pids that no longer exist.
pub async fn reap_children(state: Arc<Mutex<ContainerState>>) {
    loop {
        sleep(Duration::from_millis(100)).await;
        let mut container_state = state.lock().unwrap();

        // This is dumb, but it SHOULD be dropped at the end of the scope so it
        // SHOULD be fine?
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

fn cleanup_container<'a>(
    state: &'a mut ContainerState,
    id: &'a String,
) -> Result<(), Box<dyn Error + 'a>> {
    state.remove_container(id)?;
    fs::remove_dir_all(path_to(id))?;
    fs::remove_file(format!("/tmp/slirp4netns-{}.sock", id))?;
    Ok(())
}

pub fn path_to(id: &String) -> String {
    format!("container/{}", id)
}
