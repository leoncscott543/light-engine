//! Development supervisor: watch, gracefully stop, rebuild, and relaunch the TUI.

mod watch;

use light_tui::reload;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(100);
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

const USAGE: &str = "\
light-dev — live development for the Light dashboard

USAGE:
    cargo run --locked -p light_tui --bin light-dev -- [dashboard options]

Watches workspace Rust sources, Cargo manifests/lockfile and build configuration.
Saves are debounced, then the dashboard gracefully exits, rebuilds and restarts.
Build errors remain visible; save another edit to retry. Quit the dashboard to
stop watching. Dashboard options (e.g. --session --seed 0x1234) are forwarded.
Changes to the launcher itself require restarting this command.
";

#[derive(Debug, PartialEq, Eq)]
enum BuildOutcome {
    Launch,
    Wait,
}

fn after_build(success: bool, changed: bool) -> BuildOutcome {
    if success && !changed {
        BuildOutcome::Launch
    } else {
        BuildOutcome::Wait
    }
}

fn build_command(root: &Path, target: &Path) -> Command {
    let mut command = Command::new(env!("CARGO"));
    command.current_dir(root).args([
        "build",
        "--locked",
        "-p",
        "light_tui",
        "--bin",
        "light",
        "--target-dir",
    ]);
    command.arg(target);
    command
}

fn dashboard_command(binary: &Path, args: &[OsString], signal: &Path) -> Command {
    let mut command = Command::new(binary);
    command.args(args).env(reload::SIGNAL_ENV, signal);
    command
}

struct Session {
    child: Option<Child>,
    signal: PathBuf,
    stopping_since: Option<Instant>,
}

enum ChildEvent {
    Reload,
    Quit(ExitStatus),
}

impl Session {
    fn new(directory: &Path) -> io::Result<Self> {
        fs::create_dir_all(directory)?;
        let signal = directory.join(format!("reload-{}", std::process::id()));
        let mut session = Self {
            child: None,
            signal,
            stopping_since: None,
        };
        session.clear_signal()?;
        Ok(session)
    }

    fn clear_signal(&mut self) -> io::Result<()> {
        match fs::remove_file(&self.signal) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn request_reload(&mut self) -> io::Result<()> {
        if self.stopping_since.is_none() {
            fs::write(&self.signal, b"reload")?;
            self.stopping_since = Some(Instant::now());
        }
        Ok(())
    }

    fn launch(&mut self, binary: &Path, args: &[OsString]) -> io::Result<()> {
        self.clear_signal()?;
        self.stopping_since = None;
        self.child = Some(dashboard_command(binary, args, &self.signal).spawn()?);
        Ok(())
    }

    fn poll(&mut self, now: Instant) -> io::Result<Option<ChildEvent>> {
        if let Some(child) = &mut self.child {
            if let Some(status) = child.try_wait()? {
                self.child = None;
                return Ok(Some(if self.stopping_since.is_some() && status.success() {
                    ChildEvent::Reload
                } else {
                    ChildEvent::Quit(status)
                }));
            }
            if self
                .stopping_since
                .is_some_and(|since| now.duration_since(since) >= STOP_TIMEOUT)
            {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "dashboard did not acknowledge reload within 10 seconds",
                ));
            }
        }
        Ok(None)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            // Even error paths allow ratatui to restore the terminal before exit.
            if let Err(error) = fs::write(&self.signal, b"reload") {
                eprintln!("light-dev: cannot request dashboard shutdown: {error}");
            }
            let deadline = Instant::now() + STOP_TIMEOUT;
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
                    result => {
                        if let Err(error) = result {
                            eprintln!("light-dev: cannot check dashboard exit: {error}");
                        }
                        eprintln!(
                            "light-dev: forcing dashboard shutdown; terminal may need resetting"
                        );
                        if let Err(error) = child.kill() {
                            eprintln!("light-dev: cannot terminate dashboard: {error}");
                        }
                        if let Err(error) = child.wait() {
                            eprintln!("light-dev: cannot reap dashboard: {error}");
                        }
                        break;
                    }
                }
            }
        }
        if let Err(error) = self.clear_signal() {
            eprintln!("light-dev: cannot remove reload signal: {error}");
        }
    }
}

fn supervise(root: &Path, args: &[OsString]) -> io::Result<ExitCode> {
    // A separate target directory also permits rebuilding the launcher on Windows.
    let target = root.join("target").join("simtui-dev");
    let binary = target
        .join("debug")
        .join(format!("light{}", std::env::consts::EXE_SUFFIX));
    let mut session = Session::new(&target)?;
    let mut watcher = watch::Watcher::new(root.to_path_buf())?;
    let mut build = true;
    loop {
        if build {
            eprintln!("light-dev: building dashboard...");
            let status = build_command(root, &target).status()?;
            watcher.poll(Instant::now())?;
            match after_build(status.success(), watcher.pending()) {
                BuildOutcome::Launch => session.launch(&binary, args)?,
                BuildOutcome::Wait if !status.success() => {
                    eprintln!("light-dev: build failed; watching for the next edit...");
                }
                BuildOutcome::Wait => {
                    eprintln!(
                        "light-dev: files changed during build; waiting for saves to settle..."
                    );
                }
            }
            build = false;
        }

        if let Some(event) = session.poll(Instant::now())? {
            match event {
                ChildEvent::Reload => {
                    build = true;
                    continue;
                }
                ChildEvent::Quit(status) => return Ok(child_exit(status)),
            }
        }

        if watcher.poll(Instant::now())? {
            if session.child.is_some() {
                session.request_reload()?;
            } else {
                build = true;
            }
        }
        thread::sleep(POLL);
    }
}

fn child_exit(status: ExitStatus) -> ExitCode {
    if status.success() {
        ExitCode::SUCCESS
    } else {
        eprintln!("light-dev: dashboard exited unexpectedly: {status}");
        ExitCode::FAILURE
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("TUI package must be inside the workspace's apps directory");
    match supervise(root, &args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("light-dev: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

    fn session() -> Session {
        Session::new(&std::env::temp_dir().join(format!(
            "light-dev-session-test-{}-{}",
            std::process::id(),
            NEXT_SESSION.fetch_add(1, Ordering::Relaxed)
        )))
        .unwrap()
    }

    fn finish(mut session: Session) {
        assert!(session.child.is_none());
        let directory = session.signal.parent().unwrap().to_path_buf();
        session.clear_signal().unwrap();
        drop(session);
        fs::remove_dir(directory).unwrap();
    }

    fn wait_for_exit(session: &mut Session) -> ChildEvent {
        let deadline = Instant::now() + STOP_TIMEOUT;
        loop {
            if let Some(event) = session.poll(Instant::now()).unwrap() {
                return event;
            }
            assert!(Instant::now() < deadline, "test child did not exit");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn successful_unchanged_build_launches() {
        assert_eq!(after_build(true, false), BuildOutcome::Launch);
    }

    #[test]
    fn failed_build_waits_instead_of_running_stale_binary() {
        assert_eq!(after_build(false, false), BuildOutcome::Wait);
        assert_eq!(after_build(false, true), BuildOutcome::Wait);
    }

    #[test]
    fn edits_during_build_prevent_stale_launch() {
        assert_eq!(after_build(true, true), BuildOutcome::Wait);
        assert_eq!(after_build(true, false), BuildOutcome::Launch);
    }

    #[test]
    fn build_is_locked_and_only_targets_dashboard() {
        let root = Path::new("workspace with spaces");
        let target = root.join("target").join("simtui-dev");
        let command = build_command(root, &target);
        assert_eq!(command.get_current_dir(), Some(root));
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(
            args,
            [
                OsString::from("build"),
                OsString::from("--locked"),
                OsString::from("-p"),
                OsString::from("light_tui"),
                OsString::from("--bin"),
                OsString::from("light"),
                OsString::from("--target-dir"),
                target.into_os_string(),
            ]
        );
    }

    #[test]
    fn dashboard_arguments_are_forwarded_without_shell_interpretation() {
        let args = vec![OsString::from("--session"), OsString::from("a b; $test")];
        let command = dashboard_command(Path::new("light"), &args, Path::new("reload"));
        assert_eq!(command.get_args().collect::<Vec<_>>(), args);
        assert_eq!(
            command.get_envs().collect::<Vec<_>>(),
            vec![(
                std::ffi::OsStr::new(reload::SIGNAL_ENV),
                Some(std::ffi::OsStr::new("reload"))
            )]
        );
    }

    #[test]
    fn reload_signal_is_idempotent_and_cleaned_up() {
        let mut session = session();
        let directory = session.signal.parent().unwrap().to_path_buf();
        {
            assert!(!session.signal.exists());
            session.request_reload().unwrap();
            let since = session.stopping_since;
            assert!(session.signal.is_file());
            session.request_reload().unwrap();
            assert_eq!(session.stopping_since, since);
            session.clear_signal().unwrap();
            session.clear_signal().unwrap();
        }
        drop(session);
        assert!(fs::read_dir(&directory).unwrap().next().is_none());
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn graceful_reload_waits_for_child_exit_and_allows_relaunch() {
        let mut session = session();
        let binary = std::env::current_exe().unwrap();
        let args = [
            OsString::from("--exact"),
            OsString::from("tests::dashboard_child"),
            OsString::from("--ignored"),
        ];
        for _ in 0..2 {
            session.launch(&binary, &args).unwrap();
            assert!(!session.signal.exists());
            session.request_reload().unwrap();
            assert!(matches!(wait_for_exit(&mut session), ChildEvent::Reload));
            assert!(session.child.is_none());
        }
        finish(session);
    }

    #[test]
    fn user_quit_does_not_restart() {
        let mut session = session();
        session
            .launch(&std::env::current_exe().unwrap(), &["--list".into()])
            .unwrap();
        match wait_for_exit(&mut session) {
            ChildEvent::Quit(status) => assert!(status.success()),
            ChildEvent::Reload => panic!("normal quit must not restart"),
        }
        finish(session);
    }

    #[test]
    fn child_failure_does_not_restart_even_when_reload_was_requested() {
        let mut session = session();
        session
            .launch(
                &std::env::current_exe().unwrap(),
                &["--invalid-test-argument".into()],
            )
            .unwrap();
        session.request_reload().unwrap();
        match wait_for_exit(&mut session) {
            ChildEvent::Quit(status) => {
                assert!(!status.success());
                assert_eq!(child_exit(status), ExitCode::FAILURE);
            }
            ChildEvent::Reload => panic!("failed child must not restart"),
        }
        finish(session);
    }

    #[test]
    fn missing_executable_is_reported() {
        let mut session = session();
        assert!(
            session
                .launch(&session.signal.with_file_name("missing-binary"), &[])
                .is_err()
        );
        assert!(session.child.is_none());
        finish(session);
    }

    #[test]
    #[ignore = "subprocess fixture, launched by lifecycle tests"]
    fn dashboard_child() {
        let signal = reload::Signal::from_env();
        let deadline = Instant::now() + STOP_TIMEOUT;
        while !signal.requested().unwrap() {
            assert!(Instant::now() < deadline, "reload was not requested");
            thread::sleep(Duration::from_millis(10));
        }
    }
}
