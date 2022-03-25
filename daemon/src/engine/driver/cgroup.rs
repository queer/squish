use crate::util::{Result, SquishError};

use std::path::Path;

use async_recursion::async_recursion;
use tokio::fs;

/// Detects the current cgroup. This is done by reading `/proc/self/cgroup` and
/// parsing out the cgroup's name.
///
/// The cgroup name we get out is formatted as such:
///
/// ```
/// hierarchy-ID:controller-list:cgroup-path
/// ```
///
/// For example:
///
/// ```
/// 5:cpuacct,cpu,cpuset:/daemons
/// ```
///
/// For more information, see [here](https://man7.org/linux/man-pages/man7/cgroups.7.html)
/// and search for "hierarchy-ID:controller-list:cgroup-path".
pub async fn detect_current_cgroup() -> Result<String> {
    let cgroup_info = fs::read_to_string("/proc/self/cgroup").await?;
    let cgroup: Vec<&str> = cgroup_info.trim().split(':').collect();
    Ok(cgroup[2].to_string())
}

/// The path to the current cgroup on the filesystem.
pub async fn current_cgroup_path() -> Result<String> {
    Ok(format!("/sys/fs/cgroup{}", detect_current_cgroup().await?))
}

/// Detect the current cgroup slice name, if possible.
pub async fn detect_current_cgroup_slice() -> Result<String> {
    let cgroup = detect_current_cgroup().await?;
    Ok(detect_current_cgroup_cgroup_slice_recursive(&cgroup)?)
}

fn detect_current_cgroup_cgroup_slice_recursive(cgroup: &String) -> Result<String> {
    // TODO: Rewrite this to use Path
    let mut iter = cgroup.split('/').rev();
    if let Some(item) = iter.next() {
        let mut parts: Vec<String> = vec![];
        if item.ends_with(".slice") {
            parts.push(item.to_string());
            for item in &mut iter {
                parts.push(item.to_string());
            }
            parts.reverse();
            Ok(parts.join("/"))
        } else {
            for item in &mut iter {
                parts.push(item.to_string());
            }
            parts.reverse();
            detect_current_cgroup_cgroup_slice_recursive(&parts.join("/"))
        }
    } else {
        Err(Box::new(SquishError::CgroupNoMoreSlices))
    }
}

pub async fn detect_current_cgroup_slice_name() -> Result<String> {
    let cgroup = detect_current_cgroup_slice().await?;
    let part = cgroup.split('/').rev().next();
    match part {
        Some(part) => Ok(part.to_string()),
        None => Err(Box::new(SquishError::CgroupNoMoreSlices)),
    }
}

/// The different types of cgroup controller. Some require privileges.
/// See also: https://wiki.archlinux.org/title/Cgroups#Controller_types
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub enum Controller {
    Cpu,
    Cpuset,
    Freezer,
    Hugetlb,
    Io,
    Memory,
    PerfEvent,
    Pids,
    Rdma,
}

/// Detect all cgroup controller delegations for the current cgroup. We get the
/// current cgroup, then read delegations out of
/// `/sys/fs/cgroup/[cgroup]/cgroup.controllers`. If this file doesn't exist,
/// we recurse up the directory tree until we find valid cgroup controllers or
/// we've run out of the cgroup sysfs.
pub async fn detect_cgroup_controller_delegations() -> Result<Vec<Controller>> {
    detect_cgroup_controller_delegations_recursive(current_cgroup_path().await?).await
}

#[async_recursion]
async fn detect_cgroup_controller_delegations_recursive(path: String) -> Result<Vec<Controller>> {
    let path = format!("{}/cgroup.controllers", path);
    let exists = Path::new(&path).exists();
    if exists {
        let delegations = fs::read_to_string(path).await?;
        parse_cgroup_controller_delegations(delegations)
    } else {
        let mut split: Vec<String> = path.split('/').map(|s| s.to_string()).collect();
        split.truncate(split.len() - 1);
        detect_cgroup_controller_delegations_recursive(split.join("/")).await
    }
}

fn parse_cgroup_controller_delegations<T: Into<String>>(delegations: T) -> Result<Vec<Controller>> {
    let delegations: String = delegations.into();
    let delegations: Vec<Controller> = delegations
        .split_whitespace()
        .map(|d| delegation_to_controller(d).unwrap())
        .collect();
    Ok(delegations)
}

fn delegation_to_controller<T: Into<String>>(delegation: T) -> Result<Controller> {
    match delegation.into().as_ref() {
        "cpu" => Ok(Controller::Cpu),
        "cpuset" => Ok(Controller::Cpuset),
        "freezer" => Ok(Controller::Freezer),
        "hugetlb" => Ok(Controller::Hugetlb),
        "io" => Ok(Controller::Io),
        "memory" => Ok(Controller::Memory),
        "perf_event" => Ok(Controller::PerfEvent),
        "pids" => Ok(Controller::Pids),
        "rdma" => Ok(Controller::Rdma),
        _ => Err(Box::new(SquishError::CgroupDelegationInvalid)),
    }
}

#[cfg(test)]
mod test {
    use super::Controller;
    use nix::unistd::getuid;

    #[tokio::test]
    pub async fn detects_current_cgroup() {
        let uid = getuid().as_raw();
        assert_eq!(
            format!("/user.slice/user-{}.slice/session-1.scope", uid),
            super::detect_current_cgroup().await.unwrap()
        );
    }

    #[tokio::test]
    pub async fn parses_cgroup_delegations() {
        assert_eq!(
            vec![Controller::Memory, Controller::Pids],
            super::parse_cgroup_controller_delegations("memory pids").unwrap()
        );
    }

    #[tokio::test]
    pub async fn parses_cgroup_delegations_from_fs() {
        // memory and pids will always be valid controller delegations
        let delegations = super::detect_cgroup_controller_delegations().await.unwrap();
        assert!(delegations.contains(&Controller::Memory));
        assert!(delegations.contains(&Controller::Pids));
    }

    #[tokio::test]
    pub async fn test_cgroup_slice_detection() {
        let slice = super::detect_current_cgroup_slice().await.unwrap();
        assert_eq!("/user.slice/user-1000.slice", slice);
    }

    #[tokio::test]
    pub async fn test_cgroup_slice_name_detection() {
        let slice = super::detect_current_cgroup_slice_name().await.unwrap();
        assert_eq!("user-1000.slice", slice);
    }
}
