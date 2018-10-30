
extern crate cmake;
#[macro_use] extern crate log;
#[macro_use] extern crate failure;
extern crate git2;
extern crate simplelog;

use std::path::{Path, PathBuf};
use failure::Error;

fn get_it(src_path: &Path) -> Result<bool, Error> {
    let updated = match git2::Repository::open(src_path) {
        Ok(repo) => {
            trace!("Found glslang repo");
            let mut remote = repo.find_remote("origin")?;
            remote.fetch(&["master"], None, None)?;
            let fh = repo.find_reference("FETCH_HEAD")?;
            let mut master = repo.find_branch("master", git2::BranchType::Local)?
                .into_reference();

            if fh != master {
                trace!("Updated");
                master.set_target(fh.target().unwrap(), "q")?;
                true
            } else {
                trace!("Up-to-date");
                false
            }
        }
        Err(_) => {
            trace!("Clone glslang repo");
            let _ = git2::Repository::clone("https://github.com/KhronosGroup/glslang.git", src_path)?;
            true
        }
    };

    Ok(updated)
}

fn build_it(src_path: &Path, force: bool) -> Result<(), Error> {
    let build_path = src_path.join("build");
    if !build_path.is_dir() {
        bail!("glslang build dir is not a dir");
    }
    if !build_path.exists() {
        std::fs::create_dir(&build_path)?;
    } else if !force {
        let target = build_path.join("bin/glslangValidator");
        if target.is_file() {
            trace!("Found glslangValidator");
            return Ok(());
        }
    }

    trace!("Build glslangValidator");
    std::env::set_current_dir(&build_path)?;

    cmake::Config::new(&src_path)
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("CMAKE_INSTALL_PREFIX", &build_path)
        .build();

    Ok(())
}

fn main() -> Result<(), Error> {
    simplelog::SimpleLogger::init(simplelog::LevelFilter::Debug, simplelog::Config::default())?;

    match std::env::var("RENDY_SHADER_GLSLANGVALIDATOR") {
        Ok(_) => {
            info!("Use custom glslangValidator binary");
        },
        Err(_) => {
            let out_dir = PathBuf::from(
                std::env::var("RENDY_SHADER_OUT_DIR").or(std::env::var("OUT_DIR"))?
            );
            let src_path = out_dir.join("glslang");
            let updated = get_it(&src_path)?;
            build_it(&src_path, updated)?;
        }
    };

    Ok(())
}

