use tempfile;

use std::fs;
use std::path;

use crate::{
    Chiri, Pkg,
    error::{ChiriError, ResultExt},
    petal::Petal,
};

impl Petal {
    /// emit_rpm generates RHEL family packages.
    ///
    /// Requirements:
    ///
    /// * pkg.petal.chipset present, nonblank
    /// * pkg.petal.copyright present, nonblank
    /// * pkg.petal.license present, nonblank
    /// * pkg.petal.summary present, nonblank
    pub fn emit_rpm(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        let blank = String::new();
        let base_dir_rpm = String::from("rpm");
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let base_dir = petal.get("base_dir").unwrap_or(&base_dir_rpm);
        let dist = petal.get("dist").unwrap_or(&blank);
        let chipset = petal.get("chipset").unwrap_or(&blank);
        let copyright = petal.get("copyright").unwrap_or(&blank);
        let license = petal.get("license").unwrap_or(&blank);
        let summary = petal.get("summary").unwrap_or(&blank);
        let description = petal.get("description").unwrap_or(&blank);
        let maintainer = petal.get("maintainer").unwrap_or(&blank);
        let manufacturer = petal.get("manufacturer").unwrap_or(&blank);
        let dependencies_comma_separated = petal.get("dependencies").unwrap_or(&blank);
        let url = petal.get("url").unwrap_or(&blank);
        let dest = pkg.dest.clone().unwrap_or_default();

        if chipset.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.chipset blank/missing",
            )));
        }

        if license.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.license blank/missing",
            )));
        }

        if summary.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.summary blank/missing",
            )));
        }

        //
        // Generate RPM
        //

        let mut rpm_pkg = rpm::PackageBuilder::new(name, version, license, chipset, summary);

        if !rev.is_empty() {
            rpm_pkg.release(rev);
        }

        if !description.is_empty() {
            rpm_pkg.description(description);
        }

        if !dependencies_comma_separated.is_empty() {
            let dependencies_at_separated = dependencies_comma_separated.split(',');

            for dependency_at_separated in dependencies_at_separated {
                let parts = dependency_at_separated.split('@').collect::<Vec<&str>>();

                if parts.len() != 2 {
                    return Err(ChiriError::ValidationError(format!(
                        "invalid rpm dependencies: {dependencies_comma_separated}"
                    )));
                }

                let (name, version) = (parts[0], parts[1]);

                if name.is_empty() {
                    return Err(ChiriError::ValidationError(format!(
                        "blank package name in rpm dependencies: {dependencies_comma_separated}"
                    )));
                }

                let dependency_rpm = if version.is_empty() {
                    rpm::Dependency::any(name)
                } else {
                    rpm::Dependency::greater_eq(name, version)
                };

                rpm_pkg.requires(dependency_rpm);
            }
        }

        if !maintainer.is_empty() {
            rpm_pkg.packager(maintainer);
        }

        if !url.is_empty() {
            rpm_pkg.url(url);
        }

        if !manufacturer.is_empty() {
            rpm_pkg.vendor(manufacturer);
        }

        let build_config = rpm::BuildConfig::default().compression(rpm::CompressionType::Gzip);
        rpm_pkg.using_config(build_config);

        //
        // Generate COPYING
        //

        let mut eula = String::from("");

        if !copyright.is_empty() {
            eula.push_str(copyright);
            eula.push('\n');
        }

        eula.push_str(license);
        eula.push('\n');

        let t = tempfile::tempdir().with_path("<tmp>")?;
        let copying_pth = t.path().join("COPYING");

        fs::write(&copying_pth, eula).with_path(&copying_pth)?;
        let copying_entry_pth = format!("/usr/share/licenses/{name}/COPYING");
        let copying_options = rpm::FileOptions::new(copying_entry_pth);
        rpm_pkg.with_file(copying_pth, copying_options)?;

        for (target, source) in dest {
            let target = format!("/{target}");
            let target = path::Path::new(&target);
            let source = path::Path::new(&source);
            let metadata = fs::metadata(source).with_path(source)?;

            if metadata.is_dir() {
                for entry in walkdir::WalkDir::new(source).sort_by_file_name() {
                    let entry = entry?;
                    let entry_pth = entry.path();
                    let md = entry.metadata()?;

                    if md.is_dir() {
                        continue;
                    }

                    if !md.is_file() {
                        return Err(ChiriError::UnsupportedFileSystemFileTypeError(
                            entry_pth.to_path_buf(),
                        ));
                    }

                    let entry_adoptee = target.join(entry_pth.strip_prefix(source)?);
                    let entry_adoptee_string = entry_adoptee
                        .components()
                        .collect::<path::PathBuf>()
                        .to_str()
                        .ok_or(ChiriError::UnrenderablePathError(entry_adoptee))?
                        .to_string();

                    let options = rpm::FileOptions::new(&entry_adoptee_string);
                    rpm_pkg.with_file(entry_pth, options)?;
                }
            } else if metadata.is_file() {
                let target_pth = target
                    .to_str()
                    .ok_or(ChiriError::UnrenderablePathError(target.to_path_buf()))?;
                let options = rpm::FileOptions::new(target_pth);
                rpm_pkg.with_file(source, options)?;
            } else {
                return Err(ChiriError::UnsupportedFileSystemFileTypeError(
                    source.to_path_buf(),
                ));
            }
        }

        let mut basename = format!("{name}-{version}");

        if !rev.is_empty() {
            basename.push('-');
            basename.push_str(rev);
        }

        if !dist.is_empty() {
            basename.push('.');
            basename.push_str(dist);
        }

        if !chipset.is_empty() {
            basename.push('.');
            basename.push_str(chipset);
        }

        basename.push_str(".rpm");
        let rpm_artifacts_root = chiri.artifacts.clone().unwrap().join(base_dir);
        fs::create_dir_all(&rpm_artifacts_root).with_path(&rpm_artifacts_root)?;
        let artifact_filename = rpm_artifacts_root.join(basename);
        rpm_pkg.build()?.write_file(artifact_filename)?;

        Ok(())
    }
}
