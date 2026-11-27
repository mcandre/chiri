use appimage;

use std::fs;
use std::io;
use std::path;

use crate::{
    Chiri, DEFAULT_CACHE, Pkg,
    error::{ChiriError, ResultExt},
    petal::Petal,
};

/// appimage_runtimes_dir denotes the standard directory for caching AppImage runtimes
pub fn appimage_runtimes_dir() -> path::PathBuf {
    path::Path::new(DEFAULT_CACHE).join("runtimes")
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
        let appimage = String::from("appimage");
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let base_dir = petal.get("base_dir").unwrap_or(&appimage);
        let dist = petal.get("dist").unwrap_or(&blank);
        let categories_semicolon_separated = petal.get("categories").unwrap_or(&blank);
        let chipset = petal.get("chipset").unwrap_or(&blank);
        let copyright = petal.get("copyright").unwrap_or(&blank);
        let license = petal.get("license").unwrap_or(&blank);
        let executable = petal.get("executable").unwrap_or(&blank);
        let icon = petal.get("icon").unwrap_or(&blank);
        let dest = pkg.dest.clone().unwrap_or_default();

        if categories_semicolon_separated.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.categories blank/missing",
            )));
        }

        let categories = categories_semicolon_separated
            .split(";")
            .collect::<Vec<&str>>();

        if categories.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.categories empty",
            )));
        }

        if chipset.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.chipset blank/missing",
            )));
        }

        if copyright.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.copyright blank/missing",
            )));
        }

        if license.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.license blank/missing",
            )));
        }

        if executable.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.executable blank/missing",
            )));
        }

        if icon.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.icon blank/missing",
            )));
        }

        //
        // Generate runtime
        //

        let runtimes_dir = appimage_runtimes_dir();
        fs::create_dir_all(&runtimes_dir).with_path(&runtimes_dir)?;
        let runtime_basename = format!("runtime-{chipset}");
        let runtime_pth = runtimes_dir.join(runtime_basename);

        if !runtime_pth.exists() {
            let runtime_url = format!(
                "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-{chipset}"
            );

            eprintln!("info: caching runtime: {runtime_url}");
            let mut response = ureq::get(runtime_url).call()?;
            let mut body = response.body_mut().as_reader();
            let mut runtime_file = fs::File::create(&runtime_pth).with_path(&runtime_pth)?;
            let _ = io::copy(&mut body, &mut runtime_file).with_path(&runtime_pth)?;
        }

        let runtime_content_bytes = fs::read(&runtime_pth).with_path(&runtime_pth)?;
        let runtime = appimage::Runtime::try_from(runtime_content_bytes)?;

        //
        // Generate desktop
        //

        let mut desktop = appimage::DesktopEntry::new(name, appimage::DesktopCommand::new(name));
        desktop.categories = &categories;

        //
        // Generate AppDir
        //

        let t = tempfile::tempdir().with_path("<tmp>")?;
        let t_pth = t.path();
        let appdir_basename = format!("{name}.AppDir");
        let appdir_pth = t_pth.join(appdir_basename);
        let mut appdir = appimage::AppDir::create(appdir_pth)?;

        for (target, source) in dest {
            let target = path::Path::new(&target);
            let source = path::Path::new(&source);
            let target_app_path = appimage::AppPath::new(target)?;
            let metadata = source.metadata().with_path(source)?;

            if metadata.is_dir() {
                appdir.copy_directory(source, target_app_path)?;
            } else if metadata.is_file() {
                appdir.copy_file(source, target_app_path)?;
            } else {
                return Err(ChiriError::UnsupportedFileSystemFileTypeError(
                    source.to_path_buf(),
                ));
            }
        }

        let executable_app_path = appimage::AppPath::new(executable)?;
        let mut executable_app_file = appdir.file(executable_app_path)?;
        executable_app_file.set_apprun(appimage::AppRun::Script)?;
        let app_id = appimage::AppId::new(name)?;
        let icon = appimage::Icon::from_path(path::Path::new(icon))?;
        appdir.write_desktop_entry(&app_id, &desktop, icon)?;
        appdir.validate()?;

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

        if !dist.is_empty() {
            basename.push('.');
            basename.push_str(dist);
        }

        basename.push_str(".AppImage");

        let appimage_artifacts_root = chiri.artifacts.clone().unwrap().join(base_dir);
        fs::create_dir_all(&appimage_artifacts_root).with_path(&appimage_artifacts_root)?;
        let artifact_filename = appimage_artifacts_root.join(basename);

        if artifact_filename.exists() {
            fs::remove_file(&artifact_filename).with_path(&artifact_filename)?;
        }

        let _ = appimage::Packer::new(&appdir, &runtime).write_to(artifact_filename)?;

        Ok(())
    }
}
