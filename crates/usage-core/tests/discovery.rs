use std::path::PathBuf;
use usage_core::codex_candidates;

#[test]
fn windows_finds_native_executable_in_path_and_npm_vendor() {
    let candidates = codex_candidates(
        "windows",
        vec![PathBuf::from("tools")],
        Some(PathBuf::from("home")),
        Some(PathBuf::from("roaming")),
        "x86_64",
    );
    assert_eq!(candidates[0], PathBuf::from("tools/codex.exe"));
    assert!(candidates.contains(&PathBuf::from(
        "roaming/npm/node_modules/@openai/codex/vendor/x86_64-pc-windows-msvc/codex/codex.exe"
    )));
    assert!(candidates
        .iter()
        .all(|p| p.extension().is_some_and(|e| e == "exe")));
}

#[test]
fn linux_includes_user_local_and_cargo_installations() {
    let candidates = codex_candidates(
        "linux",
        vec![PathBuf::from("tools")],
        Some(PathBuf::from("home")),
        None,
        "x86_64",
    );
    assert_eq!(candidates[0], PathBuf::from("tools/codex"));
    assert!(candidates.contains(&PathBuf::from("home/.local/bin/codex")));
    assert!(candidates.contains(&PathBuf::from("home/.cargo/bin/codex")));
}
