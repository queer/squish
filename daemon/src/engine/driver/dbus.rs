use super::cgroup;
use crate::util::Result;

use std::sync::Arc;
use std::time::Duration;

use dbus::arg::{AppendAll, ReadAll, RefArg, Variant};
use dbus::nonblock::{Proxy, SyncConnection};

pub struct DbusDriver {
    conn: Arc<SyncConnection>,
}

impl DbusDriver {
    pub async fn new() -> Result<Self> {
        use dbus_tokio::connection;

        let (resource, conn) = connection::new_session_sync()?;

        let _handle = tokio::spawn(async {
            let err = resource.await;
            panic!("Lost connection to D-Bus: {}", err);
        });

        Ok(Self { conn })
    }

    async fn method_call<'a, S, A, R>(
        &self,
        dest: S,
        path: S,
        interface: S,
        method: S,
        args: A,
    ) -> Result<R>
    where
        S: Into<&'a str>,
        A: AppendAll,
        R: ReadAll + 'static,
    {
        let proxy = Proxy::new(
            dest.into(),
            path.into(),
            Duration::from_secs(2),
            self.conn.clone(),
        );
        let result = proxy
            .method_call(interface.into(), method.into(), args)
            .await?;
        Ok(result)
    }
}

#[async_trait::async_trait]
trait SystemdDbusDriver<IS: Into<String> + Send + Sync + 'static> {
    async fn get_unit(&self, name: IS) -> Result<String>;
    async fn start_transient_unit(
        &self,
        name: IS,
        description: IS,
        pids: Vec<i32>,
    ) -> Result<String>;
    async fn stop_unit(&self, name: IS, mode: IS) -> Result<String>;
}

#[async_trait::async_trait]
impl<IS: Into<String> + Send + Sync + 'static> SystemdDbusDriver<IS> for DbusDriver {
    async fn get_unit(&self, name: IS) -> Result<String> {
        let dbus_path: (dbus::Path<'static>,) = self
            .method_call(
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
                "GetUnit",
                (name.into(),),
            )
            .await?;
        Ok(dbus_path.0.to_string())
    }

    async fn start_transient_unit(
        &self,
        name: IS,
        description: IS,
        pids: Vec<i32>,
    ) -> Result<String> {
        let properties: Vec<(&str, Variant<Box<dyn RefArg>>)> = vec![
            (
                "Slice",
                Variant(Box::new(cgroup::detect_current_cgroup_slice_name().await?)),
            ),
            ("Delegate", Variant(Box::new(true))),
            ("PIDs", Variant(Box::new(pids.clone()))),
            ("Description", Variant(Box::new(description.into()))),
        ];
        #[allow(clippy::type_complexity)]
        let aux: Vec<(&str, Vec<(&str, Variant<Box<dyn RefArg>>)>)> = vec![];
        let dbus_path: (dbus::Path<'static>,) = self
            .method_call(
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
                "StartTransientUnit",
                (name.into(), "replace", properties, aux),
            )
            .await?;
        Ok(dbus_path.0.to_string())
    }

    async fn stop_unit(&self, name: IS, mode: IS) -> Result<String> {
        let dbus_path: (dbus::Path<'static>,) = self
            .method_call(
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
                "StopUnit",
                (name.into(), mode.into()),
            )
            .await?;
        Ok(dbus_path.0.to_string())
    }
}

#[cfg(test)]
mod test {
    use super::{DbusDriver, SystemdDbusDriver};
    use crate::util::Result;

    const UNIT: &str = "squish-test.scope";

    #[tokio::test]
    pub async fn test_transient_unit_functionality() -> Result<()> {
        let driver = DbusDriver::new().await?;

        let path = driver.get_unit(UNIT).await;
        assert!(path.is_err());
        dbg!(&path);

        let path = driver
            .start_transient_unit(UNIT, "gay", vec![nix::unistd::getpid().as_raw()])
            .await;
        dbg!(&path);
        assert!(path.is_ok());
        let path = path?;
        dbg!(&path);
        // assert_eq!("", path);

        let path = driver.get_unit(UNIT).await;
        assert!(path.is_ok());
        let path = path?;
        dbg!(&path);

        driver.stop_unit(UNIT, "replace").await?;
        Ok(())
    }
}
