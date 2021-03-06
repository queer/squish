#![warn(clippy::needless_pass_by_value)]

extern crate hyper;
extern crate hyperlocal;
extern crate serde_json;
extern crate tokio;

mod client;

use std::cmp::max;

use clap::{Arg, Command};
use libsquish::squishfile;
use libsquish::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("squish")
        .arg(Arg::new("debug").long("debug").short('d').help(""))
        .subcommand(Command::new("ps").about("List running containers"))
        .subcommand(
            Command::new("create")
                .about("Create new containers")
                .arg(Arg::new("squishfile").required(true)),
        )
        .subcommand(
            Command::new("validate")
                .about("Validate a squishfile")
                .arg(Arg::new("squishfile").required(true)),
        )
        .subcommand(
            Command::new("stop")
                .about("Stop a container")
                .arg(Arg::new("id").required(true)),
        )
        .get_matches();

    match matches.subcommand_name() {
        Some("ps") => {
            let res = client::get("/containers/list").await?;
            let value: Vec<libsquish::RunningContainer> =
                serde_json::from_str(res.as_str()).unwrap();

            let mut max_name = 0;
            for container in &value {
                max_name = max(container.name.len(), max_name);
            }
            println!(
                "{:id_width$} {:name_width$} PID",
                "ID",
                "NAME",
                id_width = 7,
                name_width = max_name
            );
            for container in &value {
                println!(
                    "{} {:name_width$} {}",
                    &container.id[..7],
                    container.name,
                    container.pid,
                    name_width = max_name
                );
            }
        }
        Some("create") => {
            // safe
            let path = matches
                .subcommand_matches("create")
                .ok_or("impossible")?
                .value_of("squishfile")
                .ok_or("impossible")?;
            let mut squishfile = squishfile::parse(path)?;
            squishfile.resolve_paths();

            // Send to daemon
            let res = client::post(
                "/containers/create",
                Some(serde_json::to_string(&squishfile)?),
            )
            .await?;
            let ids: serde_json::Value = serde_json::from_str(res.as_str())?;
            for id in ids.as_array().unwrap() {
                println!("{}", id.as_str().unwrap());
            }
        }
        Some("stop") => {
            // safe
            let container_id = matches
                .subcommand_matches("stop")
                .ok_or("impossible")?
                .value_of("id")
                .ok_or("impossible")?;

            // Send to daemon
            let res =
                client::post::<String, String>(format!("/containers/stop/{}", container_id), None)
                    .await?;
            let ids = serde_json::from_str(res.as_str())?;
            match ids {
                serde_json::Value::Array(ids) => {
                    for id in ids {
                        println!("{}", id.as_str().unwrap());
                    }
                }
                _ => eprintln!("got unknown value: {}", res),
            }
        }
        Some("validate") => {
            // safe
            let path = matches
                .subcommand_matches("validate")
                .ok_or("impossible")?
                .value_of("squishfile")
                .ok_or("impossible")?;
            let _squishfile = squishfile::parse(path)?;
            println!("ok");
        }
        Some(cmd) => {
            println!("Unknown subcommand '{}'", cmd);
        }
        None => {
            println!("No subcommand provided :<");
        }
    }

    Ok(())
}
