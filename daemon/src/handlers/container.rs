use crate::engine;
use crate::engine::containers::ContainerState;
use crate::util::SquishError;

use std::sync::Arc;
use std::sync::Mutex;

use warp::Rejection;

pub async fn create_container(state: Arc<Mutex<ContainerState>>) -> Result<impl warp::Reply, Rejection> {
    let mut container_state = state.lock().unwrap();
    let (id, name) = ContainerState::generate_id();
    info!("spawning container {} ({})", name, id);
    let pid = engine::spawn_container(id.clone()).map_err(|e| SquishError::GenericError(e))?;
    container_state.add_container(pid, id, name).unwrap();
    let res = "{\"status\":\"ok\"}".to_string();
    Ok(warp::reply::json(&res))
}

pub async fn list_containers(state: Arc<Mutex<ContainerState>>) -> Result<impl warp::Reply, Rejection> {
    info!("listing containers");
    let container_state = state.lock().unwrap();
    let running_containers = container_state.running_containers();
    Ok(warp::reply::json(&running_containers))
}
