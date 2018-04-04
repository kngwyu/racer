use cargo::Config;
use cargo::core::{TargetKind, Workspace, registry::PackageRegistry};
use cargo::ops::{resolve_ws_precisely, Packages};
use cargo::util::important_paths::find_project_manifest;
use core::Session;
use nameres::RUST_SRC_PATH;
use std::path::{Path, PathBuf};

/// get crate file from current path & crate name
pub fn get_crate_file(name: &str, from_path: &Path, session: &Session) -> Option<PathBuf> {
    debug!("get_crate_file {}, {:?}", name, from_path);

    if let Some(path) = get_outer_crates(name, from_path) {
        debug!("get_outer_crates returned {:?} for {}", path, name);
        return Some(path);
    }

    let srcpath = &*RUST_SRC_PATH;
    {
        // try lib<name>/lib.rs, like in the rust source dir
        let cratelibname = format!("lib{}", name);
        let filepath = srcpath.join(cratelibname).join("lib.rs");
        if filepath.exists() || session.contains_file(&filepath) {
            return Some(filepath);
        }
    }
    {
        // try <name>/lib.rs
        let filepath = srcpath.join(name).join("lib.rs");
        if filepath.exists() || session.contains_file(&filepath) {
            return Some(filepath);
        }
    }
    None
}

/// get module file from current path & crate name
pub fn get_module_file(name: &str, parentdir: &Path, session: &Session) -> Option<PathBuf> {
    {
        // try just <name>.rs
        let filepath = parentdir.join(format!("{}.rs", name));
        if filepath.exists() || session.contains_file(&filepath) {
            return Some(filepath);
        }
    }
    {
        // try <name>/mod.rs
        let filepath = parentdir.join(name).join("mod.rs");
        if filepath.exists() || session.contains_file(&filepath) {
            return Some(filepath);
        }
    }
    None
}

/// try to get outer crates
fn get_outer_crates(libname: &str, from_path: &Path) -> Option<PathBuf> {
    macro_rules! cargo_res {
        ($r:expr) => {
            match $r {
                Ok(val) => val,
                Err(err) => {
                    warn!("[get_outer_crates]: {}", err);
                    return None;
                }
            }
        };
    }
    debug!(
        "[get_outer_crates] lib name: {:?}, from_path: {:?}",
        libname, from_path
    );
    let manifest = cargo_res!(find_project_manifest(from_path, "Cargo.toml"));
    let config = cargo_res!(Config::default());
    let ws = cargo_res!(Workspace::new(&manifest, &config));
    let pkg = ws.current_opt()?;
    for dep in pkg.dependencies() {}
    let mut registry = cargo_res!(PackageRegistry::new(ws.config()));
    for (url, patches) in ws.root_patch() {
        cargo_res!(registry.patch(url, patches));
    }
    warn!("[get_outer_crates] failed to find package");
    None
}
