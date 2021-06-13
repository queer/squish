use crate::engine;
use crate::engine::containers::ContainerState;
use crate::util::SquishError;

use std::sync::Arc;
use std::sync::Mutex;

use warp::Rejection;

pub async fn create_container(state: Arc<Mutex<ContainerState>>) -> Result<impl warp::Reply, Rejection> {
    info!("spawning container");
    let pid = engine::spawn_container().map_err(|e| SquishError::NixError(e))?;
    let mut container_state = state.lock().unwrap();
    container_state.add_container(pid).unwrap();
    let res = "{\"status\":\"ok\"}".to_string();
    Ok(warp::reply::json(&res))
}

pub async fn list_containers(state: Arc<Mutex<ContainerState>>) -> Result<impl warp::Reply, Rejection> {
    info!("listing containers");
    let container_state = state.lock().unwrap();
    dbg!(container_state);
    let res = "{\"status\":\"ok\"}".to_string();
    Ok(warp::reply::json(&res))
}
