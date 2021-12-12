extern crate clap;
extern crate futures;
extern crate nix;
extern crate reqwest;
extern crate rlimit;
extern crate tokio;

mod engine;

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::FromRawFd;

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
                .help("path to rootfs"),
        )
        .arg(
            Arg::new("id")
                .long("id")
                .takes_value(true)
                .required(true)
                .help("container id"),
        )
        .arg(
            Arg::new("path")
                .long("path")
                .takes_value(true)
                .required(true)
                .help("path to container directory"),
        )
        .arg(
            Arg::new("squishfile-memfd")
                .long("squishfile-memfd")
                .takes_value(true)
                .required(true)
                .help("squishfile memfd to run from"),
        )
        .get_matches();

    let squishfile_memfd: i32 = matches
        .value_of("squishfile-memfd")
        .unwrap()
        .to_string()
        .parse()?;
    // Safety: We created this in the daemon, and since this is cloned off of
    //         the daemon process, we know that the fd exists. Since the daemon
    //         disables FD_CLOEXEC before forking, we know that the fd is
    //         guaranteed to exist.
    let mut squishfile_json = unsafe { File::from_raw_fd(squishfile_memfd) };
    let mut squishfile = String::new();
    squishfile_json.read_to_string(&mut squishfile)?;

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
            soft as usize
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
