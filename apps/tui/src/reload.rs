//! Cooperative shutdown handshake between the dashboard and its dev launcher.

use std::io;
use std::path::PathBuf;

/// Private launcher-to-dashboard environment variable; not a user-facing option.
pub const SIGNAL_ENV: &str = "LIGHT_TUI_RELOAD_SIGNAL";

/// Optional reload request, disabled in ordinary dashboard launches.
pub struct Signal {
    path: Option<PathBuf>,
}

impl Signal {
    /// Read the launcher handshake once, before the dashboard's render loop.
    pub fn from_env() -> Self {
        Self {
            path: std::env::var_os(SIGNAL_ENV).map(PathBuf::from),
        }
    }

    /// Report filesystem errors rather than silently losing live reload.
    pub fn requested(&self) -> io::Result<bool> {
        match &self.path {
            Some(path) => match std::fs::metadata(path) {
                Ok(metadata) if metadata.is_file() => Ok(true),
                Ok(_) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("reload signal is not a file: {}", path.display()),
                )),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(error),
            },
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_launch_never_requests_reload() {
        assert!(!Signal { path: None }.requested().unwrap());
    }

    #[test]
    fn missing_signal_does_not_request_reload() {
        let path = std::env::temp_dir().join(format!(
            "light-missing-signal-{}-{}",
            std::process::id(),
            line!()
        ));
        assert!(!Signal { path: Some(path) }.requested().unwrap());
    }

    #[test]
    fn existing_file_requests_reload() {
        let signal = Signal {
            path: Some(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("src")
                    .join("reload.rs"),
            ),
        };
        assert!(signal.requested().unwrap());
    }

    #[test]
    fn invalid_signal_is_reported() {
        let signal = Signal {
            path: Some(std::env::temp_dir()),
        };
        assert_eq!(
            signal.requested().unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
