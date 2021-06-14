extern crate bincode;
extern crate clap;
extern crate futures;
extern crate nix;
extern crate rand;
extern crate reqwest;
extern crate rlimit;
extern crate rtnetlink;
extern crate tokio;

mod engine;

use std::error::Error;

use clap::{App, Arg};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
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
        .get_matches();

    let pid = spawn_container(
        matches.value_of("rootfs").unwrap().to_string(),
        matches.value_of("id").unwrap().to_string(),
    )?;
    println!("{}", pid.as_raw());
    Ok(())
}

fn spawn_container(rootfs: String, container_id: String) -> Result<nix::unistd::Pid, nix::Error> {
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

    let callback = move || {
        match engine::setup_container(&rootfs, &container_id) {
            Ok(_) => 0,
            _ => 1,
        }
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
        // TODO: Better way?
        Some(Signal::SIGCHLD as i32),
    )
    .unwrap();
    if (pid.as_raw() as i32) == -1 {
        println!("clone error");
        println!("{:?}", std::io::Error::last_os_error());
    }

    Ok(pid)
}
