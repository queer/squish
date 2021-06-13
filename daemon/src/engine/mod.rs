pub mod alpine;
pub mod containers;

use crate::util;

use std::fs;
use std::io::Error;
use std::process;
use std::sync::mpsc::channel;
use std::thread::spawn;

use nix::mount::{mount, MsFlags};
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use nix::unistd::{chdir, chroot};
use rlimit::Resource;

pub fn spawn_container() -> Result<nix::unistd::Pid, nix::Error> {
    let stack_size = match Resource::STACK.get() {
        Ok((soft, hard)) => {
            debug!(
                "soft stack={}, hard stack={}",
                soft.as_usize(),
                hard.as_usize()
            );
            soft.as_usize()
        }
        Err(_) => {
            // 8MB
            8 * 1024 * 1024
        }
    };

    // TODO: This should probably re-exec /proc/self/exe instead of just immediately cloning
    println!("boot at {}", util::now_ms());
    let callback = || {
        println!(">> re-exec as container...");

        fs::create_dir_all("container/rootfs").expect("couldn't create rootfs folder!");

        // TODO: User-defined bindmounts
        mount(
            Some(alpine::current_rootfs().as_str()),
            "container/rootfs",
            Some(""),
            MsFlags::MS_BIND | MsFlags::MS_RDONLY,
            Some(""),
        )
        .expect("couldn't mount rootfs");
        chroot("container/rootfs").expect("couldn't chroot!?");
        chdir("/").expect("couldn't chdir to /!?");

        println!(">> container exec at {}", util::now_ms());
        run_in_container();
        println!(">> done!");
        0
    };

    let (tx, rx) = channel();

    spawn(move || {
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
        println!("forked into {}", pid);
        tx.send(pid).unwrap();
    });

    Ok(rx.recv().unwrap())
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
