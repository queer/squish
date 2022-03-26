use std::fs::{self, OpenOptions};
use std::os::unix::io::IntoRawFd;
use std::path::Path;
use std::process;

use libsquish::squishfile::{LayerSpec, Squishfile};
use libsquish::Result;
use nix::mount::{mount, MsFlags};
use nix::unistd::{chdir, chroot, close, dup, dup2};

pub struct Engine<'a> {
    squishfile: &'a Squishfile,
    rootfs_path: &'a str,
    container_path: &'a str,
    #[allow(dead_code)]
    container_id: &'a str,
    container_rootfs_path: String,
}

impl<'a> Engine<'a> {
    pub fn new(
        squishfile: &'a Squishfile,
        rootfs: &'a str,
        container_path: &'a str,
        container_id: &'a str,
    ) -> Self {
        Engine {
            squishfile,
            rootfs_path: rootfs,
            container_path,
            container_id,
            container_rootfs_path: format!("{}/rootfs", container_path),
        }
    }

    pub fn setup_container(&self) -> Result<&Self> {
        // Set up container rootfs
        fs::create_dir_all(&self.container_rootfs_path).expect("couldn't create rootfs directory!");

        // redirect stdout/err
        let stdout_dup = dup(1)?;
        let stderr_dup = dup(2)?;
        close(1)?;
        close(2)?;

        let stdout_log = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(format!("{}/stdout.log", &self.container_path))?;
        let stdout_log_fd = stdout_log.into_raw_fd();
        let stderr_log = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(format!("{}/stderr.log", &self.container_path))?;
        let stderr_log_fd = stderr_log.into_raw_fd();

        dup2(stdout_log_fd, stdout_dup)?;
        dup2(stderr_log_fd, stderr_dup)?;
        close(stdout_dup)?;
        close(stderr_dup)?;

        // Bindmount rootfs ro
        self.bind_mount(
            self.rootfs_path,
            &self.container_rootfs_path,
            MsFlags::MS_RDONLY | MsFlags::MS_NOATIME | MsFlags::MS_NOSUID,
        )?;

        // Bind-mount *nix stuff in
        println!(">> bindmounting devices");
        self.bind_mount_dev(
            "/dev/null",
            &format!("{}/dev/null", self.container_rootfs_path),
        )?;
        self.bind_mount_dev(
            "/dev/zero",
            &format!("{}/dev/zero", self.container_rootfs_path),
        )?;
        self.bind_mount_dev(
            "/dev/random",
            &format!("{}/dev/random", self.container_rootfs_path),
        )?;
        self.bind_mount_dev(
            "/dev/urandom",
            &format!("{}/dev/urandom", self.container_rootfs_path),
        )?;
        println!(">> bindmounting devices finished!");

        // Bindmount /tmp rw
        let tmp_path = format!("{}/tmp", &self.container_path);
        fs::create_dir_all(&tmp_path)?;
        self.bind_mount(
            &tmp_path,
            &format!("{}/tmp", self.container_rootfs_path),
            MsFlags::MS_NOSUID,
        )?;

        for (layer_name, layer) in self.squishfile.layers() {
            if layer_name != "alpine" && layer_name != "app" {
                self.bind_mount_layer::<&str>(
                    &self.container_rootfs_path,
                    layer_name,
                    layer,
                    None,
                )?;
            } else if layer_name == "app" {
                self.bind_mount_layer(
                    &self.container_rootfs_path,
                    layer_name,
                    layer,
                    Some("/app/"),
                )?;
            }
        }
        Ok(self)
    }

    pub fn run_container(&self) -> Result<()> {
        // chroot!
        chroot(self.container_rootfs_path.as_str()).expect("couldn't chroot!?");
        chdir("/").expect("couldn't chdir to /!?");

        self.run_in_container()?;
        println!(">> done!");
        Ok(())
    }

    fn run_in_container(&self) -> Result<()> {
        println!(">> inside the container!");
        println!(">> i am {}", process::id());
        println!(
            ">> running: {} {:?}",
            self.squishfile.run().command(),
            self.squishfile.run().args()
        );

        std::process::Command::new(self.squishfile.run().command())
            .envs(self.squishfile.env())
            .args(self.squishfile.run().args())
            .output()
            .unwrap();
        Ok(())
    }

    fn bind_mount_layer<TO>(
        &self,
        container_path: &str,
        layer_name: &str,
        layer: &LayerSpec,
        target_override: Option<TO>,
    ) -> Result<()>
    where
        TO: Into<String>,
    {
        // Bind-mount squishfile layers
        println!(">> bindmounting {:?} => {:?}", layer.path(), layer.target());
        if layer.path().is_none() && layer.version().is_none() {
            panic!("squishfile: nothing to mount for layer {}!?", layer_name);
        }
        let target = match layer.target() {
            Some(target) => target.clone(),
            None => {
                if layer.path().is_some() {
                    // If path but no target, mount into /app
                    let target = layer
                        .path()
                        .as_ref()
                        .unwrap()
                        .replace("../", "")
                        .replace("./", "");
                    format!("/app/{}", target)
                } else if layer.version().is_none() {
                    // If no path and no target and no version, panic
                    panic!("squishfile no path or version for layer {}", layer_name);
                } else {
                    // If no path or target, but there is a version, mount into /sdk
                    format!("/sdk/{}", layer_name)
                }
            }
        };
        let target = if let Some(target_override) = target_override {
            let file_name = Path::new(&target)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            format!("{}/{}", target_override.into(), file_name)
        } else {
            target
        };
        let target = format!("{}/{}", container_path, target);
        // if layer.path().is_none() && layer.version().is_some() && layer.target().is_none() {
        //     todo!("mounting squish layer normally");
        // }
        let path = layer.path().as_ref().unwrap();
        let mount_path = Path::new(path);
        // Yeah this is technically racy, but literally who cares?
        if mount_path.exists() {
            let meta = fs::metadata(path)?;
            let target_path = Path::new(&target);
            if meta.is_dir() {
                self.touch_dir(target_path)?;
            } else if meta.is_file() {
                let parent = target_path.parent().unwrap();
                self.touch_dir(parent)?;
                self.touch(target_path)?;
            } else {
                println!(">> mount is not a directory or file");
            }
            let mut bind_flags = MsFlags::MS_NOATIME | MsFlags::MS_NOSUID;
            if !matches!(layer.rw(), Some(true)) {
                bind_flags |= MsFlags::MS_RDONLY;
            }
            self.bind_mount(path, &target, bind_flags)?;
        } else {
            println!(">> mount didn't exist");
        }
        Ok(())
    }

    fn bind_mount_dev(&self, dev: &'static str, target: &str) -> Result<()> {
        println!(">> bindmount dev {} -> {}", dev, target);
        mount(Some(dev), target, Some(""), MsFlags::MS_BIND, Some(""))?;
        Ok(())
    }

    fn bind_mount(&self, src: &str, target: &str, flags: MsFlags) -> Result<()> {
        println!(">> bindmount {} -> {}", src, target);

        mount(
            Some(src),
            target,
            Some(""),
            MsFlags::MS_BIND | flags,
            Some(""),
        )?;
        Ok(())
    }

    fn touch(&self, path: &Path) -> Result<()> {
        match OpenOptions::new().create(true).write(true).open(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn touch_dir(&self, path: &Path) -> Result<()> {
        match fs::create_dir_all(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }
}
