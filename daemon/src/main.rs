extern crate flate2;
extern crate haikunator;
extern crate hex;
extern crate hmac_sha256;
extern crate libsquish;
#[macro_use]
extern crate log;
extern crate nix;
extern crate pretty_env_logger;
extern crate reqwest;
extern crate tar;
extern crate tokio;
extern crate warp;
extern crate yaml_rust;

use crate::engine::containers::ContainerState;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use libsquish::squishfile;
use warp::hyper::body::Bytes;
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

    info!("prefetching alpine base image...");
    engine::alpine::download_base_image().await?;
    info!("prefetching slirp4netns binary...");
    engine::slirp::download_slirp4netns().await?;

    let path = Path::new("/tmp/squishd.sock");
    if path.exists() {
        fs::remove_file(path)?;
    }

    let global_state = Arc::new(Mutex::new(ContainerState::new()));

    let clone = global_state.clone();
    tokio::spawn(engine::containers::reap_children(clone));

    let container_create = warp::path!("containers" / "create")
        .and(warp::post())
        .and(with_state(global_state.clone()))
        .and(warp::body::bytes().map(|bytes: Bytes| {
            let vec: Vec<u8> = bytes.to_vec();
            // TODO: Better error handling(?)
            let body = String::from_utf8(vec).expect("squishfile not valid string");
            squishfile::parse_str(&*body).expect("squishfile invalid")
        }))
        .and_then(handlers::container::create_container);
    let container_list = warp::path!("containers" / "list")
        .and(warp::get())
        .and(with_state(global_state.clone()))
        .and_then(handlers::container::list_containers);

    let log = warp::log("squishd");
    let routes = warp::any()
        .and(container_create.or(container_list))
        .with(log);

    let listener = UnixListener::bind(path).unwrap();
    let incoming = UnixListenerStream::new(listener);
    warp::serve(routes).run_incoming(incoming).await;

    Ok(())
}

#[cfg(not(unix))]
#[tokio::main]
async fn main() {
    panic!("squishd must be run on a unix-like os!");
}

fn with_state<T: Clone + Send + Sync>(
    state: T,
) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}
