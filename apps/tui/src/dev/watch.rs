//! Content-based workspace watching with debounce, including atomic editor saves.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type Snapshot = BTreeMap<PathBuf, Vec<u8>>;

pub const DEBOUNCE: Duration = Duration::from_millis(300);

pub struct Watcher {
    root: PathBuf,
    latest: Snapshot,
    changed_at: Option<Instant>,
}

impl Watcher {
    pub fn new(root: PathBuf) -> io::Result<Self> {
        let latest = snapshot(&root)?;
        Ok(Self {
            root,
            latest,
            changed_at: None,
        })
    }

    pub fn poll(&mut self, now: Instant) -> io::Result<bool> {
        Ok(self.observe(snapshot(&self.root)?, now))
    }

    pub fn pending(&self) -> bool {
        self.changed_at.is_some()
    }

    fn observe(&mut self, current: Snapshot, now: Instant) -> bool {
        if current != self.latest {
            self.latest = current;
            self.changed_at = Some(now);
        }
        if self
            .changed_at
            .is_some_and(|changed| now.duration_since(changed) >= DEBOUNCE)
        {
            self.changed_at = None;
            return true;
        }
        false
    }
}

fn snapshot(root: &Path) -> io::Result<Snapshot> {
    let mut files = Snapshot::new();
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain",
        "rust-toolchain.toml",
    ] {
        read_file(&root.join(name), &mut files)?;
    }
    for name in ["apps", "crates", "src", ".cargo"] {
        visit(&root.join(name), &mut files)?;
    }
    Ok(files)
}

fn visit(path: &Path, files: &mut Snapshot) -> io::Result<()> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some("target" | ".git" | "node_modules" | ".idea" | ".vscode")
            ) {
                visit(&path, files)?;
            }
        } else if kind.is_file() && watched(&path) {
            read_file(&path, files)?;
        }
    }
    Ok(())
}

fn watched(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("rs" | "toml" | "lock")
    ) || matches!(
        path.file_name().and_then(OsStr::to_str),
        Some("config" | "rust-toolchain")
    )
}

fn read_file(path: &Path, files: &mut Snapshot) -> io::Result<()> {
    match fs::read(path) {
        Ok(bytes) => {
            files.insert(path.to_path_buf(), bytes);
            Ok(())
        }
        // Editors may replace a file between directory enumeration and reading it.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    struct Workspace(PathBuf);

    impl Workspace {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "light-watch-test-{}-{}",
                std::process::id(),
                NEXT_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn write(&self, path: &str, bytes: &[u8]) {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }

    impl Drop for Workspace {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn changed(path: &str, content: &[u8]) -> Snapshot {
        BTreeMap::from([(PathBuf::from(path), content.to_vec())])
    }

    fn watcher() -> Watcher {
        Watcher {
            root: PathBuf::new(),
            latest: Snapshot::new(),
            changed_at: None,
        }
    }

    #[test]
    fn unchanged_files_do_not_reload() {
        let mut watcher = watcher();
        let now = Instant::now();
        assert!(!watcher.observe(Snapshot::new(), now));
        assert!(!watcher.observe(Snapshot::new(), now + DEBOUNCE));
        assert!(!watcher.pending());
    }

    #[test]
    fn changes_debounce_and_fire_exactly_once() {
        let mut watcher = watcher();
        let now = Instant::now();
        let files = changed("main.rs", b"hello");
        assert!(!watcher.observe(files.clone(), now));
        assert!(watcher.pending());
        assert!(!watcher.observe(files.clone(), now + DEBOUNCE / 2));
        assert!(watcher.observe(files.clone(), now + DEBOUNCE));
        assert!(!watcher.pending());
        assert!(!watcher.observe(files, now + DEBOUNCE * 2));
    }

    #[test]
    fn rapid_edits_reset_debounce() {
        let mut watcher = watcher();
        let now = Instant::now();
        watcher.observe(changed("main.rs", b"a"), now);
        let files = changed("main.rs", b"b");
        assert!(!watcher.observe(files.clone(), now + DEBOUNCE / 2));
        assert!(!watcher.observe(files.clone(), now + DEBOUNCE));
        assert!(watcher.observe(files, now + DEBOUNCE * 2));
    }

    #[test]
    fn same_size_content_edits_are_detected() {
        let workspace = Workspace::new();
        let source = PathBuf::from("apps")
            .join("tui")
            .join("src")
            .join("main.rs");
        workspace.write(source.to_str().unwrap(), b"old");
        let mut watcher = Watcher::new(workspace.0.clone()).unwrap();
        workspace.write(source.to_str().unwrap(), b"new");
        assert!(!watcher.poll(Instant::now()).unwrap());
        assert!(watcher.pending());
    }

    #[test]
    fn file_creation_deletion_and_rename_are_detected() {
        let workspace = Workspace::new();
        let mut watcher = Watcher::new(workspace.0.clone()).unwrap();
        let now = Instant::now();
        let source = PathBuf::from("crates")
            .join("mock")
            .join("src")
            .join("lib.rs");
        workspace.write(source.to_str().unwrap(), b"new");
        assert!(!watcher.poll(now).unwrap());
        assert!(watcher.poll(now + DEBOUNCE).unwrap());
        let renamed = workspace.0.join(source.with_file_name("other.rs"));
        fs::rename(workspace.0.join(&source), &renamed).unwrap();
        assert!(!watcher.poll(now + DEBOUNCE * 2).unwrap());
        assert!(watcher.poll(now + DEBOUNCE * 3).unwrap());
        fs::remove_file(renamed).unwrap();
        assert!(!watcher.poll(now + DEBOUNCE * 4).unwrap());
        assert!(watcher.poll(now + DEBOUNCE * 5).unwrap());
    }

    #[test]
    fn atomic_save_with_identical_content_does_not_reload() {
        let workspace = Workspace::new();
        workspace.write("Cargo.toml", b"same");
        let mut watcher = Watcher::new(workspace.0.clone()).unwrap();
        fs::remove_file(workspace.0.join("Cargo.toml")).unwrap();
        workspace.write("Cargo.toml", b"same");
        assert!(!watcher.poll(Instant::now()).unwrap());
        assert!(!watcher.pending());
    }

    #[test]
    fn watches_workspace_sources_manifests_and_build_configuration() {
        let workspace = Workspace::new();
        let paths = [
            PathBuf::from("Cargo.toml"),
            PathBuf::from("Cargo.lock"),
            PathBuf::from("rust-toolchain.toml"),
            PathBuf::from(".cargo").join("config.toml"),
            PathBuf::from(".cargo").join("config"),
            PathBuf::from("src").join("main.rs"),
            PathBuf::from("apps").join("tui").join("Cargo.toml"),
            PathBuf::from("apps")
                .join("tui")
                .join("src")
                .join("views")
                .join("new.rs"),
            PathBuf::from("crates").join("mock").join("build.rs"),
        ];
        for path in &paths {
            workspace.write(path.to_str().unwrap(), b"source");
        }
        let files = snapshot(&workspace.0).unwrap();
        for path in paths {
            assert!(files.contains_key(&workspace.0.join(path)));
        }
    }

    #[test]
    fn ignores_outputs_git_secrets_and_editor_temporary_files() {
        let workspace = Workspace::new();
        for path in [
            PathBuf::from("target").join("generated.rs"),
            PathBuf::from("apps")
                .join("tui")
                .join("target")
                .join("generated.rs"),
            PathBuf::from("crates").join(".git").join("hidden.rs"),
            PathBuf::from(".env"),
            PathBuf::from("apps")
                .join("tui")
                .join("src")
                .join("main.rs.swp"),
        ] {
            workspace.write(path.to_str().unwrap(), b"ignored");
        }
        assert!(snapshot(&workspace.0).unwrap().is_empty());
    }

    #[test]
    fn filesystem_errors_are_not_hidden() {
        let workspace = Workspace::new();
        workspace.write("apps", b"not a directory");
        assert!(Watcher::new(workspace.0.clone()).is_err());
    }
}
