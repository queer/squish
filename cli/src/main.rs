extern crate hyper;
extern crate hyperlocal;
extern crate reqwest;
extern crate tokio;

use std::{error::Error, vec};

// use hyper::{Body, Client};
use hyper::{Client, body::HttpBody};
use hyperlocal::{UnixClientExt, Uri};
// use tokio::io::{self, AsyncWriteExt as _};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // let echo_json = reqwest::Client::new()
    //     .get("unix:///tmp/squishd.sock/containers/list")
    //     .send()
    //     .await?
    //     .text()
    //     .await?;
    // println!("{:#?}", echo_json);

    let url = Uri::new("/tmp/squishd.sock", "/containers/list").into();
    let client = Client::unix();
    let mut response = client.get(url).await?;
    let mut body: Vec<u8> = vec![];
    println!("reading body");
    while let Some(next) = response.data().await {
        let chunk = next?;
        let bytes: Vec<u8> = chunk.to_vec();
        body.extend(&bytes);
        println!("read {} bytes", bytes.len());
        // io::stdout().write_all(&chunk).await?;
    }
    println!("got value: {}", std::str::from_utf8(body.as_slice()).unwrap());

    Ok(())
}
