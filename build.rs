extern crate bindgen;

use chrono::Utc;
use std::path::PathBuf;
use std::{env, process::Command};

const HEADER_NAME: &str = "src/plugin_api.h";

fn main() {
    println!("cargo:rerun-if-changed={}", HEADER_NAME);
    set_build_version();

    let bindings = bindgen::Builder::default()
        .header(HEADER_NAME)
        .rustified_enum("PluginApiID")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    if cfg!(debug_assertions) {
        bindings
            .write_to_file(out_path.join("../../../bindings.rs"))
            .expect("Couldn't write bindings to debug path");
    }
}

fn get_git_version() -> Option<String> {
    let commit_hash = Command::new("git")
        .args(["rev-parse", "--short=9", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            let commit_hash = String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_string());

            commit_hash
        });

    let is_dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(false);

    let dirty_suffix = if is_dirty { "-dirty" } else { "" };
    Some(format!("{}{}", commit_hash?, dirty_suffix))
}

fn get_rustc_version() -> Option<String> {
    Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
}

fn set_build_version() {
    let build_date = Utc::now().format("%Y-%m-%d").to_string();
    let git_version = get_git_version();
    let rustc_version = get_rustc_version();
    let cargo_pkg_version = env!("CARGO_PKG_VERSION").to_string();

    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
    println!(
        "cargo:rustc-env=GIT_VERSION={}",
        git_version.unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=RUSTC_VERSION={}",
        rustc_version.unwrap_or_default()
    );
    println!("cargo:rustc-env=CARGO_PKG_VERSION={}", cargo_pkg_version);
}
