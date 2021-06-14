extern crate hyper;
extern crate hyperlocal;
extern crate serde_json;
extern crate tokio;

mod client;

use std::cmp::max;
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
        .subcommand(App::new("ps"))
        .subcommand(App::new("create"))
        .get_matches();

    match matches.subcommand_name() {
        Some("ps") => {
            let res = client::get("/containers/list").await?;
            let value: Vec<libsquish::RunningContainer> = serde_json::from_str(res.as_str()).unwrap();

            let mut max_name = 0;
            for container in &value {
                max_name = max(container.name.len(), max_name);
            }
            println!("{:id_width$} {:name_width$} {}", "ID", "NAME", "PID", id_width=7, name_width=max_name);
            for container in &value {
                println!("{} {:name_width$} {}", &container.id[..7], container.name, container.pid, name_width=max_name);
            }
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
