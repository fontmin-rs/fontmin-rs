use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use fontmin::Asset;
use fontmin_fs::contained_path;
use miette::{Context, IntoDiagnostic, Result, miette};
use tokio::io::AsyncWriteExt;

static TEMPORARY_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) struct BuildOutput {
    contents: Vec<u8>,
    file_name: PathBuf,
}

impl BuildOutput {
    pub(super) fn from_asset(asset: Asset) -> Self {
        Self {
            contents: asset.contents,
            file_name: asset.path,
        }
    }

    pub(super) fn from_cache(file_name: PathBuf, contents: Vec<u8>) -> Self {
        Self {
            contents,
            file_name,
        }
    }

    pub(super) fn contents(&self) -> &[u8] {
        &self.contents
    }

    pub(super) fn file_name(&self) -> &Path {
        &self.file_name
    }
}

pub(super) async fn write_outputs(out_dir: &Path, outputs: &[BuildOutput]) -> Result<()> {
    let mut output_paths = HashSet::with_capacity(outputs.len());

    for output in outputs {
        let output_path = contained_path(out_dir, output.file_name(), "output file name")?;
        let normalized_output_path = absolute_normalized_path(&output_path)?;

        if !output_paths.insert(normalized_output_path) {
            return Err(miette!(
                "duplicate output path: {}",
                output.file_name().display()
            ));
        }
    }

    tokio::fs::create_dir_all(out_dir)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", out_dir.display()))?;

    for output in outputs {
        let output_path = contained_path(out_dir, output.file_name(), "output file name")?;
        let parent = output_path
            .parent()
            .ok_or_else(|| miette!("failed to determine parent for {}", output_path.display()))?;
        let existing_ancestor = nearest_existing_ancestor(parent).await?;

        ensure_parent_within_root(out_dir, &existing_ancestor).await?;
        tokio::fs::create_dir_all(parent)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
        ensure_parent_within_root(out_dir, parent).await?;
        reject_symbolic_link(&output_path).await?;

        atomic_write(&output_path, output.contents()).await?;
    }

    Ok(())
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

pub(super) async fn clean_output_directory(
    cwd: &Path,
    path: &Path,
    protected_paths: &[PathBuf],
) -> Result<()> {
    let cwd = absolute_normalized_path(cwd)?;
    let path = absolute_normalized_path(path)?;
    let path_is_configured_inside_cwd = path.starts_with(&cwd);
    let protected_paths = protected_paths
        .iter()
        .map(|protected| absolute_normalized_path(protected))
        .collect::<Result<Vec<_>>>()?;
    let protects_input = protected_paths
        .iter()
        .any(|protected| protected.starts_with(&path));

    if path.parent().is_none() || cwd.starts_with(&path) || protects_input {
        return Err(miette!(
            "refusing to clean output directory {} because it is the project directory, an input ancestor, or a filesystem root",
            path.display(),
        ));
    }

    if tokio::fs::try_exists(&path).await.into_diagnostic()? {
        let canonical_cwd = tokio::fs::canonicalize(&cwd)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to resolve {}", cwd.display()))?;
        let canonical_path = tokio::fs::canonicalize(&path)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to resolve {}", path.display()))?;
        let mut canonical_path_contains_input = false;

        for protected in &protected_paths {
            let canonical_protected = tokio::fs::canonicalize(protected)
                .await
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to resolve {}", protected.display()))?;

            if canonical_protected.starts_with(&canonical_path) {
                canonical_path_contains_input = true;
                break;
            }
        }

        if canonical_path.parent().is_none()
            || canonical_cwd.starts_with(&canonical_path)
            || canonical_path_contains_input
            || (path_is_configured_inside_cwd && !canonical_path.starts_with(&canonical_cwd))
        {
            return Err(miette!(
                "refusing to clean output directory {} because its resolved location is unsafe for project {}",
                path.display(),
                cwd.display()
            ));
        }
    }

    match tokio::fs::remove_dir_all(&path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to clean {}", path.display())),
    }
}

fn absolute_normalized_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().into_diagnostic()?.join(path)
    };
    let mut normalized = PathBuf::new();

    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(miette!(
                        "path escapes the filesystem root: {}",
                        path.display()
                    ));
                }
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }

    Ok(normalized)
}

async fn ensure_parent_within_root(root: &Path, parent: &Path) -> Result<()> {
    let canonical_root = tokio::fs::canonicalize(root)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve {}", root.display()))?;
    let canonical_parent = tokio::fs::canonicalize(parent)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve {}", parent.display()))?;

    if !canonical_parent.starts_with(&canonical_root) {
        return Err(miette!(
            "output path resolves outside its destination directory: {}",
            parent.display()
        ));
    }

    Ok(())
}

async fn nearest_existing_ancestor(path: &Path) -> Result<PathBuf> {
    let mut candidate = path.to_path_buf();

    loop {
        match tokio::fs::symlink_metadata(&candidate).await {
            Ok(_) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("failed to inspect {}", candidate.display()));
            }
        }

        if !candidate.pop() {
            return Err(miette!(
                "failed to find an existing ancestor for {}",
                path.display()
            ));
        }
    }
}

async fn reject_symbolic_link(path: &Path) -> Result<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(miette!(
            "refusing to write output through symbolic link {}",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to inspect {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{BuildOutput, clean_output_directory, write_outputs};

    #[tokio::test]
    async fn write_replaces_existing_output_without_leaving_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        let output_path = output.join("nested/font.ttf");
        tokio::fs::create_dir_all(output_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&output_path, b"old").await.unwrap();
        let asset = BuildOutput::from_cache(PathBuf::from("nested/font.ttf"), b"new".to_vec());

        write_outputs(&output, &[asset]).await.unwrap();

        assert_eq!(tokio::fs::read(&output_path).await.unwrap(), b"new");
        let mut entries = tokio::fs::read_dir(output_path.parent().unwrap())
            .await
            .unwrap();
        let mut entry_names = Vec::new();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            entry_names.push(entry.file_name());
        }
        assert_eq!(entry_names, ["font.ttf"]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn write_refuses_a_symlinked_parent_without_outside_side_effects() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        let outside = directory.path().join("outside");
        tokio::fs::create_dir_all(&output).await.unwrap();
        tokio::fs::create_dir_all(&outside).await.unwrap();
        symlink(&outside, output.join("linked")).unwrap();
        let asset =
            BuildOutput::from_cache(PathBuf::from("linked/nested/font.ttf"), b"font".to_vec());

        let error = write_outputs(&output, &[asset]).await.unwrap_err();

        assert!(error.to_string().contains("outside its destination"));
        assert!(!outside.join("nested").exists());
    }

    #[tokio::test]
    async fn clean_refuses_the_project_root() {
        let directory = tempfile::tempdir().unwrap();
        let sentinel = directory.path().join("sentinel.txt");
        tokio::fs::write(&sentinel, "keep").await.unwrap();

        let error = clean_output_directory(directory.path(), directory.path(), &[])
            .await
            .unwrap_err();

        assert!(error.to_string().contains("refusing to clean"));
        assert!(sentinel.exists());
    }

    #[tokio::test]
    async fn clean_refuses_an_input_ancestor() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        let input = output.join("font.ttf");
        tokio::fs::create_dir_all(&output).await.unwrap();
        tokio::fs::write(&input, "keep").await.unwrap();

        let error = clean_output_directory(directory.path(), &output, std::slice::from_ref(&input))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("refusing to clean"));
        assert!(input.exists());
    }
}
