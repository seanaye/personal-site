use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=tailwind.css");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let tailwind = topcoat::tailwind::BuildConfig::new()
        .input(manifest_dir.join("tailwind.css"))
        // CSS-native configuration keeps the build Node-free. When
        // TAILWIND_CLI is unset, Topcoat downloads and caches this exact CLI.
        .version("4.3.3");
    let tailwind = if env::var_os("TAILWIND_CLI").is_some() {
        println!("cargo:rerun-if-env-changed=TAILWIND_CLI");
        // Offline hermetic builds supply the same official CLI through Nix.
        tailwind.executable_env("TAILWIND_CLI")
    } else {
        tailwind
    };
    tailwind.render().expect("failed to build Tailwind CSS");
}
