use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use miette::{Context, IntoDiagnostic, Result, miette};
use tokio::io::AsyncWriteExt;

use super::cache_root;

const CACHE_LOCK_RETRY_COUNT: usize = 200;
const CACHE_LOCK_RETRY_DELAY: Duration = Duration::from_millis(25);
const CACHE_LOCK_STALE_AFTER: Duration = Duration::from_secs(5 * 60);
static CACHE_LOCK_OWNER_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) struct CacheLock {
    file: Option<tokio::fs::File>,
    initialized: bool,
    owner: String,
    path: PathBuf,
}

impl CacheLock {
    pub(super) async fn release(mut self) -> Result<()> {
        drop(self.file.take());
        remove_cache_lock_if_owned(&self.path, &self.owner)
            .await
            .map(|_| ())
    }
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        drop(self.file.take());

        if !self.initialized {
            let _cleanup_result = std::fs::remove_file(&self.path);
            return;
        }

        if std::fs::read_to_string(&self.path).is_ok_and(|owner| owner == self.owner) {
            if let Some(root) = self.path.parent() {
                cleanup_cache_temporary_files_sync(root);
            }
            let _cleanup_result = std::fs::remove_file(&self.path);
        }
    }
}

pub(super) async fn acquire(cache_dir: &Path) -> Result<CacheLock> {
    let root = cache_root(cache_dir);
    let lock_path = root.join(".write.lock");
    let owner = cache_lock_owner();
    let mut recovered_lock = false;

    tokio::fs::create_dir_all(&root)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", root.display()))?;

    for _ in 0..CACHE_LOCK_RETRY_COUNT {
        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .await
        {
            Ok(file) => {
                let mut cache_lock = CacheLock {
                    file: Some(file),
                    initialized: false,
                    owner: owner.clone(),
                    path: lock_path.clone(),
                };
                let initialization_result = async {
                    let file = cache_lock
                        .file
                        .as_mut()
                        .expect("new cache locks retain their file");

                    file.write_all(owner.as_bytes()).await?;
                    file.flush().await
                }
                .await;

                if let Err(error) = initialization_result {
                    return Err(error)
                        .into_diagnostic()
                        .wrap_err_with(|| format!("failed to initialize {}", lock_path.display()));
                }

                cache_lock.initialized = true;

                if recovered_lock && let Err(error) = cleanup_cache_temporary_files(&root).await {
                    return Err(error);
                }

                return Ok(cache_lock);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if let Some(stale_owner) = stale_cache_lock_owner(&lock_path).await?
                    && remove_cache_lock_if_owned(&lock_path, &stale_owner).await?
                {
                    recovered_lock = true;
                    continue;
                }

                tokio::time::sleep(CACHE_LOCK_RETRY_DELAY).await;
            }
            Err(error) => {
                return Err(error)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("failed to acquire {}", lock_path.display()));
            }
        }
    }

    Err(miette!(
        "timed out waiting for cache write lock {}",
        lock_path.display()
    ))
}

async fn stale_cache_lock_owner(path: &Path) -> Result<Option<String>> {
    let owner = match tokio::fs::read_to_string(path).await {
        Ok(owner) => owner,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to inspect cache lock {}", path.display()));
        }
    };
    if cache_lock_owner_pid(&owner).is_some_and(|pid| !process_is_alive(pid)) {
        return Ok(Some(owner));
    }

    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to inspect cache lock {}", path.display()));
        }
    };
    let modified = metadata
        .modified()
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to inspect cache lock {}", path.display()))?;

    if !SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|age| age > CACHE_LOCK_STALE_AFTER)
    {
        return Ok(None);
    }

    Ok(Some(owner))
}

fn cache_lock_owner_pid(owner: &str) -> Option<u32> {
    owner.split(':').next()?.parse().ok().filter(|pid| *pid > 0)
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }

    let Ok(pid) = i32::try_from(pid) else {
        return true;
    };
    let result = unsafe { kill(pid, 0) };

    result == 0 || std::io::Error::last_os_error().kind() == std::io::ErrorKind::PermissionDenied
}

#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    use std::ffi::c_void;

    unsafe extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
        fn GetExitCodeProcess(process: *mut c_void, exit_code: *mut u32) -> i32;
        fn CloseHandle(object: *mut c_void) -> i32;
    }

    const ERROR_INVALID_PARAMETER: i32 = 87;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };

    if process.is_null() {
        return std::io::Error::last_os_error().raw_os_error() != Some(ERROR_INVALID_PARAMETER);
    }

    let mut exit_code = 0;
    let status_read = unsafe { GetExitCodeProcess(process, &raw mut exit_code) };
    let _close_result = unsafe { CloseHandle(process) };

    status_read == 0 || exit_code == STILL_ACTIVE
}

#[cfg(not(any(unix, windows)))]
fn process_is_alive(_pid: u32) -> bool {
    true
}

async fn cleanup_cache_temporary_files(root: &Path) -> Result<()> {
    cleanup_temporary_files_in_directory(root).await?;

    let mut entries = tokio::fs::read_dir(root)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to inspect {}", root.display()))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to inspect {}", root.display()))?
    {
        if entry
            .file_type()
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to inspect {}", entry.path().display()))?
            .is_dir()
        {
            cleanup_temporary_files_in_directory(&entry.path()).await?;
        }
    }

    Ok(())
}

async fn cleanup_temporary_files_in_directory(path: &Path) -> Result<()> {
    let mut entries = match tokio::fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to inspect {}", path.display()));
        }
    };

    while let Some(entry) = entries
        .next_entry()
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to inspect {}", path.display()))?
    {
        if entry
            .file_type()
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to inspect {}", entry.path().display()))?
            .is_file()
            && entry.file_name().to_string_lossy().ends_with(".tmp")
        {
            match tokio::fs::remove_file(entry.path()).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error)
                        .into_diagnostic()
                        .wrap_err_with(|| format!("failed to clean {}", entry.path().display()));
                }
            }
        }
    }

    Ok(())
}

fn cleanup_cache_temporary_files_sync(root: &Path) {
    cleanup_temporary_files_in_directory_sync(root);

    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
            cleanup_temporary_files_in_directory_sync(&entry.path());
        }
    }
}

fn cleanup_temporary_files_in_directory_sync(path: &Path) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|file_type| file_type.is_file())
            && entry.file_name().to_string_lossy().ends_with(".tmp")
        {
            let _cleanup_result = std::fs::remove_file(entry.path());
        }
    }
}

async fn remove_cache_lock_if_owned(path: &Path, owner: &str) -> Result<bool> {
    let current_owner = match tokio::fs::read_to_string(path).await {
        Ok(current_owner) => current_owner,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to inspect cache lock {}", path.display()));
        }
    };

    if current_owner != owner {
        return Ok(false);
    }

    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to release cache lock {}", path.display())),
    }
}

fn cache_lock_owner() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let counter = CACHE_LOCK_OWNER_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{}:{timestamp}:{counter}", std::process::id())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Write},
        path::Path,
        process::{Command, Stdio},
        time::Duration,
    };

    use super::{acquire, cache_root};

    #[tokio::test]
    async fn cache_lock_release_preserves_a_successor_lock() {
        let directory = tempfile::tempdir().unwrap();
        let lock_path = cache_root(directory.path()).join(".write.lock");
        let cache_lock = acquire(directory.path()).await.unwrap();

        tokio::fs::remove_file(&lock_path).await.unwrap();
        tokio::fs::write(&lock_path, "successor").await.unwrap();
        cache_lock.release().await.unwrap();

        assert_eq!(
            tokio::fs::read_to_string(&lock_path).await.unwrap(),
            "successor"
        );
    }

    #[tokio::test]
    async fn cancelled_cache_write_releases_its_lock() {
        let directory = tempfile::tempdir().unwrap();
        let cache_dir = directory.path().to_path_buf();
        let lock_path = cache_root(&cache_dir).join(".write.lock");
        let (acquired_sender, acquired_receiver) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let lock = acquire(&cache_dir).await.unwrap();

            acquired_sender.send(lock.owner.clone()).unwrap();
            std::future::pending::<()>().await;
        });

        let owner = acquired_receiver.await.unwrap();
        assert_eq!(tokio::fs::read_to_string(&lock_path).await.unwrap(), owner);
        task.abort();
        task.await.unwrap_err();

        assert!(!lock_path.exists());
    }

    #[tokio::test]
    async fn cache_lock_process_probe() {
        let Ok(cache_dir) = std::env::var("FONTMIN_CACHE_LOCK_PROBE_DIR") else {
            return;
        };
        let lock = acquire(Path::new(&cache_dir)).await.unwrap();
        let root = cache_root(Path::new(&cache_dir));
        let temporary_path = root.join(format!(".probe.{}.0.tmp", std::process::id()));

        tokio::fs::write(&temporary_path, b"incomplete")
            .await
            .unwrap();
        println!("CACHE_LOCK_ACQUIRED");
        std::io::stdout().flush().unwrap();
        tokio::time::sleep(Duration::from_secs(60)).await;
        lock.release().await.unwrap();
    }

    #[tokio::test]
    async fn interrupted_cache_writer_is_reclaimed_and_cleaned() {
        let tempdir = tempfile::tempdir().unwrap();
        let cache_dir = tempdir.path().join("cache");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("commands::build::cache::lock::tests::cache_lock_process_probe")
            .arg("--nocapture")
            .env("FONTMIN_CACHE_LOCK_PROBE_DIR", &cache_dir)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();

        loop {
            line.clear();
            assert_ne!(output.read_line(&mut line).unwrap(), 0);
            if line.contains("CACHE_LOCK_ACQUIRED") {
                break;
            }
        }

        child.kill().unwrap();
        child.wait().unwrap();

        let lock = acquire(&cache_dir).await.unwrap();
        let entries = std::fs::read_dir(cache_root(&cache_dir))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();

        assert!(
            !entries
                .iter()
                .any(|name| name.to_string_lossy().ends_with(".tmp"))
        );
        lock.release().await.unwrap();
        assert!(!cache_root(&cache_dir).join(".write.lock").exists());
    }
}
