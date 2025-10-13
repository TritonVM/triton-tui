use std::env;
use std::path::Path;

fn main() {
    let git_dir = maybe_get_git_dir();
    trigger_rebuild_if_head_or_some_relevant_ref_changes(git_dir);

    write_git_info(git_info());
}

fn maybe_get_git_dir() -> Option<String> {
    let git_output = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok();
    git_output.as_ref().and_then(|output| {
        std::str::from_utf8(&output.stdout)
            .ok()
            .and_then(|s| s.strip_suffix('\n').or_else(|| s.strip_suffix("\r\n")))
            .map(str::to_string)
    })
}

fn trigger_rebuild_if_head_or_some_relevant_ref_changes(git_dir: Option<String>) {
    let Some(git_dir) = git_dir else {
        return;
    };

    let git_path = Path::new(&git_dir);
    let refs_path = git_path.join("refs");
    if git_path.join("HEAD").exists() {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
    }
    if git_path.join("packed-refs").exists() {
        println!("cargo:rerun-if-changed={git_dir}/packed-refs");
    }
    if refs_path.join("heads").exists() {
        println!("cargo:rerun-if-changed={git_dir}/refs/heads");
    }
    if refs_path.join("tags").exists() {
        println!("cargo:rerun-if-changed={git_dir}/refs/tags");
    }
}

fn git_info() -> String {
    const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

    let git_info = std::process::Command::new("git")
        .args(["describe", "--always", "--tags", "--long", "--dirty"])
        .output()
        .ok();
    let git_info = git_info
        .as_ref()
        .and_then(|output| std::str::from_utf8(&output.stdout).ok().map(str::trim));

    let Some(git_info) = git_info else {
        return CARGO_PKG_VERSION.to_string();
    };
    let Some(git_info) = git_info.strip_prefix('v') else {
        return CARGO_PKG_VERSION.to_string();
    };
    let Some(git_info) = git_info.strip_prefix(CARGO_PKG_VERSION) else {
        println!("cargo:warning=CARGO_PKG_VERSION and \"git describe\" are out of sync.");
        return CARGO_PKG_VERSION.to_string();
    };
    let Some(git_info) = git_info.strip_prefix('-') else {
        return CARGO_PKG_VERSION.to_string();
    };

    // no commits ahead and clean repo: no additional metadata required
    if git_info.starts_with('0') && !git_info.ends_with("dirty") {
        return CARGO_PKG_VERSION.to_string();
    }

    format!("{CARGO_PKG_VERSION}+{git_info}")
}

fn write_git_info(info: String) {
    let rust_const = format!("pub(crate) const CURR_VERSION: &str = \"{info}\";");
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let file_path = Path::new(&out_dir).join("version.rs");
    std::fs::write(file_path, rust_const).unwrap();
}
