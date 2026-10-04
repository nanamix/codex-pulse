fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("native/touchbar.m")
            .flag("-fobjc-arc")
            .compile("codex_touchbar");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rerun-if-changed=native/touchbar.m");
    }
    tauri_build::build();
}
