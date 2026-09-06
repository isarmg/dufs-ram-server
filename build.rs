use std::{path::Path, process::Command};

fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let pointer_width = std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap_or_default();
    assert!(
        target_arch == "x86_64"
            && target_env == "gnu"
            && target_os == "linux"
            && pointer_width == "64",
        "dufs server supports only x86_64-unknown-linux-gnu (requested arch={target_arch}, os={target_os}, env={target_env}, pointer_width={pointer_width})"
    );

    println!("cargo:rerun-if-env-changed=DUFS_BUILD_GIT_SHA");
    emit_git_rerun_paths();
    println!("cargo:rustc-env=DUFS_BUILD_GIT_SHA={}", build_git_sha());
    embed_platform_assets();
}

fn embed_platform_assets() {
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("clients/web/dist");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = std::fs::read_dir(&root)
        .expect("build the Web platform before compiling the Server")
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    files.sort();
    assert!(root.join("platform.js").is_file() && root.join("platform.css").is_file());
    let mut generated = String::from("&[\n");
    for path in files {
        let metadata = std::fs::symlink_metadata(&path).unwrap();
        assert!(metadata.is_file() && !metadata.file_type().is_symlink());
        let name = path.file_name().unwrap().to_str().unwrap();
        if name == "platform.d.ts" {
            continue; // TypeScript's build-time declarations are not runtime assets.
        }
        assert!(
            name.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        );
        assert!(
            metadata.len() <= 256 * 1024,
            "platform asset exceeds native budget"
        );
        let content_type = match path.extension().and_then(|value| value.to_str()) {
            Some("js") => "application/javascript; charset=UTF-8",
            Some("css") => "text/css; charset=UTF-8",
            Some("woff2") => "font/woff2",
            Some("txt") => "text/plain; charset=UTF-8",
            _ => panic!("unsupported platform asset: {name}"),
        };
        generated.push_str(&format!(
            "EmbeddedAsset {{ name: {:?}, contents: include_bytes!({:?}), content_type: {:?} }},\n",
            format!("dist/{name}"),
            path.to_str().unwrap(),
            content_type
        ));
    }
    generated.push_str("]\n");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("platform-assets.rs"), generated).unwrap();
}

fn emit_git_rerun_paths() {
    // A linked worktree stores a small `.git` file in the checkout and keeps
    // HEAD plus the shared references elsewhere. Ask Git for those real paths
    // instead of assuming that `.git` is a directory below the package root.
    if Path::new(".git").is_file() {
        println!("cargo:rerun-if-changed=.git");
    }
    for name in ["HEAD", "refs", "packed-refs", "reftable"] {
        if let Some(path) = git_output(&["rev-parse", "--git-path", name]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    // Shared refs in a reftable repository live below GIT_COMMON_DIR, while a
    // linked worktree's `--git-path reftable` resolves its private ref stack.
    if let Some(common_dir) = git_output(&["rev-parse", "--git-common-dir"]) {
        println!(
            "cargo:rerun-if-changed={}",
            Path::new(&common_dir).join("reftable").display()
        );
    }
}

fn build_git_sha() -> String {
    if let Ok(value) = std::env::var("DUFS_BUILD_GIT_SHA") {
        let value = value.trim();
        if is_valid_git_sha(value) {
            return value.to_ascii_lowercase();
        }
        panic!("DUFS_BUILD_GIT_SHA must contain 7 to 64 hexadecimal characters");
    }

    git_output(&["rev-parse", "--short=12", "HEAD"])
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| is_valid_git_sha(value))
        .unwrap_or_else(|| "unknown".to_string())
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let output = String::from_utf8(output.stdout).ok()?;
    let output = output.strip_suffix('\n').unwrap_or(&output);
    let output = output.strip_suffix('\r').unwrap_or(output);
    (!output.is_empty() && !output.contains(['\n', '\r'])).then(|| output.to_owned())
}

fn is_valid_git_sha(value: &str) -> bool {
    (7..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
