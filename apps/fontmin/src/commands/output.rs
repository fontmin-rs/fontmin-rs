use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use miette::{Context, IntoDiagnostic, Result, miette};
use tokio::io::AsyncWriteExt;

static TEMPORARY_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) async fn write_file_atomically(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        tokio::fs::create_dir_all(parent)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }

    atomic_write(path, contents).await
}

pub(super) async fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let file_name = path
        .file_name()
        .ok_or_else(|| miette!("failed to determine file name for {}", path.display()))?;
    let (temporary_path, mut temporary_file) = loop {
        let counter = TEMPORARY_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary_path = path.with_file_name(format!(
            ".{}.{}.{counter}.tmp",
            file_name.to_string_lossy(),
            std::process::id()
        ));
        match tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary_path)
            .await
        {
            Ok(file) => break (temporary_path, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("failed to create {}", temporary_path.display()));
            }
        }
    };

    let write_result = async {
        temporary_file.write_all(contents).await?;
        temporary_file.sync_all().await?;
        drop(temporary_file);
        tokio::fs::rename(&temporary_path, path).await
    }
    .await;
    if let Err(error) = write_result {
        let _cleanup_result = tokio::fs::remove_file(&temporary_path).await;
        return Err(error)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to atomically replace {}", path.display()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write_file_atomically;

    #[tokio::test]
    async fn replaces_existing_files_without_leaving_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("nested/font.ttf");
        tokio::fs::create_dir_all(output.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&output, b"old").await.unwrap();

        write_file_atomically(&output, b"new").await.unwrap();

        assert_eq!(tokio::fs::read(&output).await.unwrap(), b"new");
        let mut entries = tokio::fs::read_dir(output.parent().unwrap()).await.unwrap();
        let mut entry_names = Vec::new();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            entry_names.push(entry.file_name());
        }
        assert_eq!(entry_names, ["font.ttf"]);
    }
}
