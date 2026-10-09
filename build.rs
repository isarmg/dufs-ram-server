use std::{path::Path, process::Command};

fn main() {
    let tagging = include_str!("schema/tagging.sql");
    let tagging_fingerprint =
        xcss_sqlite::fingerprint_trusted_ddl(&[tagging]).expect("current tag schema fingerprint");
    println!("cargo:rustc-env=XCZS_TAG_SCHEMA_SHA256={tagging_fingerprint}");
    println!("cargo:rerun-if-changed=schema/tagging.sql");

    let schema = include_str!("schema/product.sql");
    let fingerprint =
        xcss_sqlite::fingerprint_trusted_ddl(&[schema]).expect("current XCZS schema fingerprint");
    println!("cargo:rustc-env=XCZS_CURRENT_SCHEMA_SHA256={fingerprint}");
    println!("cargo:rerun-if-changed=schema/product.sql");

    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let pointer_width = std::env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap_or_default();
    assert!(
        target_arch == "x86_64"
            && target_env == "gnu"
            && target_os == "linux"
            && pointer_width == "64",
        "xczs server supports only x86_64-unknown-linux-gnu (requested arch={target_arch}, os={target_os}, env={target_env}, pointer_width={pointer_width})"
    );

    println!("cargo:rerun-if-env-changed=XCZS_BUILD_GIT_SHA");
    println!("cargo:rerun-if-env-changed=XCSS_WEB_DIST");
    emit_git_rerun_paths();
    println!("cargo:rustc-env=XCZS_BUILD_GIT_SHA={}", build_git_sha());
    let lockfile = std::fs::read_to_string("Cargo.lock").expect("read Cargo.lock");
    let revisions: std::collections::BTreeSet<_> = lockfile
        .lines()
        .filter_map(|line| {
            line.strip_prefix("source = \"git+https://github.com/isarmg/xcss.git?rev=")
        })
        .map(|line| {
            let (requested, locked) = line
                .trim_end_matches('"')
                .split_once('#')
                .expect("Foundation source requires a locked commit");
            assert!(
                requested.len() == 40
                    && requested.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && requested == locked,
                "Foundation source must bind one exact full commit"
            );
            requested
        })
        .collect();
    assert!(
        revisions.len() == 1,
        "one exact Foundation Git revision is required"
    );
    println!(
        "cargo:rustc-env=XCSS_FOUNDATION_REVISION={}",
        revisions.iter().next().unwrap()
    );
    println!("cargo:rerun-if-changed=Cargo.lock");

    let root = std::env::var_os("XCSS_WEB_DIST")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web/runtime-dist")
        });
    xcss_web_assets::build::generate(root)
        .expect("build the Web platform with the Foundation builder before compiling Server");
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
    if let Ok(value) = std::env::var("XCZS_BUILD_GIT_SHA") {
        let value = value.trim();
        if value == "unbound" {
            return "unbound".to_owned();
        }
        if is_valid_git_sha(value) {
            return value.to_ascii_lowercase();
        }
        panic!("XCZS_BUILD_GIT_SHA must contain 7 to 64 hexadecimal characters");
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
