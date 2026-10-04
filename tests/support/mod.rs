use std::{path::PathBuf, sync::OnceLock};
/// Build the test-only native peer once per integration-test process.
pub fn server() -> PathBuf {
    static SERVER: OnceLock<PathBuf> = OnceLock::new();
    SERVER
        .get_or_init(|| {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            assert!(std::process::Command::new(env!("CARGO"))
                .args(["build", "--offline", "--quiet", "-p", "usage-test-server"])
                .current_dir(root)
                .status()
                .unwrap()
                .success());
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(format!("usage-test-server{}", std::env::consts::EXE_SUFFIX))
        })
        .clone()
}
