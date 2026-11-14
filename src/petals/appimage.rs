use appimage;

use std::fs;
use std::io::Write;
use std::path;

use crate::{Chiri, DEFAULT_CACHE, Pkg, error::ChiriError, petal::Petal};

/// appimage_runtimes_dir denotes the standard directory for caching AppImage runtimes
pub fn appimage_runtimes_dir() -> path::PathBuf {
    path::Path::new(DEFAULT_CACHE).join("appimage_runtimes")
}

impl Petal {
    /// emit_appimage generates AppImage packages.
    ///
    /// Requirements:
    ///
    /// * pkg.petal.categories present, nonblank
    /// * pkg.petal.chipset present, nonblank
    /// * pkg.petal.copyright present, nonblank
    /// * pkg.petal.license present, nonblank
    /// * pkg.petal.executable present, nonblank
    /// * pkg.petal.icon present, nonblank
    pub fn emit_appimage(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        let blank = String::new();
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let categories_semicolon_separated = petal.get("categories").unwrap_or(&blank);
        let chipset = petal.get("chipset").unwrap_or(&blank);
        let copyright = petal.get("copyright").unwrap_or(&blank);
        let license = petal.get("license").unwrap_or(&blank);
        let executable = petal.get("executable").unwrap_or(&blank);
        let icon = petal.get("icon").unwrap_or(&blank);
        let dest = pkg.dest.clone().unwrap_or_default();

        if categories_semicolon_separated.is_empty() {
            return Err(ChiriError::IOError(
                "petal.categories blank/missing".to_string(),
            ));
        }

        let categories = categories_semicolon_separated
            .split(";")
            .collect::<Vec<&str>>();

        if categories.is_empty() {
            return Err(ChiriError::IOError("petal.categories empty".to_string()));
        }

        if chipset.is_empty() {
            return Err(ChiriError::IOError(
                "petal.chipset blank/missing".to_string(),
            ));
        }

        if copyright.is_empty() {
            return Err(ChiriError::IOError(
                "petal.copyright blank/missing".to_string(),
            ));
        }

        if license.is_empty() {
            return Err(ChiriError::IOError(
                "petal.license blank/missing".to_string(),
            ));
        }

        if executable.is_empty() {
            return Err(ChiriError::IOError(
                "petal.executable blank/missing".to_string(),
            ));
        }

        if icon.is_empty() {
            return Err(ChiriError::IOError("petal.icon blank/missing".to_string()));
        }

        //
        // Generate runtime
        //

        let runtimes_dir = appimage_runtimes_dir();
        fs::create_dir_all(&runtimes_dir).map_err(|e| ChiriError::IOError(e.to_string()))?;
        let runtime_basename = format!("runtime-{chipset}");
        let runtime_pth = runtimes_dir.join(runtime_basename);

        if !runtime_pth.exists() {
            let runtime_url = format!(
                "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-{chipset}"
            );
            eprintln!("info: caching runtime: {runtime_url}");
            let response = reqwest::blocking::get(runtime_url)
                .map_err(|e| ChiriError::IOError(e.to_string()))?;
            let runtime_content_bytes = response
                .bytes()
                .map_err(|e| ChiriError::IOError(e.to_string()))?;
            let mut runtime_file =
                fs::File::create(&runtime_pth).map_err(|e| ChiriError::IOError(e.to_string()))?;
            runtime_file
                .write_all(&runtime_content_bytes)
                .map_err(|e| ChiriError::IOError(e.to_string()))?;
        }

        let runtime_content_bytes =
            fs::read(runtime_pth).map_err(|e| ChiriError::IOError(e.to_string()))?;
        let runtime = appimage::Runtime::try_from(runtime_content_bytes)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        //
        // Generate desktop
        //

        let mut desktop = appimage::DesktopEntry::new(name, appimage::DesktopCommand::new(name));
        desktop.categories = &categories;

        //
        // Generate AppDir
        //

        let t = tempfile::tempdir().map_err(|e| ChiriError::IOError(e.to_string()))?;
        let t_pth = t.path();
        let appdir_basename = format!("{name}.AppDir");
        let appdir_pth = t_pth.join(appdir_basename);
        let mut appdir =
            appimage::AppDir::create(appdir_pth).map_err(|e| ChiriError::IOError(e.to_string()))?;

        for (target, source) in dest {
            let target = path::Path::new(&target);
            let source = path::Path::new(&source);
            let target_app_path =
                appimage::AppPath::new(target).map_err(|e| ChiriError::IOError(e.to_string()))?;
            let metadata = source
                .metadata()
                .map_err(|e| ChiriError::IOError(e.to_string()))?;

            if metadata.is_dir() {
                appdir
                    .copy_directory(source, target_app_path)
                    .map_err(|e| ChiriError::IOError(e.to_string()))?;
            } else if metadata.is_file() {
                appdir
                    .copy_file(source, target_app_path)
                    .map_err(|e| ChiriError::IOError(e.to_string()))?;
            } else {
                return Err(ChiriError::IOError(format!(
                    "unsupported file system file type for path: {:?}",
                    source
                )));
            }
        }

        let executable_app_path =
            appimage::AppPath::new(executable).map_err(|e| ChiriError::IOError(e.to_string()))?;
        let mut executable_app_file = appdir
            .file(executable_app_path)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;
        executable_app_file
            .set_apprun(appimage::AppRun::Script)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        let app_id = appimage::AppId::new(name).map_err(|e| ChiriError::IOError(e.to_string()))?;

        let icon = appimage::Icon::from_path(path::Path::new(icon))
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        appdir
            .write_desktop_entry(&app_id, &desktop, icon)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;
        appdir
            .validate()
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        //
        // Generate AppImage
        //

        let mut basename = format!("{name}-{version}");

        if !rev.is_empty() {
            basename.push('-');
            basename.push_str(rev);
        }

        basename.push('-');
        basename.push_str(chipset);
        basename.push_str(".AppImage");

        let appimage_artifacts_root = chiri.artifacts.clone().unwrap().join("appimage");
        fs::create_dir_all(&appimage_artifacts_root)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;
        let artifact_filename = appimage_artifacts_root.join(basename);

        let _ = appimage::Packer::new(&appdir, &runtime)
            .write_to(artifact_filename)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        Ok(())
    }
}
