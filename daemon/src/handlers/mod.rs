use warp::Rejection;

pub mod container;

pub async fn status() -> Result<impl warp::Reply, Rejection> {
    Ok(warp::reply::reply())
}
