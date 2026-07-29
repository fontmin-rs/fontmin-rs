use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use fontmin::Asset;
use fontmin_config::{CssConfig, FontminConfig, OutputConfig};
use fontmin_fs::{contained_path, path_to_string, resolve_path};
use miette::{Context, IntoDiagnostic, Result, miette};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::output::BuildOutput;

mod lock;

const CACHE_SCHEMA_VERSION: &str = "v1";
const FONTMIN_VERSION: &str = env!("CARGO_PKG_VERSION");
static TEMPORARY_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) struct BuildCache {
    dir: PathBuf,
    enabled: bool,
}

pub(super) struct CacheKey(String);

impl BuildCache {
    pub(super) fn from_config(config: &FontminConfig, cwd: &Path) -> Self {
        Self {
            dir: resolve_path(cwd, &config.cache.dir),
            enabled: config.cache.enabled,
        }
    }

    pub(super) fn key_for_input(
        &self,
        input: &Path,
        contents: &[u8],
        config: &FontminConfig,
    ) -> Result<Option<CacheKey>> {
        if !self.enabled {
            return Ok(None);
        }

        let key = json!({
            "config": {
                "css": config.css,
                "delivery": config.delivery,
                "otf": config.otf,
                "outputs": config.outputs,
                "plugins": config.plugins,
                "preserveOriginal": config.preserve_original,
                "subset": config.subset,
            },
            "fontminVersion": FONTMIN_VERSION,
            "input": {
                "hash": sha256(contents),
                "path": path_to_string(input),
            },
            "schema": CACHE_SCHEMA_VERSION,
        });

        Ok(Some(CacheKey(hash_key(&key)?)))
    }

    pub(super) fn key_for_iconfont_inputs(
        &self,
        input_paths: &[PathBuf],
        assets: &[Asset],
        outputs: &[OutputConfig],
        css: &CssConfig,
        font_family: &str,
    ) -> Result<Option<CacheKey>> {
        if !self.enabled {
            return Ok(None);
        }

        let key = json!({
            "fontminVersion": FONTMIN_VERSION,
            "icons": assets
                .iter()
                .zip(input_paths)
                .map(|(asset, input)| {
                    json!({
                        "hash": sha256(&asset.contents),
                        "input": path_to_string(input),
                        "path": path_to_string(&asset.path),
                    })
                })
                .collect::<Vec<_>>(),
            "kind": "iconfont",
            "options": {
                "css": css,
                "fontFamily": font_family,
                "outputs": outputs,
            },
            "schema": CACHE_SCHEMA_VERSION,
        });

        Ok(Some(CacheKey(hash_key(&key)?)))
    }

    pub(super) async fn restore(&self, key: &CacheKey) -> Result<Option<Vec<BuildOutput>>> {
        read_cached_outputs(&self.dir, &key.0).await
    }

    pub(super) async fn store(&self, key: &CacheKey, outputs: &[BuildOutput]) -> Result<()> {
        write_cached_outputs(&self.dir, &key.0, outputs).await
    }
}

async fn read_cached_outputs(cache_dir: &Path, key: &str) -> Result<Option<Vec<BuildOutput>>> {
    let manifest_path = cache_manifest_path(cache_dir, key);
    let manifest = match tokio::fs::read_to_string(&manifest_path).await {
        Ok(manifest) => manifest,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to read {}", manifest_path.display()));
        }
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&manifest) else {
        return Ok(None);
    };

    if manifest["version"] != CACHE_SCHEMA_VERSION || manifest["key"] != key {
        return Ok(None);
    }

    let Some(records) = manifest["outputs"].as_array() else {
        return Ok(None);
    };
    let entry_dir = cache_entry_dir(cache_dir, key);
    let mut outputs = Vec::with_capacity(records.len());

    for record in records {
        let (Some(file_name), Some(cache_file_name)) = (
            record["fileName"].as_str(),
            record["cacheFileName"].as_str(),
        ) else {
            return Ok(None);
        };
        let cache_file = contained_path(&entry_dir, Path::new(cache_file_name), "cache file name")?;
        if !tokio::fs::try_exists(&cache_file).await.into_diagnostic()? {
            return Ok(None);
        }
        ensure_existing_path_within_root(&entry_dir, &cache_file).await?;
        let contents = match tokio::fs::read(&cache_file).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("failed to read {}", cache_file.display()));
            }
        };
        let file_name = contained_path(Path::new(""), Path::new(file_name), "cached output path")?;

        outputs.push(BuildOutput::from_cache(file_name, contents));
    }

    Ok(Some(outputs))
}

async fn write_cached_outputs(cache_dir: &Path, key: &str, outputs: &[BuildOutput]) -> Result<()> {
    let cache_lock = lock::acquire(cache_dir).await?;
    let result = write_cached_outputs_locked(cache_dir, key, outputs).await;
    let unlock_result = cache_lock.release().await;

    match (result, unlock_result) {
        (Err(error), _) | (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

async fn write_cached_outputs_locked(
    cache_dir: &Path,
    key: &str,
    outputs: &[BuildOutput],
) -> Result<()> {
    let entry_dir = cache_entry_dir(cache_dir, key);
    let mut records = Vec::with_capacity(outputs.len());

    tokio::fs::create_dir_all(&entry_dir)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", entry_dir.display()))?;

    for (index, output) in outputs.iter().enumerate() {
        let extension = output
            .file_name()
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("bin");
        let cache_file_name = format!("{index:03}.{extension}");
        let cache_file = entry_dir.join(&cache_file_name);

        atomic_write(&cache_file, output.contents()).await?;
        records.push(json!({
            "cacheFileName": cache_file_name,
            "fileName": path_to_string(output.file_name()),
        }));
    }

    let manifest = json!({
        "key": key,
        "outputs": records,
        "version": CACHE_SCHEMA_VERSION,
    });
    let manifest_path = cache_manifest_path(cache_dir, key);

    atomic_write(
        &manifest_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&manifest).into_diagnostic()?
        )
        .as_bytes(),
    )
    .await?;
    update_cache_index(cache_dir, key, outputs).await
}

async fn update_cache_index(cache_dir: &Path, key: &str, outputs: &[BuildOutput]) -> Result<()> {
    let index_path = cache_index_path(cache_dir);
    let mut index = match tokio::fs::read_to_string(&index_path).await {
        Ok(index) => serde_json::from_str::<Value>(&index).unwrap_or_else(|_| empty_cache_index()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => empty_cache_index(),
        Err(error) => {
            return Err(error)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to read {}", index_path.display()));
        }
    };

    if index["version"] != CACHE_SCHEMA_VERSION || !index["entries"].is_object() {
        index = empty_cache_index();
    }

    if let Some(entries) = index["entries"].as_object_mut() {
        entries.insert(
            key.into(),
            json!({
                "outputs": outputs
                    .iter()
                    .map(|output| path_to_string(output.file_name()))
                    .collect::<Vec<_>>(),
                "updatedAt": cache_timestamp(),
            }),
        );
    }

    let Some(root) = index_path.parent() else {
        return Err(miette!(
            "failed to determine cache root for {}",
            index_path.display()
        ));
    };

    tokio::fs::create_dir_all(root)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to create {}", root.display()))?;
    atomic_write(
        &index_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&index).into_diagnostic()?
        )
        .as_bytes(),
    )
    .await
}

fn empty_cache_index() -> Value {
    json!({
        "entries": {},
        "version": CACHE_SCHEMA_VERSION,
    })
}

fn hash_key(value: &Value) -> Result<String> {
    Ok(sha256(serde_json::to_vec(value).into_diagnostic()?))
}

fn cache_root(cache_dir: &Path) -> PathBuf {
    cache_dir.join(CACHE_SCHEMA_VERSION)
}

fn cache_index_path(cache_dir: &Path) -> PathBuf {
    cache_root(cache_dir).join("index.json")
}

fn cache_entry_dir(cache_dir: &Path, key: &str) -> PathBuf {
    cache_root(cache_dir).join(key)
}

fn cache_manifest_path(cache_dir: &Path, key: &str) -> PathBuf {
    cache_entry_dir(cache_dir, key).join("index.json")
}

async fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let file_name = path
        .file_name()
        .ok_or_else(|| miette!("failed to determine file name for {}", path.display()))?;
    let counter = TEMPORARY_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary_path = path.with_file_name(format!(
        ".{}.{}.{counter}.tmp",
        file_name.to_string_lossy(),
        std::process::id()
    ));

    if let Err(error) = tokio::fs::write(&temporary_path, contents).await {
        return Err(error)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to write {}", temporary_path.display()));
    }

    if let Err(error) = tokio::fs::rename(&temporary_path, path).await {
        let _cleanup_result = tokio::fs::remove_file(&temporary_path).await;

        return Err(error)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to replace {}", path.display()));
    }

    Ok(())
}

async fn ensure_existing_path_within_root(root: &Path, path: &Path) -> Result<()> {
    let canonical_root = tokio::fs::canonicalize(root)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve {}", root.display()))?;
    let canonical_path = tokio::fs::canonicalize(path)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to resolve {}", path.display()))?;

    if !canonical_path.starts_with(&canonical_root) {
        return Err(miette!(
            "cache file resolves outside its cache entry: {}",
            path.display()
        ));
    }

    Ok(())
}

fn cache_timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
        .to_string()
}

fn sha256(input: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(input);
    let mut hash = String::with_capacity(digest.len() * 2);

    for byte in digest {
        use std::fmt::Write as _;

        write!(&mut hash, "{byte:02x}").expect("writing a hash to a string cannot fail");
    }

    hash
}
