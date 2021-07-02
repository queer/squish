use std::path::Path;

use warp::Rejection;

pub async fn get_image(
    group: String,
    name: String,
    tag: String,
) -> Result<impl warp::Reply, Rejection> {
    let exists = Path::new(&format!("images/{}/{}/{}", group, name, tag)).exists();
    if !exists {
        return Err(warp::reject::not_found());
    }
    Ok(warp::reply::json(&""))
}
