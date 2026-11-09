use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use sha2::{Digest, Sha256};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let guest_dir = manifest_dir.join("guest");
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let target_dir = workspace_root.join("target").join("codemode-guest");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let out_wasm = out_dir.join("guest.wasm");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=guest/Cargo.toml");
    println!("cargo:rerun-if-changed=guest/Cargo.lock");
    println!("cargo:rerun-if-changed=guest/src");

    if out_wasm.exists() && !needs_rebuild(&out_wasm, &guest_dir, &manifest_dir.join("build.rs")) {
        log_sha256(&out_wasm);
        return;
    }

    let sdk = env::var("WASI_SDK_PATH").unwrap_or_else(|_| panic!("{}", sdk_bootstrap_help(None)));
    let sdk = PathBuf::from(sdk);
    let clang = sdk.join("bin").join("clang");
    let ar = sdk.join("bin").join("llvm-ar");
    let sysroot = sdk.join("share").join("wasi-sysroot");
    if !clang.exists() || !ar.exists() || !sysroot.exists() {
        panic!("{}", sdk_bootstrap_help(Some(&sdk)));
    }

    let status = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip1",
            "--target-dir",
            target_dir.to_str().expect("target dir"),
            "--manifest-path",
            guest_dir.join("Cargo.toml").to_str().expect("manifest"),
        ])
        .env("CC_wasm32_wasip1", &clang)
        .env("AR_wasm32_wasip1", &ar)
        .env(
            "CFLAGS_wasm32_wasip1",
            format!("--sysroot={}", sysroot.display()),
        )
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .status()
        .unwrap_or_else(|error| panic!("failed to spawn cargo for codemode guest: {error}"));
    if !status.success() {
        panic!(
            "codemode guest build failed. Check WASI_SDK_PATH (WASI SDK 24) and run: just codemode-guest"
        );
    }

    let built = target_dir
        .join("wasm32-wasip1")
        .join("release")
        .join("xylitol_codemode_guest.wasm");
    fs::copy(&built, &out_wasm).unwrap_or_else(|error| {
        panic!(
            "failed to copy {} to {}: {error}",
            built.display(),
            out_wasm.display()
        )
    });
    log_sha256(&out_wasm);
}

fn sdk_bootstrap_help(sdk_path: Option<&Path>) -> String {
    let mut help = match sdk_path {
        Some(path) => format!(
            "WASI_SDK_PATH={} does not look like WASI SDK 24 (need bin/clang, bin/llvm-ar, share/wasi-sysroot).\n",
            path.display()
        ),
        None => "WASI_SDK_PATH is not set.\n".to_string(),
    };
    help.push_str(concat!(
        "codemode guest compiles guest.wasm from source for wasm32-wasip1.\n",
        "One-shot bootstrap:\n",
        "    just wasi-sdk\n",
        "  (downloads WASI SDK 24 to ~/.local/share/wasi-sdk-24 and prints the export line)\n",
        "Manual bootstrap:\n",
        "    1. download wasi-sdk-24.0-<arch>-linux.tar.gz (or -macos.tar.gz) from\n",
        "       https://github.com/WebAssembly/wasi-sdk/releases/tag/wasi-sdk-24\n",
        "    2. tar -xzf <tarball> -C ~/.local/share/wasi-sdk-24 --strip-components=1\n",
        "    3. export WASI_SDK_PATH=\"$HOME/.local/share/wasi-sdk-24\"\n",
        "    4. rustup target add wasm32-wasip1\n",
        "Then re-run cargo build; this build script compiles the guest itself.",
    ));
    help
}

fn needs_rebuild(wasm: &Path, guest_dir: &Path, build_rs: &Path) -> bool {
    let Some(wasm_time) = modified(wasm) else {
        return true;
    };
    let mut newest = modified(build_rs);
    for path in [
        guest_dir.join("Cargo.toml"),
        guest_dir.join("Cargo.lock"),
        guest_dir.join("src"),
    ] {
        newest = newest.max(modified_tree(&path));
    }
    newest.is_some_and(|newest| newest > wasm_time)
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn modified_tree(path: &Path) -> Option<SystemTime> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.is_file() {
        return metadata.modified().ok();
    }
    let mut newest = metadata.modified().ok();
    for entry in fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        newest = newest.max(modified_tree(&entry.path()));
    }
    newest
}

fn log_sha256(path: &Path) {
    let bytes = fs::read(path).expect("read guest wasm");
    let digest = Sha256::digest(&bytes);
    let hash: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    println!("cargo:warning=codemode guest wasm sha256={hash}");
}
