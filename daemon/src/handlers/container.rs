use warp::Rejection;

use crate::engine;
use crate::util::SquishError;

pub async fn create_container() -> Result<impl warp::Reply, Rejection> {
    engine::spawn_container().map_err(|e| SquishError::NixError(e))?;
    let res = "{\"status\":\"ok\"}".to_string();
    Ok(warp::reply::json(&res))
}

pub async fn list_containers() -> Result<impl warp::Reply, Rejection> {
    info!("listing containers");
    let res = "{\"status\":\"ok\"}".to_string();
    Ok(warp::reply::json(&res))
}
