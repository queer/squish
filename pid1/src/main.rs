extern crate clap;
extern crate futures;
extern crate nix;
extern crate reqwest;
extern crate rlimit;
extern crate tokio;

mod engine;

use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;

use clap::{App, Arg};
use libsquish::squishfile::Squishfile;
use nix::sched::{clone, CloneFlags};
use rlimit::Resource;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("pid1")
        .arg(
            Arg::new("rootfs")
                .long("rootfs")
                .takes_value(true)
                .required(true)
                .about("path to rootfs"),
        )
        .arg(
            Arg::new("id")
                .long("id")
                .takes_value(true)
                .required(true)
                .about("container id"),
        )
        .arg(
            Arg::new("path")
                .long("path")
                .takes_value(true)
                .required(true)
                .about("path to container directory"),
        )
        .arg(
            Arg::new("squishfile")
                .long("squishfile")
                .takes_value(true)
                .required(true)
                .about("squishfile to run"),
        )
        .get_matches();

    let squishfile_path = matches.value_of("squishfile").unwrap().to_string();
    let mut squishfile_json = File::open(&squishfile_path)?;
    let mut squishfile = String::new();
    squishfile_json.read_to_string(&mut squishfile)?;
    fs::remove_file(&squishfile_path)?;

    let pid = spawn_container(
        matches.value_of("rootfs").unwrap().to_string(),
        matches.value_of("path").unwrap().to_string(),
        matches.value_of("id").unwrap().to_string(),
        Squishfile::from_json(squishfile.as_str())
            .expect("impossible (couldn't deser squishfile)!?"),
    )?;
    println!("{}", pid.as_raw());
    Ok(())
}

fn spawn_container(
    rootfs: String,
    path: String,
    container_id: String,
    squishfile: Squishfile,
) -> Result<nix::unistd::Pid, Box<dyn Error>> {
    let stack_size = match Resource::STACK.get() {
        Ok((soft, _hard)) => {
            // debug!(
            //     "soft stack={}, hard stack={}",
            //     soft.as_usize(),
            //     hard.as_usize()
            // );
            soft.as_usize()
        }
        Err(_) => {
            // 8MB
            8 * 1024 * 1024
        }
    };

    let callback =
        move || match engine::setup_and_run_container(&rootfs, &path, &container_id, &squishfile) {
            Ok(_) => 0,
            _ => 1,
        };

    let mut stack_vec = vec![0u8; stack_size];
    let stack: &mut [u8] = stack_vec.as_mut_slice();

    let pid = clone(
        Box::new(callback),
        stack,
        CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWUTS
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWNET
            | CloneFlags::CLONE_NEWUSER,
        None,
    )
    .unwrap();
    if (pid.as_raw() as i32) == -1 {
        println!("clone error");
        println!("{:?}", std::io::Error::last_os_error());
    }

    Ok(pid)
}
