use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use fontmin_testing::ROBOTO;

pub(super) struct CliSandbox {
    directory: tempfile::TempDir,
}

impl CliSandbox {
    pub(super) fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("failed to create CLI test sandbox"),
        }
    }

    pub(super) fn new_in(parent: impl AsRef<Path>, prefix: &str) -> Self {
        Self {
            directory: tempfile::Builder::new()
                .prefix(prefix)
                .tempdir_in(parent)
                .expect("failed to create CLI test sandbox"),
        }
    }

    pub(super) fn command(&self) -> Command {
        let mut command = fontmin_command();
        command.current_dir(self.root());
        command
    }

    pub(super) fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root().join(relative)
    }

    pub(super) fn root(&self) -> &Path {
        self.directory.path()
    }

    pub(super) fn write(&self, relative: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.path(relative);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("failed to create CLI test fixture directory");
        }
        std::fs::write(&path, contents).expect("failed to write CLI test fixture");

        path
    }

    pub(super) fn write_roboto(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.write(relative, ROBOTO)
    }
}

pub(super) fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

pub(super) fn fontmin_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
}
