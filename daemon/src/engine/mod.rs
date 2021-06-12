use std::fs;
use std::io::Error;
use std::process;

use nix::sched::{clone, CloneFlags};
// use nix::mount::{mount, MsFlags};
use nix::unistd::{chdir, chroot};

const STACK_SIZE: usize = 4 * 1024 * 1024;

pub fn spawn_container() -> Result<(), nix::Error> {
    let ref mut stack: [u8; STACK_SIZE] = [0; STACK_SIZE];

    let callback = || {
        println!(">> re-exec as container...");

        fs::create_dir_all("container/oldrootfs").expect("couldn't create oldrootfs folder!");
        fs::create_dir_all("container/newrootfs").expect("couldn't create newrootfs folder!");

        // Once we're inside the container, mount a new rootfs
        // TODO: Bind mounts
        // mount(Some("container"), "container", Some(""), MsFlags::MS_BIND, Some("")).expect("couldn't mount rootfs");
        chroot("container/newrootfs").expect("couldn't pivot root!?");
        chdir("/").expect("couldn't chdir to /!?");

        run_in_container();
        println!(">> done!");
        0
    };

    let pid = clone(
        Box::new(callback),
        stack,
        CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWUTS
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWUSER,
        None,
    )?;
    if (pid.as_raw() as i32) == -1 {
        println!("clone error");
        println!("{:?}", Error::last_os_error());
    }
    println!("forked into {}", pid);
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
