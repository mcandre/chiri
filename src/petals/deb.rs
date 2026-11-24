use tempfile;
use xarxarxar;

use std::fs;
use std::path;

use crate::{
    Attributes, Chiri, Pkg, add_dir_with_root, add_file_with_root,
    error::{ChiriError, ResultExt},
    petal::Petal,
};

impl Petal {
    /// emit_deb generates Debian family packages.
    ///
    /// Requirements:
    ///
    /// * pkg.petal.chipset present, nonblank
    /// * pkg.petal.copyright present, nonblank
    /// * pkg.petal.license present, nonblank
    /// * pkg.petal.maintainer present, nonblank
    /// * pkg.petal.description present, nonblank
    pub fn emit_deb(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        let blank = String::new();
        let optional = String::from("optional");
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let chipset = petal.get("chipset").unwrap_or(&blank);
        let copyright = petal.get("copyright").unwrap_or(&blank);
        let license = petal.get("license").unwrap_or(&blank);
        let maintainer = petal.get("maintainer").unwrap_or(&blank);
        let priority = petal.get("priority").unwrap_or(&optional);
        let description = petal.get("description").unwrap_or(&blank);
        let dependencies = petal.get("dependencies").unwrap_or(&blank);
        let url = petal.get("url").unwrap_or(&blank);
        let dest = pkg.dest.clone().unwrap_or_default();

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

        if maintainer.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.maintainer blank/missing",
            )));
        }

        if description.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.description blank/missing",
            )));
        }

        if priority.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.priority blank",
            )));
        }

        let t = tempfile::tempdir().with_path("<tmp>")?;
        let t_pth = t.path();

        //
        // Generate control.tar.zst
        //

        let control_artifact_pth = t_pth.join("control.tar.zst");
        let mut control_archive = xarxarxar::WriteArchive::new()
            .format(xarxarxar::ArchiveFormat::TarGnu)
            .compression(xarxarxar::CompressionFormat::Zstd)
            .open_file(&control_artifact_pth)?;

        let mut control = String::from("Package: ");
        control.push_str(name);
        control.push_str("\nVersion: ");
        control.push_str(version);

        if !rev.is_empty() {
            control.push('-');
            control.push_str(rev);
        }

        control.push_str("\nMaintainer: ");
        control.push_str(maintainer);
        control.push_str("\nArchitecture: ");
        control.push_str(chipset);
        control.push_str("\nPriority: ");
        control.push_str(priority);
        control.push_str("\nDescription: ");
        control.push_str(description);

        if !dependencies.is_empty() {
            control.push_str("\nDepends: ");
            control.push_str(dependencies);
        }

        if !url.is_empty() {
            control.push_str("\nHomepage: ");
            control.push_str(url);
        }

        control.push('\n');
        control_archive.add_file("control", control.as_bytes())?;

        let mut eula = String::from("Files: *");
        eula.push_str("\nCopyright: ");
        eula.push_str(copyright);
        eula.push_str("\nLicense: ");
        eula.push_str(license);
        eula.push('\n');

        control_archive.add_file("copyright", eula.as_bytes())?;
        control_archive.finish()?;

        //
        // Generate data.tar.zst
        //

        let data_artifact_pth = t_pth.join("data.tar.zst");
        let mut data_archive = xarxarxar::WriteArchive::new()
            .format(xarxarxar::ArchiveFormat::TarGnu)
            .compression(xarxarxar::CompressionFormat::Zstd)
            .open_file(&data_artifact_pth)?;

        let attributes = Attributes::default();

        for (target, source) in dest {
            let metadata = fs::metadata(&source).with_path(&source)?;
            let source_pth = path::Path::new(&source);
            let target_pth = path::Path::new(&target);

            if metadata.is_dir() {
                add_dir_with_root(&mut data_archive, source_pth, target_pth, &attributes)?;
            } else {
                add_file_with_root(&mut data_archive, source_pth, target_pth, &attributes)?;
            }
        }

        data_archive.finish()?;

        //
        // Generate DEB
        //

        let mut basename = format!("{name}_{version}");

        if !rev.is_empty() {
            basename.push('-');
            basename.push_str(rev);
        }

        if !chipset.is_empty() {
            basename.push('_');
            basename.push_str(chipset);
        }

        basename.push_str(".deb");

        let deb_artifacts_root = chiri.artifacts.clone().unwrap().join("deb");

        fs::create_dir_all(&deb_artifacts_root).with_path(&deb_artifacts_root)?;

        let artifact_filename = deb_artifacts_root.join(basename);

        let mut deb_archive = xarxarxar::WriteArchive::new()
            .format(xarxarxar::ArchiveFormat::Ar)
            .open_file(artifact_filename)?;

        deb_archive.add_file("debian-binary", b"2.0\n")?;

        add_file_with_root(
            &mut deb_archive,
            control_artifact_pth,
            path::Path::new("control.tar.zst"),
            &attributes,
        )?;

        add_file_with_root(
            &mut deb_archive,
            data_artifact_pth,
            path::Path::new("data.tar.zst"),
            &attributes,
        )?;

        deb_archive.finish()?;

        Ok(())
    }
}
