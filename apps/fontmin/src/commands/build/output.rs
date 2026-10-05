use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::{Component, Path, PathBuf},
};

use super::super::output::atomic_write;
use fontmin::Asset;
use fontmin_fs::contained_path;
use miette::{Context, IntoDiagnostic, Result, miette};

// File synchronization is I/O-bound, independently of per-file conversion threads.
const OUTPUT_WRITE_BATCH_SIZE: usize = 4;

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
    let mut unique_paths = HashSet::with_capacity(outputs.len());
    let mut output_paths = Vec::with_capacity(outputs.len());

    for output in outputs {
        let output_path = contained_path(out_dir, output.file_name(), "output file name")?;
        let normalized_output_path = absolute_normalized_path(&output_path)?;

        if !unique_paths.insert(normalized_output_path) {
            return Err(miette!(
                "duplicate output path: {}",
                output.file_name().display()
            ));
        }

        output_paths.push(output_path);
    }

    tokio::fs::create_dir_all(out_dir)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", out_dir.display()))?;
    let canonical_root = tokio::fs::canonicalize(out_dir)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve {}", out_dir.display()))?;
    let mut prepared_parents = HashMap::from([(out_dir.to_path_buf(), canonical_root.clone())]);
    unique_paths.clear();

    for (output_path, output) in output_paths.iter_mut().zip(outputs) {
        let parent = output_path
            .parent()
            .ok_or_else(|| miette!("failed to determine parent for {}", output_path.display()))?;
        let canonical_parent = if let Some(prepared) = prepared_parents.get(parent) {
            prepared
        } else {
            let existing_ancestor = nearest_existing_ancestor(parent).await?;

            ensure_parent_within_root(&canonical_root, &existing_ancestor).await?;
            tokio::fs::create_dir_all(parent)
                .await
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
            let canonical_parent = ensure_parent_within_root(&canonical_root, parent).await?;

            prepared_parents
                .entry(parent.to_path_buf())
                .or_insert(canonical_parent)
        };
        let file_name = output_path.file_name().ok_or_else(|| {
            miette!(
                "failed to determine file name for {}",
                output_path.display()
            )
        })?;
        let resolved_path = canonical_parent.join(file_name);

        if !unique_paths.insert(resolved_path.clone()) {
            return Err(miette!(
                "duplicate output path: {}",
                output.file_name().display()
            ));
        }

        // Use the checked parent instead of following its symbolic link again per write.
        *output_path = resolved_path;
    }

    for (paths, outputs) in output_paths
        .chunks(OUTPUT_WRITE_BATCH_SIZE)
        .zip(outputs.chunks(OUTPUT_WRITE_BATCH_SIZE))
    {
        if writes_need_ordering(paths) {
            for (path, output) in paths.iter().zip(outputs) {
                write_output(path, output).await?;
            }

            continue;
        }

        let writes = std::array::from_fn(|index| async move {
            let Some(path) = paths.get(index) else {
                return Ok(());
            };

            write_output(path, &outputs[index]).await
        });

        finish_write_batch(writes).await?;
    }

    Ok(())
}

fn writes_need_ordering(paths: &[PathBuf]) -> bool {
    let mut folded_names = HashSet::with_capacity(paths.len());

    paths.iter().any(|path| {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };
        let folded_name = file_name.to_ascii_lowercase();

        // Preserve ordered overwrites on case-insensitive filesystems. Unicode names
        // can also alias through normalization, so conservatively serialize those batches.
        // Hidden .tmp outputs can overlap another write's temporary file namespace.
        !file_name.is_ascii()
            || (file_name.starts_with('.')
                && file_name
                    .rsplit_once('.')
                    .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("tmp")))
            || !folded_names.insert((path.parent(), folded_name))
    })
}

async fn write_output(path: &Path, output: &BuildOutput) -> Result<()> {
    reject_symbolic_link(path).await?;
    atomic_write(path, output.contents()).await
}

async fn finish_write_batch<Write>(writes: [Write; OUTPUT_WRITE_BATCH_SIZE]) -> Result<()>
where
    Write: Future<Output = Result<()>>,
{
    let [first, second, third, fourth] = writes;
    // Drain every in-flight write on error so atomic_write can remove its temporary file.
    let (first, second, third, fourth) = tokio::join!(first, second, third, fourth);

    for result in [first, second, third, fourth] {
        result?;
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

async fn ensure_parent_within_root(canonical_root: &Path, parent: &Path) -> Result<PathBuf> {
    let canonical_parent = tokio::fs::canonicalize(parent)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve {}", parent.display()))?;

    if !canonical_parent.starts_with(canonical_root) {
        return Err(miette!(
            "output path resolves outside its destination directory: {}",
            parent.display()
        ));
    }

    Ok(canonical_parent)
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
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use miette::miette;
    use tokio::sync::Barrier;

    use super::{
        BuildOutput, OUTPUT_WRITE_BATCH_SIZE, clean_output_directory, finish_write_batch,
        write_outputs, writes_need_ordering,
    };

    #[test]
    fn serializes_outputs_that_can_overlap_atomic_write_temporary_files() {
        for temporary_name in [".font.ttf.123.0.tmp", ".FONT.ttf.123.0.TMP"] {
            let paths = [PathBuf::from("font.ttf"), PathBuf::from(temporary_name)];

            assert!(writes_need_ordering(&paths));
        }
    }

    #[tokio::test]
    async fn write_batch_runs_concurrently_and_drains_peers_after_an_error() {
        let barrier = Barrier::new(OUTPUT_WRITE_BATCH_SIZE);
        let completed = AtomicUsize::new(0);
        let writes = std::array::from_fn(|index| {
            let barrier = &barrier;
            let completed = &completed;

            async move {
                barrier.wait().await;

                if index == 0 {
                    return Err(miette!("write failed"));
                }

                tokio::task::yield_now().await;
                completed.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        });

        let error = tokio::time::timeout(Duration::from_secs(5), finish_write_batch(writes))
            .await
            .expect("all writes should reach the barrier concurrently")
            .unwrap_err();

        assert_eq!(error.to_string(), "write failed");
        assert_eq!(
            completed.load(Ordering::Relaxed),
            OUTPUT_WRITE_BATCH_SIZE - 1
        );
    }

    #[tokio::test]
    async fn writes_multiple_batches_and_a_partial_batch_with_shared_parents() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        let assets = (0..=OUTPUT_WRITE_BATCH_SIZE * 2)
            .map(|index| {
                BuildOutput::from_cache(
                    PathBuf::from(format!("nested/{}/font-{index}.ttf", index % 2)),
                    format!("font-{index}").into_bytes(),
                )
            })
            .collect::<Vec<_>>();

        write_outputs(&output, &assets).await.unwrap();

        for asset in &assets {
            assert_eq!(
                tokio::fs::read(output.join(asset.file_name()))
                    .await
                    .unwrap(),
                asset.contents()
            );
        }
    }

    #[tokio::test]
    async fn preserves_write_order_for_case_and_unicode_filename_aliases() {
        for (first, second) in [
            ("font.ttf", "FONT.ttf"),
            ("σ.ttf", "ς.ttf"),
            ("é.ttf", "e\u{301}.ttf"),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let output = directory.path();
            tokio::fs::write(output.join(first), b"old").await.unwrap();
            let names_alias = tokio::fs::try_exists(output.join(second)).await.unwrap();
            let assets = [
                BuildOutput::from_cache(PathBuf::from(first), vec![1; 128 * 1024]),
                BuildOutput::from_cache(PathBuf::from(second), b"last".to_vec()),
            ];

            write_outputs(output, &assets).await.unwrap();

            let expected_first = if names_alias { &assets[1] } else { &assets[0] };
            assert_eq!(
                tokio::fs::read(output.join(first)).await.unwrap(),
                expected_first.contents()
            );
            assert_eq!(tokio::fs::read(output.join(second)).await.unwrap(), b"last");
        }
    }

    #[tokio::test]
    async fn rejects_normalized_duplicate_paths_before_creating_the_destination() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        let assets = [
            BuildOutput::from_cache(PathBuf::from("nested/font.ttf"), b"first".to_vec()),
            BuildOutput::from_cache(PathBuf::from("nested/./font.ttf"), b"second".to_vec()),
        ];

        let error = write_outputs(&output, &assets).await.unwrap_err();

        assert!(error.to_string().contains("duplicate output path"));
        assert!(!output.exists());
    }

    #[tokio::test]
    async fn failed_write_drains_its_batch_and_cleans_up_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        tokio::fs::create_dir_all(output.join("blocked.ttf"))
            .await
            .unwrap();
        let assets = [
            BuildOutput::from_cache(PathBuf::from("blocked.ttf"), b"blocked".to_vec()),
            BuildOutput::from_cache(PathBuf::from("first.ttf"), vec![1; 128 * 1024]),
            BuildOutput::from_cache(PathBuf::from("second.ttf"), vec![2; 128 * 1024]),
            BuildOutput::from_cache(PathBuf::from("third.ttf"), vec![3; 128 * 1024]),
            BuildOutput::from_cache(PathBuf::from("pending.ttf"), b"pending".to_vec()),
        ];

        let error = write_outputs(&output, &assets).await.unwrap_err();

        assert!(error.to_string().contains("failed to atomically replace"));
        for asset in &assets[1..OUTPUT_WRITE_BATCH_SIZE] {
            assert_eq!(
                tokio::fs::read(output.join(asset.file_name()))
                    .await
                    .unwrap(),
                asset.contents()
            );
        }
        assert!(!output.join("pending.ttf").exists());
        let mut entries = tokio::fs::read_dir(&output).await.unwrap();
        let mut entry_count = 0;
        while let Some(entry) = entries.next_entry().await.unwrap() {
            assert!(!entry.file_name().to_string_lossy().ends_with(".tmp"));
            entry_count += 1;
        }
        assert_eq!(entry_count, OUTPUT_WRITE_BATCH_SIZE);
    }

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

    #[cfg(unix)]
    #[tokio::test]
    async fn write_refuses_a_symbolic_link_output_without_changing_its_target() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        let target = directory.path().join("target.ttf");
        tokio::fs::create_dir_all(&output).await.unwrap();
        tokio::fs::write(&target, b"keep").await.unwrap();
        symlink(&target, output.join("font.ttf")).unwrap();
        let asset = BuildOutput::from_cache(PathBuf::from("font.ttf"), b"replace".to_vec());

        let error = write_outputs(&output, &[asset]).await.unwrap_err();

        assert!(error.to_string().contains("through symbolic link"));
        assert_eq!(tokio::fs::read(&target).await.unwrap(), b"keep");
        assert!(
            tokio::fs::symlink_metadata(output.join("font.ttf"))
                .await
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn write_rejects_aliased_output_paths_before_writing() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output");
        let nested = output.join("nested");
        tokio::fs::create_dir_all(&nested).await.unwrap();
        symlink(&nested, output.join("alias")).unwrap();
        let assets = [
            BuildOutput::from_cache(PathBuf::from("nested/font.ttf"), b"first".to_vec()),
            BuildOutput::from_cache(PathBuf::from("alias/font.ttf"), b"second".to_vec()),
        ];

        let error = write_outputs(&output, &assets).await.unwrap_err();

        assert!(error.to_string().contains("duplicate output path"));
        assert!(!nested.join("font.ttf").exists());
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
