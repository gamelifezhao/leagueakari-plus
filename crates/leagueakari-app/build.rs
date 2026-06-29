use std::{env, fs, path::PathBuf};

fn main() {
    write_embedded_probe_source();
    tauri_build::build()
}

fn write_embedded_probe_source() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    let workspace_root = manifest_dir.join("../..");
    let probe_path = workspace_root
        .join("target")
        .join(profile)
        .join(probe_executable_name());
    let generated_path = out_dir.join("embedded_probe.rs");

    println!("cargo:rerun-if-changed={}", probe_path.display());

    let source = if probe_path.is_file() {
        format!(
            "const EMBEDDED_PROBE_BYTES: Option<&'static [u8]> = Some(include_bytes!(r#\"{}\"#));\n",
            probe_path.display()
        )
    } else {
        "const EMBEDDED_PROBE_BYTES: Option<&'static [u8]> = None;\n".to_string()
    };

    fs::write(generated_path, source).expect("write embedded probe source");
}

fn probe_executable_name() -> &'static str {
    if cfg!(windows) {
        "leagueakari-probe.exe"
    } else {
        "leagueakari-probe"
    }
}
