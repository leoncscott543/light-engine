//! Development supervisor: watch, gracefully stop, rebuild, and relaunch the TUI.

mod watch;

use light_tui::build_status::{self, BuildState};
use light_tui::reload;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(100);
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

const USAGE: &str = "\
light-dev — live development for the Light dashboard

USAGE:
    cargo run --locked -p light_tui --bin light-dev -- [dashboard options]

Watches workspace Rust sources, Cargo manifests/lockfile and build configuration.
Saves compile in the background; only successful builds replace the dashboard.
Failed builds leave the current dashboard running. See its corner build badge
and target/simtui-dev/build.log for diagnostics. Quit the dashboard to
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
    command
        .args(args)
        .env(reload::SIGNAL_ENV, signal)
        .env(build_status::STATUS_ENV, signal.with_extension("status"));
    command
}

struct Session {
    child: Option<Child>,
    signal: PathBuf,
    stopping_since: Option<Instant>,
    state: Option<BuildState>,
    live_binary: Option<PathBuf>,
    prepared: Option<PathBuf>,
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
            state: None,
            live_binary: None,
            prepared: None,
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

    fn set_status(&mut self, state: BuildState) {
        if self.state != Some(state) {
            match build_status::write_status(&self.signal.with_extension("status"), state) {
                Ok(()) => self.state = Some(state),
                Err(error) => eprintln!("light-dev: cannot publish build status: {error}"),
            }
        }
    }

    fn launch_prepared(&mut self, binary: &Path, args: &[OsString]) -> io::Result<()> {
        if let Some(previous) = self.live_binary.take() {
            fs::remove_file(previous)?;
        }
        self.launch(binary, args)?;
        self.live_binary = Some(binary.to_path_buf());
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
        for path in [
            Some(self.signal.with_extension("status")),
            Some(self.signal.with_extension("tmp")),
            self.live_binary.take(),
            self.prepared.take(),
        ]
        .into_iter()
        .flatten()
        {
            if let Err(error) = fs::remove_file(path)
                && error.kind() != io::ErrorKind::NotFound
            {
                eprintln!("light-dev: cannot remove development file: {error}");
            }
        }
    }
}

struct BuildJob {
    child: Child,
    obsolete: bool,
}

impl Drop for BuildJob {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            result => {
                if let Err(error) = result {
                    eprintln!("light-dev: cannot check compiler exit: {error}");
                }
                if let Err(error) = self.child.kill() {
                    eprintln!("light-dev: cannot stop compiler: {error}");
                }
                if let Err(error) = self.child.wait() {
                    eprintln!("light-dev: cannot reap compiler: {error}");
                }
            }
        }
    }
}

fn compile(root: &Path, target: &Path, log: &Path) -> io::Result<Child> {
    let output = fs::File::create(log)?;
    build_command(root, target)
        .stdin(Stdio::null())
        .stdout(Stdio::from(output.try_clone()?))
        .stderr(Stdio::from(output))
        .spawn()
}

fn supervise(root: &Path, args: &[OsString]) -> io::Result<ExitCode> {
    // Cargo never writes a loaded executable, including on Windows.
    let target = root.join("target").join("simtui-dev");
    let staging = target.join("build");
    let binary = target
        .join("build")
        .join("debug")
        .join(format!("light{}", std::env::consts::EXE_SUFFIX));
    supervise_with(root, args, &binary, || {
        compile(root, &staging, &target.join("build.log"))
    })
}

fn supervise_with(
    root: &Path,
    args: &[OsString],
    binary: &Path,
    mut compile: impl FnMut() -> io::Result<Child>,
) -> io::Result<ExitCode> {
    let target = root.join("target").join("simtui-dev");
    let mut session = Session::new(&target)?;
    let mut watcher = watch::Watcher::new(root.to_path_buf())?;
    let mut needs_build = true;
    let mut building: Option<BuildJob> = None;
    let mut generation = 0;
    loop {
        let changed = watcher.poll(Instant::now())?;
        if changed || watcher.pending() {
            needs_build = true;
            if let Some(job) = &mut building {
                job.obsolete = true;
            }
            // Once shutdown is requested, the verified replacement is committed.
            // New edits schedule the next build but must not remove that lifeline.
            session.set_status(BuildState::Pending);
        }

        if let Some(event) = session.poll(Instant::now())? {
            match event {
                ChildEvent::Reload => {
                    session.stopping_since = None;
                    if let Some(path) = session.prepared.take() {
                        session.launch_prepared(&path, args)?;
                    }
                }
                ChildEvent::Quit(status) => return Ok(child_exit(status)),
            }
        }

        if let Some(job) = &mut building
            && let Some(status) = job.child.try_wait()?
        {
            let obsolete = job.obsolete || watcher.pending();
            building = None;
            match after_build(status.success(), obsolete) {
                BuildOutcome::Launch => {
                    let path = target.join(format!(
                        "light-live-{}-{}{}",
                        std::process::id(),
                        generation,
                        std::env::consts::EXE_SUFFIX
                    ));
                    generation += 1;
                    match fs::copy(binary, &path) {
                        Ok(_) => {
                            session.set_status(BuildState::Success);
                            if session.child.is_some() {
                                session.prepared = Some(path);
                                if let Err(error) = session.request_reload() {
                                    if let Some(path) = session.prepared.take()
                                        && let Err(cleanup_error) = fs::remove_file(path)
                                    {
                                        eprintln!(
                                            "light-dev: cannot remove unused replacement: {cleanup_error}"
                                        );
                                    }
                                    record_failure(
                                        &mut session,
                                        &target,
                                        &format!("cannot request reload: {error}"),
                                    );
                                }
                            } else {
                                session.launch_prepared(&path, args)?;
                            }
                        }
                        Err(error) => {
                            record_failure(
                                &mut session,
                                &target,
                                &format!("cannot stage dashboard: {error}"),
                            );
                        }
                    }
                }
                BuildOutcome::Wait if obsolete => {
                    needs_build = true;
                    session.set_status(BuildState::Pending);
                }
                BuildOutcome::Wait => {
                    record_failure(&mut session, &target, "compilation failed");
                }
            }
        }

        if needs_build
            && !watcher.pending()
            && building.is_none()
            && session.prepared.is_none()
            && session.stopping_since.is_none()
        {
            needs_build = false;
            session.set_status(BuildState::Compiling);
            if session.child.is_none() {
                eprintln!(
                    "light-dev: building dashboard; diagnostics: {}",
                    target.join("build.log").display()
                );
            }
            match compile() {
                Ok(child) => {
                    building = Some(BuildJob {
                        child,
                        obsolete: false,
                    })
                }
                Err(error) => {
                    record_failure(
                        &mut session,
                        &target,
                        &format!("cannot start compiler: {error}"),
                    );
                }
            }
        }
        thread::sleep(POLL);
    }
}

fn record_failure(session: &mut Session, target: &Path, reason: &str) {
    use std::io::Write;

    let result = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target.join("build.log"))
        .and_then(|mut log| {
            writeln!(
                log,
                "\nlight-dev: {reason}; keeping the current dashboard (if running)"
            )
        });
    if let Err(error) = result {
        eprintln!("light-dev: {reason}; cannot write build log: {error}");
    }
    session.set_status(BuildState::Failed);
    if session.child.is_none() {
        eprintln!(
            "light-dev: {reason}; save to retry. See {}",
            target.join("build.log").display()
        );
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
            vec![
                (
                    std::ffi::OsStr::new(build_status::STATUS_ENV),
                    Some(std::ffi::OsStr::new("reload.status"))
                ),
                (
                    std::ffi::OsStr::new(reload::SIGNAL_ENV),
                    Some(std::ffi::OsStr::new("reload"))
                ),
            ]
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
    fn broken_status_and_log_files_cannot_stop_the_running_dashboard() {
        let mut session = session();
        session
            .launch(
                &std::env::current_exe().unwrap(),
                &[
                    "--exact".into(),
                    "tests::dashboard_child".into(),
                    "--ignored".into(),
                ],
            )
            .unwrap();
        let status = session.signal.with_extension("status");
        fs::create_dir(&status).unwrap();
        let missing_log_directory = session.signal.parent().unwrap().join("missing");
        record_failure(&mut session, &missing_log_directory, "test build failed");
        assert!(
            session
                .child
                .as_mut()
                .unwrap()
                .try_wait()
                .unwrap()
                .is_none()
        );
        assert!(
            !session.signal.exists(),
            "reporting errors must not request reload"
        );
        fs::remove_dir(status).unwrap();
        session.request_reload().unwrap();
        assert!(matches!(wait_for_exit(&mut session), ChildEvent::Reload));
        let temporary = session.signal.with_extension("tmp");
        if temporary.exists() {
            fs::remove_file(temporary).unwrap();
        }
        finish(session);
    }

    #[test]
    fn unresponsive_child_reports_timeout_and_cleanup_stops_it() {
        let mut session = session();
        let directory = session.signal.parent().unwrap().to_path_buf();
        session
            .launch(
                &std::env::current_exe().unwrap(),
                &[
                    "--exact".into(),
                    "tests::dashboard_child".into(),
                    "--ignored".into(),
                ],
            )
            .unwrap();
        let now = Instant::now();
        session.stopping_since = Some(now);
        assert_eq!(
            session.poll(now + STOP_TIMEOUT).err().unwrap().kind(),
            io::ErrorKind::TimedOut
        );
        drop(session);
        assert!(fs::read_dir(&directory).unwrap().next().is_none());
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn supervisor_keeps_the_same_dashboard_alive_on_failed_build_and_reloads_only_success() {
        failure_scenario(Failure::Compiler);
    }

    #[test]
    fn compiler_spawn_failure_leaves_the_current_dashboard_alive() {
        failure_scenario(Failure::Spawn);
    }

    #[test]
    fn missing_build_artifact_leaves_the_current_dashboard_alive() {
        failure_scenario(Failure::Artifact);
    }

    enum Failure {
        Compiler,
        Spawn,
        Artifact,
    }

    fn failure_scenario(failure: Failure) {
        let session = session();
        let root = session.signal.parent().unwrap().to_path_buf();
        drop(session);
        fs::create_dir(root.join("src")).unwrap();
        let source = root.join("src").join("lib.rs");
        fs::write(&source, b"initial").unwrap();
        let target = root.join("target").join("simtui-dev");
        let ready = target.join("ready");
        let done = target.join("done");
        let writer_ready = ready.clone();
        let status = target.join(format!("reload-{}.status", std::process::id()));
        let signal = target.join(format!("reload-{}", std::process::id()));
        let heartbeat = target.join("heartbeat");
        let writer = thread::spawn(move || {
            wait_until(|| writer_ready.is_file());
            let first_pid = fs::read(&writer_ready).unwrap();
            fs::write(&source, b"invalid build").unwrap();
            wait_until(|| fs::read_to_string(&status).is_ok_and(|state| state == "failed"));
            assert!(!signal.exists(), "failed build must not request shutdown");
            assert_eq!(fs::read(&writer_ready).unwrap(), first_pid);
            let before = fs::read(&heartbeat).unwrap_or_default();
            wait_until(|| fs::read(&heartbeat).is_ok_and(|after| after != before));
            assert_eq!(
                fs::read(&writer_ready).unwrap(),
                first_pid,
                "old UI must still be running"
            );
            fs::write(&source, b"fixed build").unwrap();
        });
        let binary = root.join(format!("staged{}", std::env::consts::EXE_SUFFIX));
        fs::copy(std::env::current_exe().unwrap(), &binary).unwrap();
        let mut builds = 0;
        let result = supervise_with(
            &root,
            &[
                "--exact".into(),
                "tests::dashboard_cycle_child".into(),
                "--ignored".into(),
            ],
            &binary,
            || {
                builds += 1;
                match builds {
                    1 => test_compile(true),
                    2 => match failure {
                        Failure::Compiler => test_compile(false),
                        Failure::Spawn => Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            "test compiler missing",
                        )),
                        Failure::Artifact => {
                            fs::remove_file(&binary)?;
                            test_compile(true)
                        }
                    },
                    3 => {
                        fs::copy(std::env::current_exe()?, &binary)?;
                        fs::write(&done, b"done")?;
                        test_compile(true)
                    }
                    _ => panic!("supervisor rebuilt without another source change"),
                }
            },
        );
        writer.join().unwrap();
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
        assert_eq!(builds, 3);
        assert!(
            !target
                .join(format!("reload-{}", std::process::id()))
                .exists()
        );
        fs::remove_dir_all(&root).unwrap();
    }

    fn wait_until(condition: impl Fn() -> bool) {
        let deadline = Instant::now() + STOP_TIMEOUT;
        while !condition() {
            assert!(Instant::now() < deadline, "supervisor did not progress");
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn test_compile(success: bool) -> io::Result<Child> {
        Command::new(std::env::current_exe()?)
            .arg(if success {
                "--list"
            } else {
                "--invalid-test-argument"
            })
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    }

    fn workspace() -> (PathBuf, PathBuf) {
        let session = session();
        let root = session.signal.parent().unwrap().to_path_buf();
        drop(session);
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src").join("lib.rs"), b"initial").unwrap();
        let target = root.join("target").join("simtui-dev");
        (root, target)
    }

    fn cycle_args() -> Vec<OsString> {
        vec![
            "--exact".into(),
            "tests::dashboard_cycle_child".into(),
            "--ignored".into(),
        ]
    }

    fn gated_compile(target: &Path) -> io::Result<Child> {
        Command::new(std::env::current_exe()?)
            .args(["--exact", "tests::compiler_child", "--ignored"])
            .env("LIGHT_DEV_TEST_COMPILER_DIRECTORY", target)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
    }

    #[test]
    fn failed_initial_build_waits_for_edits_and_recovers() {
        let (root, target) = workspace();
        let status = target.join(format!("reload-{}.status", std::process::id()));
        let source = root.join("src").join("lib.rs");
        let writer = thread::spawn(move || {
            wait_until(|| fs::read_to_string(&status).is_ok_and(|state| state == "failed"));
            fs::write(source, b"fixed").unwrap();
        });
        let mut builds = 0;
        let result = supervise_with(
            &root,
            &cycle_args(),
            &std::env::current_exe().unwrap(),
            || {
                builds += 1;
                match builds {
                    1 => test_compile(false),
                    2 => {
                        fs::write(target.join("done"), b"done")?;
                        test_compile(true)
                    }
                    _ => panic!("unexpected extra build"),
                }
            },
        );
        writer.join().unwrap();
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
        assert_eq!(builds, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_edits_during_compilation_do_not_replace_dashboard_with_stale_build() {
        let (root, target) = workspace();
        let source = root.join("src").join("lib.rs");
        let writer_target = target.clone();
        let writer = thread::spawn(move || {
            let ready = writer_target.join("ready");
            wait_until(|| ready.is_file());
            fs::write(&source, b"edit one").unwrap();
            wait_until(|| writer_target.join("compiler-started").is_file());
            let pid = fs::read(&ready).unwrap();
            fs::write(&source, b"edit two").unwrap();
            let status = writer_target.join(format!("reload-{}.status", std::process::id()));
            wait_until(|| fs::read_to_string(&status).is_ok_and(|state| state == "pending"));
            assert_eq!(fs::read(&ready).unwrap(), pid);
            fs::write(writer_target.join("compiler-release"), b"go").unwrap();
        });
        let mut builds = 0;
        let mut first_pid = Vec::new();
        let result = supervise_with(
            &root,
            &cycle_args(),
            &std::env::current_exe().unwrap(),
            || {
                builds += 1;
                match builds {
                    1 => test_compile(true),
                    2 => {
                        first_pid = fs::read(target.join("ready"))?;
                        gated_compile(&target)
                    }
                    3 => {
                        assert_eq!(fs::read(target.join("ready"))?, first_pid);
                        assert!(
                            !target
                                .join(format!("reload-{}", std::process::id()))
                                .exists()
                        );
                        fs::write(target.join("done"), b"done")?;
                        test_compile(true)
                    }
                    _ => panic!("unexpected extra build"),
                }
            },
        );
        writer.join().unwrap();
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
        assert_eq!(builds, 3);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn quitting_dashboard_while_compiling_stops_supervisor_promptly() {
        let (root, target) = workspace();
        let source = root.join("src").join("lib.rs");
        let writer_target = target.clone();
        let writer = thread::spawn(move || {
            wait_until(|| writer_target.join("ready").is_file());
            fs::write(source, b"rebuild").unwrap();
            wait_until(|| writer_target.join("compiler-started").is_file());
            fs::write(writer_target.join("quit"), b"quit").unwrap();
        });
        let mut builds = 0;
        let started = Instant::now();
        let result = supervise_with(
            &root,
            &cycle_args(),
            &std::env::current_exe().unwrap(),
            || {
                builds += 1;
                match builds {
                    1 => test_compile(true),
                    2 => gated_compile(&target),
                    _ => panic!("unexpected extra build"),
                }
            },
        );
        writer.join().unwrap();
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
        assert!(
            started.elapsed() < STOP_TIMEOUT,
            "quit must not wait for compilation"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn edits_during_handoff_preserve_the_committed_successful_replacement() {
        let (root, target) = workspace();
        let source = root.join("src").join("lib.rs");
        let writer_target = target.clone();
        let writer = thread::spawn(move || {
            let ready = writer_target.join("ready");
            wait_until(|| ready.is_file());
            let first = fs::read(&ready).unwrap();
            fs::write(writer_target.join("delay-exit"), b"wait").unwrap();
            fs::write(&source, b"successful replacement").unwrap();
            wait_until(|| writer_target.join("exiting").is_file());
            fs::write(&source, b"broken next edit").unwrap();
            let status = writer_target.join(format!("reload-{}.status", std::process::id()));
            wait_until(|| fs::read_to_string(&status).is_ok_and(|state| state == "pending"));
            fs::write(writer_target.join("allow-exit"), b"go").unwrap();
            wait_until(|| fs::read(&ready).is_ok_and(|pid| !pid.is_empty() && pid != first));
            wait_until(|| fs::read_to_string(&status).is_ok_and(|state| state == "failed"));
            assert!(
                !writer_target
                    .join(format!("reload-{}", std::process::id()))
                    .exists()
            );
            let before = fs::read(writer_target.join("heartbeat")).unwrap();
            wait_until(|| {
                fs::read(writer_target.join("heartbeat")).is_ok_and(|after| after != before)
            });
            fs::write(writer_target.join("quit"), b"quit").unwrap();
        });
        let mut builds = 0;
        let result = supervise_with(
            &root,
            &cycle_args(),
            &std::env::current_exe().unwrap(),
            || {
                builds += 1;
                match builds {
                    1 | 2 => test_compile(true),
                    3 => test_compile(false),
                    _ => panic!("unexpected extra build"),
                }
            },
        );
        writer.join().unwrap();
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
        assert_eq!(builds, 3);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "subprocess fixture, launched by compiler lifecycle tests"]
    fn compiler_child() {
        let directory =
            PathBuf::from(std::env::var_os("LIGHT_DEV_TEST_COMPILER_DIRECTORY").unwrap());
        fs::write(directory.join("compiler-started"), b"started").unwrap();
        wait_until(|| directory.join("compiler-release").exists());
    }

    #[test]
    #[ignore = "subprocess fixture, launched by supervisor tests"]
    fn dashboard_cycle_child() {
        let signal = PathBuf::from(std::env::var_os(reload::SIGNAL_ENV).unwrap());
        let directory = signal.parent().unwrap();
        if directory.join("done").exists() {
            return;
        }
        fs::write(directory.join("ready"), std::process::id().to_string()).unwrap();
        let request = reload::Signal::from_env();
        let deadline = Instant::now() + STOP_TIMEOUT;
        let mut frames = 0_u64;
        while !request.requested().unwrap() {
            if directory.join("quit").exists() {
                return;
            }
            assert!(Instant::now() < deadline, "reload was not requested");
            frames += 1;
            fs::write(directory.join("heartbeat"), frames.to_string()).unwrap();
            thread::sleep(Duration::from_millis(10));
        }
        if directory.join("delay-exit").exists() {
            fs::write(directory.join("exiting"), std::process::id().to_string()).unwrap();
            wait_until(|| directory.join("allow-exit").exists());
        }
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
