use libarchive2;

use std::fs;
use std::path;

use crate::{Chiri, Pkg, add_dir_with_root, add_file_with_root, error::ChiriError, petal::Petal};

impl Petal {
    /// emit_tarball generates generic UNIX tarballs.
    pub fn emit_tarball(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        let blank = String::new();
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let chipset = petal.get("chipset").unwrap_or(&blank);
        let dest = pkg.dest.clone().unwrap_or_default();
        let mut basename = format!("{name}-{version}");

        if !rev.is_empty() {
            basename.push('-');
            basename.push_str(rev);
        }

        if !chipset.is_empty() {
            basename.push('-');
            basename.push_str(chipset);
        }

        basename.push_str(".tgz");
        let tarball_artifacts_root = chiri.artifacts.clone().unwrap().join("tarball");
        fs::create_dir_all(&tarball_artifacts_root)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;
        let artifact_filename = tarball_artifacts_root.join(basename);
        let mut archive = libarchive2::WriteArchive::new()
            .format(libarchive2::ArchiveFormat::TarPax)
            .compression(libarchive2::CompressionFormat::Gzip)
            .open_file(artifact_filename)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        //
        // Generate EULA
        //
        let copyright = petal.get("copyright").unwrap_or(&blank);
        let license = petal.get("license").unwrap_or(&blank);
        let mut eula = String::new();

        if !copyright.is_empty() {
            eula.push_str("Copyright: ");
            eula.push_str(copyright);
            eula.push('\n');
        }

        if !license.is_empty() {
            eula.push_str("License: ");
            eula.push_str(license);
            eula.push('\n');
        }

        if !eula.is_empty() {
            let eula_target = format!("usr/share/doc/{name}/LICENSE");
            archive
                .add_file(eula_target, eula.as_bytes())
                .map_err(|e| ChiriError::IOError(e.to_string()))?;
        }

        for (target, source) in dest {
            let metadata = fs::metadata(&source).map_err(|e| ChiriError::IOError(e.to_string()))?;
            let source_pth = path::Path::new(&source);
            let target_pth = path::Path::new(&target);

            if metadata.is_dir() {
                add_dir_with_root(&mut archive, source_pth, target_pth)
                    .map_err(|e| ChiriError::IOError(e.to_string()))?;
            } else {
                add_file_with_root(&mut archive, source_pth, target_pth)
                    .map_err(|e| ChiriError::IOError(e.to_string()))?;
            }
        }

        archive
            .finish()
            .map_err(|e| ChiriError::IOError(e.to_string()))
    }
}
