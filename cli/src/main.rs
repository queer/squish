extern crate hyper;
extern crate hyperlocal;
extern crate tokio;

mod client;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let res = client::get("/containers/list").await?;
    println!("got value: {}", res);

    Ok(())
}
