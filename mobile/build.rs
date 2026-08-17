use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    if !target.starts_with("aarch64") || !target.contains("android") {
        return;
    }

    let Some(ndk) = find_ndk() else {
        println!(
            "cargo:warning=Android NDK not found (set ANDROID_NDK_ROOT); \
             `__clear_cache` may stay unresolved at runtime"
        );
        return;
    };

    let prebuilt = ndk.join("toolchains").join("llvm").join("prebuilt");
    let host_dir = match (env::consts::OS, env::consts::ARCH) {
        ("windows", _) => "windows-x86_64",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", _) => "linux-x86_64",
        ("macos", "aarch64") => "darwin-arm64",
        ("macos", _) => "darwin-x86_64",
        _ => "windows-x86_64",
    };

    let clang_lib = prebuilt.join(host_dir).join("lib").join("clang");
    let builtins = fs::read_dir(&clang_lib)
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.join("lib").join("linux").is_dir())
                .max()
        })
        .map(|clang_dir| {
            clang_dir.join("lib").join("linux").join(
                "libclang_rt.builtins-aarch64-android.a",
            )
        });

    if let Some(path) = builtins.filter(|p| p.exists()) {
        println!("cargo:rustc-link-arg={}", path.display());
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn find_ndk() -> Option<PathBuf> {
    for var in ["ANDROID_NDK_ROOT", "ANDROID_NDK_HOME", "NDK_HOME"] {
        if let Ok(path) = env::var(var) {
            if ndk_ok(Path::new(&path)) {
                return Some(PathBuf::from(path));
            }
        }
    }

    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(sdk) = env::var(var) {
            let ndk_dir = Path::new(&sdk).join("ndk");
            if let Some(latest) = fs::read_dir(&ndk_dir)
                .ok()
                .and_then(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| ndk_ok(p))
                        .max()
                })
            {
                return Some(latest);
            }
        }
    }

    None
}

fn ndk_ok(path: &Path) -> bool {
    path.join("source.properties").exists()
        && path
            .join("toolchains")
            .join("llvm")
            .join("prebuilt")
            .is_dir()
}