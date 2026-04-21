use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() {
    ensure_sidecar_path_exists();
    tauri_build::build();
}

fn ensure_sidecar_path_exists() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let target = std::env::var("TARGET").expect("target triple");
    let sidecar_dir = manifest_dir.join("bin");
    let sidecar_path = sidecar_dir.join(sidecar_name(&target));

    println!("cargo:rerun-if-env-changed=TARGET");
    println!("cargo:rerun-if-changed={}", sidecar_path.display());

    if sidecar_path.exists() {
        return;
    }

    fs::create_dir_all(&sidecar_dir).expect("sidecar bin dir should be creatable");
    write_placeholder_sidecar(&sidecar_path);
}

fn sidecar_name(target: &str) -> String {
    if target.contains("windows") {
        format!("octopus-desktop-backend-{target}.exe")
    } else {
        format!("octopus-desktop-backend-{target}")
    }
}

fn write_placeholder_sidecar(path: &Path) {
    if cfg!(windows) {
        fs::write(path, []).expect("placeholder sidecar should be writable");
        return;
    }

    fs::write(
        path,
        "#!/bin/sh\n\
         echo 'octopus-desktop-backend sidecar has not been prepared. Run pnpm prepare:desktop-backend:sidecar.' >&2\n\
         exit 1\n",
    )
    .expect("placeholder sidecar should be writable");

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)
            .expect("placeholder sidecar metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("placeholder sidecar should be executable");
    }
}
