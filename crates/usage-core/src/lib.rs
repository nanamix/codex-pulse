mod model;
pub use model::*;
mod monitor;
mod protocol;
pub use monitor::{MonitorConfig, MonitorHandle};
pub use protocol::Client;

pub fn discover_codex() -> std::path::PathBuf {
    let paths = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    codex_candidates(
        std::env::consts::OS,
        paths,
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(Into::into),
        std::env::var_os("APPDATA").map(Into::into),
        std::env::consts::ARCH,
    )
    .into_iter()
    .find(|p| p.is_file())
    .unwrap_or_else(|| if cfg!(windows) { "codex.exe" } else { "codex" }.into())
}

/// Ordered installation locations. Windows uses native executables, never shell wrappers.
pub fn codex_candidates(
    os: &str,
    paths: Vec<std::path::PathBuf>,
    home: Option<std::path::PathBuf>,
    appdata: Option<std::path::PathBuf>,
    arch: &str,
) -> Vec<std::path::PathBuf> {
    let name = if os == "windows" {
        "codex.exe"
    } else {
        "codex"
    };
    let mut candidates: Vec<_> = paths.iter().map(|p| p.join(name)).collect();
    if let Some(home) = home {
        candidates.extend([
            home.join(".local/bin").join(name),
            home.join(".cargo/bin").join(name),
        ]);
    }
    if os == "windows" {
        let triple = if arch == "aarch64" {
            "aarch64-pc-windows-msvc"
        } else {
            "x86_64-pc-windows-msvc"
        };
        let mut npm_roots = paths;
        if let Some(appdata) = appdata {
            npm_roots.push(appdata.join("npm"));
        }
        for root in npm_roots {
            let packages = root.join("node_modules/@openai");
            for binary_dir in ["bin", "codex"] {
                candidates.push(
                    packages
                        .join("codex/vendor")
                        .join(triple)
                        .join(binary_dir)
                        .join("codex.exe"),
                );
                candidates.push(
                    packages
                        .join("codex/node_modules/@openai")
                        .join(if arch == "aarch64" {
                            "codex-win32-arm64"
                        } else {
                            "codex-win32-x64"
                        })
                        .join("vendor")
                        .join(triple)
                        .join(binary_dir)
                        .join("codex.exe"),
                );
            }
            let package = if arch == "aarch64" {
                "codex-win32-arm64"
            } else {
                "codex-win32-x64"
            };
            for binary_dir in ["bin", "codex"] {
                candidates.push(
                    packages
                        .join(package)
                        .join("vendor")
                        .join(triple)
                        .join(binary_dir)
                        .join("codex.exe"),
                );
            }
        }
    } else {
        if os == "macos" {
            candidates.push("/opt/homebrew/bin/codex".into());
        }
        candidates.extend(["/usr/local/bin/codex".into(), "/usr/bin/codex".into()]);
    }
    candidates
}
