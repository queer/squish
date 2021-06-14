extern crate hyper;
extern crate hyperlocal;
extern crate tokio;

mod client;

use std::error::Error;

use clap::{App, Arg};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let matches = App::new("squish")
        .arg(
            Arg::new("debug")
                .long("debug")
                .short('d')
                .about(""),
        )
        .subcommand(App::new("list"))
        .subcommand(App::new("create"))
        .get_matches();

    match matches.subcommand_name() {
        Some("ps") => {
            let res = client::get("/containers/list").await?;
            println!("got value: {}", res);
        },
        Some("create") => {
            let res = client::post("/containers/create").await?;
            println!("got value: {}", res);
        },
        Some(cmd) => {
            println!("Unknown subcommand '{}'", cmd);
        },
        None => {
            println!("No subcommand provided :<");
        },
    }

    Ok(())
}
