use std::{error::Error, fs};

use warp::Filter;

mod handlers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    fs::create_dir_all("./images")?;
    let log = warp::log("registry");

    // GET /image/:group/:name/:tag
    let get_image = warp::path!("image" / String / String / String)
        .and(warp::get())
        .and_then(handlers::image::get_image);

    // TODO: PUT /image/:group/:name/:tag (multipart upload)

    let routes = warp::any().and(get_image).with(log);

    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;

    Ok(())
}
