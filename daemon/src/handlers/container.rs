use crate::engine;
use crate::engine::containers::ContainerState;
use crate::util::SquishError;

use std::sync::Arc;
use std::sync::Mutex;

use libsquish::squishfile::Squishfile;
use warp::Rejection;

pub async fn create_container(
    state: Arc<Mutex<ContainerState>>,
    squishfile: Squishfile,
) -> Result<impl warp::Reply, Rejection> {
    let (id, name) = ContainerState::generate_id();
    info!("spawning container {} ({})", name, id);
    let (container_pid, slirp_pid) = engine::spawn_container(&id, squishfile)
        .await
        .map_err(|e| SquishError::GenericError(e))?;
    info!(
        "spawned container {} in pid {} (slirp={})",
        name,
        container_pid.as_raw(),
        slirp_pid.as_raw(),
    );

    // Minimise use so as to avoid lock contention
    let mut container_state = state.lock().unwrap();
    container_state
        .add_container(container_pid, slirp_pid, id, name)
        .unwrap();
    let res = "{\"status\":\"ok\"}".to_string();
    Ok(warp::reply::json(&res))
}

pub async fn list_containers(
    state: Arc<Mutex<ContainerState>>,
) -> Result<impl warp::Reply, Rejection> {
    info!("listing containers");
    let container_state = state.lock().unwrap();
    let running_containers = container_state.running_containers();
    Ok(warp::reply::json(&running_containers))
}

pub async fn stop_container(id: String, state: Arc<Mutex<ContainerState>>) -> Result<impl warp::Reply, Rejection> {
    let mut container_state = state.lock().unwrap();
    let ids = container_state.fuzzy_remove_container(&id).unwrap();
    Ok(warp::reply::json(&ids))
}
