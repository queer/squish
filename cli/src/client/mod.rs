use std::error::Error;
use std::vec;

use hyper::{body::HttpBody, Client};
use hyperlocal::{UnixClientExt, Uri};

pub async fn get(route: &'static str) -> Result<String, Box<dyn Error + Send + Sync>> {
    // TODO: Error-handling
    let url = Uri::new("/tmp/squishd.sock", route).into();
    let client = Client::unix();
    let mut response = client.get(url).await?;
    let mut body: Vec<u8> = vec![];
    while let Some(next) = response.data().await {
        let chunk = next?;
        let bytes: Vec<u8> = chunk.to_vec();
        body.extend(&bytes);
        // io::stdout().write_all(&chunk).await?;
    }
    // TODO: This should never panic, but verify anyway
    Ok(String::from_utf8(body).unwrap())
}
