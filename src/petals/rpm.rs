use tempfile;

use std::fs;
use std::path;

use crate::{Chiri, Pkg, error::ChiriError, petal::Petal};

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
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
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
            return Err(ChiriError::IOError(
                "petal.chipset blank/missing".to_string(),
            ));
        }

        if license.is_empty() {
            return Err(ChiriError::IOError(
                "petal.license blank/missing".to_string(),
            ));
        }

        if summary.is_empty() {
            return Err(ChiriError::IOError(
                "petal.summary blank/missing".to_string(),
            ));
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
                    return Err(ChiriError::IOError(format!(
                        "invalid rpm dependencies: {dependencies_comma_separated}"
                    )));
                }

                let (name, version) = (parts[0], parts[1]);

                if name.is_empty() {
                    return Err(ChiriError::IOError(format!(
                        "blank name in rpm dependencies: {dependencies_comma_separated}"
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

        let t = tempfile::tempdir().map_err(|e| ChiriError::IOError(e.to_string()))?;

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

        let copying_pth = t.path().join("COPYING");
        fs::write(&copying_pth, eula).map_err(|e| ChiriError::IOError(e.to_string()))?;
        let copying_entry_pth = format!("/usr/share/licenses/{name}/COPYING");
        let copying_options = rpm::FileOptions::new(copying_entry_pth);
        rpm_pkg
            .with_file(copying_pth, copying_options)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        for (target, source) in dest {
            let target = format!("/{target}");
            let target = path::Path::new(&target);
            let source = path::Path::new(&source);
            let metadata = fs::metadata(&source).map_err(|e| ChiriError::IOError(e.to_string()))?;

            if metadata.is_dir() {
                for entry in walkdir::WalkDir::new(source) {
                    let entry = entry.map_err(|e| ChiriError::IOError(e.to_string()))?;
                    let entry_pth = entry.path();

                    let md = entry
                        .metadata()
                        .map_err(|e| ChiriError::IOError(e.to_string()))?;

                    if md.is_dir() {
                        continue;
                    }

                    if !md.is_file() {
                        return Err(ChiriError::IOError(format!("unsupported file system file type for path: {:?}", entry_pth)));
                    }

                    let entry_adopted = target.join(
                        entry_pth
                            .strip_prefix(source)
                            .map_err(|e| ChiriError::IOError(e.to_string()))?,
                    );
                    let entry_adopted_string = entry_adopted
                        .components()
                        .collect::<path::PathBuf>()
                        .to_str()
                        .ok_or(ChiriError::IOError(format!(
                            "unable to process source path: {:?}",
                            entry_adopted
                        )))?
                        .to_string();

                    let options = rpm::FileOptions::new(&entry_adopted_string);
                    rpm_pkg
                        .with_file(entry_pth, options)
                        .map_err(|e| ChiriError::IOError(e.to_string()))?;
                }
            } else if metadata.is_file() {
                let target_pth = target
                    .to_str()
                    .ok_or(ChiriError::IOError(format!("unable to process path: {:?}", target)))?;
                let options = rpm::FileOptions::new(target_pth);
                rpm_pkg
                    .with_file(&source, options)
                    .map_err(|e| ChiriError::IOError(e.to_string()))?;
            } else {
                return Err(ChiriError::IOError(format!(
                    "unsupported file system file type for path: {:?}",
                    source
                )));
            }
        }

        let mut basename = format!("{name}-{version}");

        if !rev.is_empty() {
            basename.push('-');
            basename.push_str(rev);
        }

        if !chipset.is_empty() {
            basename.push('.');
            basename.push_str(chipset);
        }

        basename.push_str(".rpm");
        let rpm_artifacts_root = chiri.artifacts.clone().unwrap().join("rpm");

        fs::create_dir_all(&rpm_artifacts_root).map_err(|e| ChiriError::IOError(e.to_string()))?;
        let artifact_filename = rpm_artifacts_root.join(basename);

        rpm_pkg
            .build()
            .map_err(|e| ChiriError::IOError(e.to_string()))?
            .write_file(artifact_filename)
            .map_err(|e| ChiriError::IOError(e.to_string()))
    }
}
