#[macro_use]
extern crate log;
extern crate nix;
extern crate pretty_env_logger;
extern crate tokio;
extern crate warp;

use std::fs;
use std::path::Path;

use warp::Filter;

mod engine;
mod handlers;
mod util;

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::UnixListener;
    use tokio_stream::wrappers::UnixListenerStream;

    pretty_env_logger::init();
    info!("squishd booting...");

    let path = Path::new("/tmp/squishd.sock");

    if path.exists() {
        fs::remove_file(path)?;
    }

    // spawn_container()?;

    let container_create = warp::path!("containers" / "create")
        .and(warp::post())
        .and_then(handlers::container::create_container);
    let container_list = warp::path!("containers" / "list")
        .and(warp::get())
        .and_then(handlers::container::list_containers);

    let routes = warp::get().and(
        container_create
            .or(container_list)
    );

    let listener = UnixListener::bind(path).unwrap();
    let incoming = UnixListenerStream::new(listener);
    warp::serve(routes)
        .run_incoming(incoming)
        .await;

    Ok(())
}

#[cfg(not(unix))]
#[tokio::main]
async fn main() {
    panic!("Must run under Unix-like platform!");
}
