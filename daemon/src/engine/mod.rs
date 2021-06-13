pub mod alpine;

use crate::util;

use std::fs;
use std::io::Error;
use std::process;
use std::thread::spawn;

use nix::mount::{mount, MsFlags};
use nix::sched::{clone, CloneFlags};
use nix::unistd::{chdir, chroot};

const STACK_SIZE: usize = 1024 * 1024;

pub fn spawn_container() -> Result<(), nix::Error> {
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
        ).expect("couldn't mount rootfs");
        chroot("container/rootfs").expect("couldn't chroot!?");
        chdir("/").expect("couldn't chdir to /!?");

        println!(">> container exec at {}", util::now_ms());
        run_in_container();
        println!(">> done!");
        0
    };

    spawn(move || {
        let ref mut stack: [u8; STACK_SIZE] = [0; STACK_SIZE];

        let pid = clone(
            Box::new(callback),
            stack,
            CloneFlags::CLONE_NEWPID
                | CloneFlags::CLONE_NEWUTS
                | CloneFlags::CLONE_NEWNS
                | CloneFlags::CLONE_NEWUSER,
            None,
        )
        .unwrap();
        if (pid.as_raw() as i32) == -1 {
            println!("clone error");
            println!("{:?}", Error::last_os_error());
        }
        println!("forked into {}", pid);
    });

    Ok(())
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
