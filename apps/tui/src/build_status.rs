//! Launcher build status, sampled off the render thread.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

/// Private status-file handshake between the launcher and dashboard.
pub const STATUS_ENV: &str = "LIGHT_TUI_BUILD_STATUS";

/// The latest real compiler result, independent of simulation health.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BuildState {
    #[default]
    Standalone,
    Pending,
    Compiling,
    Success,
    Failed,
    Unavailable,
}

impl BuildState {
    /// Compact label used in the dashboard corner.
    pub fn label(self) -> &'static str {
        match self {
            Self::Standalone => "build --",
            Self::Pending => "pending",
            Self::Compiling => "compiling",
            Self::Success => "build ok",
            Self::Failed => "build failed",
            Self::Unavailable => "build unknown",
        }
    }

    /// Whether the rectangle's highlight should circulate.
    pub fn animated(self) -> bool {
        matches!(self, Self::Pending | Self::Compiling)
    }
}

/// Atomically replace the tiny status record; readers never see a partial write.
pub fn write_status(path: &Path, state: BuildState) -> io::Result<()> {
    let token = match state {
        BuildState::Pending => "pending",
        BuildState::Compiling => "compiling",
        BuildState::Success => "success",
        BuildState::Failed => "failed",
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a launcher build state",
            ));
        }
    };
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, token)?;
    fs::rename(temporary, path)
}

fn read_status(path: &Path) -> Result<BuildState, String> {
    let mut token = String::new();
    fs::File::open(path)
        .and_then(|file| file.take(64).read_to_string(&mut token))
        .map_err(|error| format!("cannot read build status: {error}"))?;
    match token.as_str() {
        "pending" => Ok(BuildState::Pending),
        "compiling" => Ok(BuildState::Compiling),
        "success" => Ok(BuildState::Success),
        "failed" => Ok(BuildState::Failed),
        _ => Err("invalid launcher build status record".into()),
    }
}

/// File I/O happens exclusively on a short-lived background sampler.
pub struct BuildMonitor {
    updates: Receiver<Result<BuildState, String>>,
    stop: Arc<AtomicBool>,
}

impl BuildMonitor {
    /// Ordinary dashboard launches have no compiler to monitor.
    pub fn from_env() -> io::Result<Option<Self>> {
        std::env::var_os(STATUS_ENV)
            .map(PathBuf::from)
            .map(Self::start)
            .transpose()
    }

    /// Start monitoring a launcher's status record.
    pub fn start(path: PathBuf) -> io::Result<Self> {
        let (sender, updates) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        thread::Builder::new()
            .name("light-build-status".into())
            .spawn(move || {
                let mut delivered = None;
                while !worker_stop.load(Ordering::Relaxed) {
                    let latest = read_status(&path);
                    if delivered.as_ref() != Some(&latest) {
                        match sender.try_send(latest.clone()) {
                            Ok(()) => delivered = Some(latest),
                            Err(mpsc::TrySendError::Full(_)) => {}
                            Err(mpsc::TrySendError::Disconnected(_)) => break,
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            })?;
        Ok(Self { updates, stop })
    }

    /// Drain at most one update without blocking or touching the filesystem.
    pub fn poll(&mut self) -> Option<Result<BuildState, String>> {
        self.updates.try_recv().ok()
    }
}

impl Drop for BuildMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::time::Instant;

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn directory() -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "light-status-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        directory
    }

    fn wait(monitor: &mut BuildMonitor, expected: Result<BuildState, String>) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if monitor.poll() == Some(expected.clone()) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "status update timed out: {expected:?}"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn statuses_round_trip_and_atomically_replace_previous_records() {
        let directory = directory();
        let path = directory.join("status");
        for state in [
            BuildState::Pending,
            BuildState::Compiling,
            BuildState::Failed,
            BuildState::Success,
        ] {
            write_status(&path, state).unwrap();
            assert_eq!(read_status(&path).unwrap(), state);
            assert!(!path.with_extension("tmp").exists());
        }
        assert!(write_status(&path, BuildState::Standalone).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn monitor_reports_failure_then_recovers_without_blocking() {
        let directory = directory();
        let path = directory.join("status");
        write_status(&path, BuildState::Compiling).unwrap();
        let mut monitor = BuildMonitor::start(path.clone()).unwrap();
        wait(&mut monitor, Ok(BuildState::Compiling));
        assert!(monitor.poll().is_none());
        write_status(&path, BuildState::Failed).unwrap();
        wait(&mut monitor, Ok(BuildState::Failed));
        fs::write(&path, b"invalid").unwrap();
        wait(
            &mut monitor,
            Err("invalid launcher build status record".into()),
        );
        write_status(&path, BuildState::Success).unwrap();
        wait(&mut monitor, Ok(BuildState::Success));
        drop(monitor);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn invalid_and_missing_records_do_not_look_like_success() {
        let directory = directory();
        let path = directory.join("status");
        assert!(read_status(&path).is_err());
        fs::write(&path, b"success plus garbage").unwrap();
        assert!(read_status(&path).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn only_work_in_progress_animates() {
        for state in [BuildState::Pending, BuildState::Compiling] {
            assert!(state.animated());
        }
        for state in [
            BuildState::Standalone,
            BuildState::Success,
            BuildState::Failed,
            BuildState::Unavailable,
        ] {
            assert!(!state.animated());
        }
    }
}
