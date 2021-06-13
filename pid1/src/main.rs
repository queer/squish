extern crate clap;
extern crate nix;

use std::fs;
use std::io::Error;
use std::os::unix::io::IntoRawFd;
use std::process;

use clap::{App, Arg};
use nix::mount::{mount, MsFlags};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::unistd::{chdir, chroot};
use nix::unistd::{close, dup, dup2};
use rlimit::Resource;

fn main() -> Result<(), nix::Error> {
    let matches = App::new("pid1")
        .arg(
            Arg::new("rootfs")
                .long("rootfs")
                .takes_value(true)
                .required(true)
                .about(""),
        )
        .arg(
            Arg::new("id")
                .long("id")
                .takes_value(true)
                .required(true)
                .about(""),
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
        // TODO: lol error checking
        let container_path = format!("container/{}/rootfs", container_id);
        fs::create_dir_all(&container_path).expect("couldn't create rootfs folder!");

        // redirect stdout/err
        let stdout_dup = dup(1).unwrap();
        let stderr_dup = dup(2).unwrap();
        close(1).unwrap();
        close(2).unwrap();

        let logfile = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(format!("container/{}/output.log", container_id))
            .unwrap();
        let log_fd = logfile.into_raw_fd();
        // TODO: Lol buffering
        dup2(log_fd, stdout_dup).unwrap();
        dup2(log_fd, stderr_dup).unwrap();
        close(stdout_dup).unwrap();
        close(stderr_dup).unwrap();

        // TODO: User-defined bindmounts
        mount(
            Some(rootfs.as_str()),
            container_path.clone().as_str(),
            Some(""),
            MsFlags::MS_BIND | MsFlags::MS_RDONLY,
            Some(""),
        )
        .expect("couldn't mount rootfs");
        chroot(container_path.as_str()).expect("couldn't chroot!?");
        chdir("/").expect("couldn't chdir to /!?");

        run_in_container();
        println!(">> done!");
        0
    };

    let mut stack_vec = vec![0u8; stack_size];
    let stack: &mut [u8] = stack_vec.as_mut_slice();

    let pid = clone(
        Box::new(callback),
        stack,
        CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWUTS
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWUSER,
        // TODO: Better way?
        Some(Signal::SIGCHLD as i32),
    )
    .unwrap();
    if (pid.as_raw() as i32) == -1 {
        println!("clone error");
        println!("{:?}", Error::last_os_error());
    }

    Ok(pid)
}

fn run_in_container() {
    println!(">> inside the container!");
    println!(">> i am {}", process::id());

    if let Ok(paths) = fs::read_dir("/") {
        println!(">> my rootfs has:");
        for path in paths {
            println!(">>    {}", path.unwrap().path().display());
        }
    } else {
        println!(">> warning: could not read_dir /");
    }
}
