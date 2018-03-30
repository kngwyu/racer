use std::path::{Path, PathBuf};
use cargo::{Config, core::{TargetKind, Workspace}};
use cargo::ops::{resolve_ws_precisely, Packages};
use cargo::util::important_paths::find_project_manifest;

use core::Session;
use nameres::RUST_SRC_PATH;

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
    macro_rules! unwrap_cargo_res {
        ($r: expr) => {
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
    let manifest = unwrap_cargo_res!(find_project_manifest(from_path, "Cargo.toml"));
    let libname_tmp = libname.to_owned();
    let name_slice = &[libname_tmp];
    let config = unwrap_cargo_res!(Config::default());
    let ws = unwrap_cargo_res!(Workspace::new(&manifest, &config));
    // TODO: is is really collect? (or is Packages::All needed?)
    let specs = unwrap_cargo_res!(Packages::Packages(name_slice).into_package_id_specs(&ws));
    let (packages, _) =
        unwrap_cargo_res!(resolve_ws_precisely(&ws, None, &[], false, false, &specs));
    let libname_hyphened = {
        let tmp_str = libname.to_owned();
        tmp_str.replace("_", "-")
    };
    // TODO: is there any way faster than this?
    for package_id in packages.package_ids() {
        let package = unwrap_cargo_res!(packages.get(package_id));
        let targets = package.manifest().targets();
        let lib_target = targets.into_iter().find(|target| {
            if let TargetKind::Lib(_) = target.kind() {
                let name = target.name();
                name == libname || name == libname_hyphened
            } else {
                false
            }
        });
        if let Some(target) = lib_target {
            return Some(target.src_path().to_owned());
        }
    }
    warn!("[get_outer_crates] failed to find package");
    None
}
