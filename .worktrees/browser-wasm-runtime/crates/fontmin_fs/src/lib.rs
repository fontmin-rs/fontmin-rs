use std::path::{Path, PathBuf};

use fontmin_diagnostics::{FontminError, Result};

pub fn expand_input_paths(inputs: &[String], cwd: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for input in inputs {
        paths.extend(expand_input_path(input, cwd)?);
    }

    Ok(paths)
}

pub fn expand_input_path(input: &str, cwd: &Path) -> Result<Vec<PathBuf>> {
    if !is_glob_pattern(input) {
        return Ok(vec![resolve_path(cwd, input)]);
    }

    let pattern = path_to_string(&resolve_path(cwd, input));
    let mut paths = Vec::new();

    for entry in glob::glob(&pattern).map_err(|error| FontminError::config(error.to_string()))? {
        let path = entry.map_err(|error| FontminError::config(error.to_string()))?;

        if path.is_file() {
            paths.push(path);
        }
    }

    if paths.is_empty() {
        return Err(FontminError::config(format!(
            "input glob matched no files: {input}",
        )));
    }

    paths.sort();

    Ok(paths)
}

#[must_use]
pub fn is_glob_pattern(path: &str) -> bool {
    path.chars()
        .any(|character| matches!(character, '*' | '?' | '[' | ']' | '{' | '}'))
}

#[must_use]
pub fn resolve_path(cwd: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);

    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

#[must_use]
pub fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{expand_input_paths, is_glob_pattern, resolve_path};

    #[test]
    fn resolves_relative_paths_against_cwd() {
        let cwd = PathBuf::from("/workspace/project");

        assert_eq!(resolve_path(&cwd, "fonts/a.ttf"), cwd.join("fonts/a.ttf"));
    }

    #[test]
    fn keeps_absolute_paths_unchanged() {
        let cwd = PathBuf::from("/workspace/project");
        let absolute = PathBuf::from("/tmp/font.ttf");

        assert_eq!(resolve_path(&cwd, absolute.to_str().unwrap()), absolute);
    }

    #[test]
    fn detects_supported_glob_patterns() {
        assert!(is_glob_pattern("fonts/*.ttf"));
        assert!(is_glob_pattern("fonts/{a,b}.ttf"));
        assert!(!is_glob_pattern("fonts/roboto.ttf"));
    }

    #[test]
    fn expands_globs_to_sorted_files() {
        let tempdir = tempfile::tempdir().unwrap();
        let cwd = tempdir.path();
        let fonts = cwd.join("fonts");

        std::fs::create_dir_all(&fonts).unwrap();
        std::fs::write(fonts.join("b.ttf"), b"b").unwrap();
        std::fs::write(fonts.join("a.ttf"), b"a").unwrap();
        std::fs::create_dir_all(fonts.join("nested")).unwrap();

        let paths = expand_input_paths(&["fonts/*.ttf".into()], cwd).unwrap();
        let names: Vec<_> = paths
            .iter()
            .map(|path| path.file_name().unwrap().to_str().unwrap())
            .collect();

        assert_eq!(names, vec!["a.ttf", "b.ttf"]);
    }

    #[test]
    fn reports_empty_globs() {
        let tempdir = tempfile::tempdir().unwrap();
        let error = expand_input_paths(&["fonts/*.ttf".into()], tempdir.path()).unwrap_err();

        assert!(error.to_string().contains("input glob matched no files"));
    }
}
