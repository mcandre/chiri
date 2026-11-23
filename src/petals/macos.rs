use xarxarxar;
use tempfile;

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path;
use std::time;

use crate::{
    Attributes, Chiri, Pkg, add_dir_header, add_dir_with_root, add_file_with_root, bom,
    error::{ChiriError, ResultExt},
    petal::Petal,
    recursive_file_size_bytes,
};

impl Petal {
    /// emit_macos generates macOS packages.
    ///
    /// Requirements:
    ///
    /// * pkg.petal.identifier present, nonblank
    pub fn emit_macos(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        let now = time::SystemTime::now();
        let blank = String::new();
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let chipset = petal.get("chipset").unwrap_or(&blank);
        let copyright = petal.get("copyright").unwrap_or(&blank);
        let identifier = petal.get("identifier").unwrap_or(&blank);
        let license = petal.get("license").unwrap_or(&blank);
        let mut uid = Some(0u32);

        if let Some(uid_string) = petal.get("uid") {
            uid = Some(uid_string.parse::<u32>().map_err(|_| {
                ChiriError::ValidationError(format!("invalid u32: uid: {uid_string}"))
            })?);
        }

        let mut gid = Some(0u32);

        if let Some(gid_string) = petal.get("gid") {
            gid = Some(gid_string.parse::<u32>().map_err(|_| {
                ChiriError::ValidationError(format!("invalid u32: gid: {gid_string}"))
            })?);
        }

        let uname = petal.get("uname").cloned();
        let gname = petal.get("gname").cloned();
        let dest = pkg.dest.clone().unwrap_or_default();

        if identifier.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.identifier blank/missing",
            )));
        }

        //
        // Generate base.pkg/
        //

        let t = tempfile::tempdir().with_path("<tmp>")?;
        let t_pth = t.path();
        let pkg_dir = t_pth.join("pkg");
        let base_pkg_dir = pkg_dir.join("base.pkg");
        fs::create_dir_all(&base_pkg_dir).with_path(&base_pkg_dir)?;

        //
        // Generate source media
        //

        let source_media_pth = t_pth.join("source-media");
        fs::create_dir_all(&source_media_pth).with_path(&source_media_pth)?;

        for (target, source) in &dest {
            let target = source_media_pth.join(path::Path::new(&target));
            let source = path::Path::new(&source);
            let metadata = source.metadata().with_path(source)?;

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
                    let entry_adoptive_parent = entry_adoptee
                        .parent()
                        .ok_or(ChiriError::ParentLookupError(entry_adoptee.to_path_buf()))?;
                    fs::create_dir_all(entry_adoptive_parent).with_path(entry_adoptive_parent)?;
                    fs::copy(entry_pth, &entry_adoptee).map_err(|e| ChiriError::CopyError {
                        source: e,
                        origin: entry_pth.to_path_buf(),
                        destination: entry_adoptee.to_path_buf(),
                    })?;
                }
            } else {
                let target_parent = target
                    .parent()
                    .ok_or(ChiriError::ParentLookupError(target.to_path_buf()))?;
                fs::create_dir_all(target_parent).with_path(target_parent)?;
                fs::copy(source, &target).map_err(|e| ChiriError::CopyError {
                    source: e,
                    origin: source.to_path_buf(),
                    destination: target.to_path_buf(),
                })?;
            }
        }

        let mtime = fs::FileTimes::new().set_modified(now);

        for entry in walkdir::WalkDir::new(&source_media_pth) {
            let entry = entry?;
            let entry_pth = entry.path();
            fs::set_times(entry_pth, mtime).with_path(entry_pth)?;
        }

        //
        // Generate Bom
        //

        let bom_filename = base_pkg_dir.join("Bom");
        bom::create_bom(&source_media_pth, uid, gid)?.write(&bom_filename)?;

        //
        // Generate Payload
        //

        let payload_filename = base_pkg_dir.join("Payload");
        let mut payload_archive = xarxarxar::WriteArchive::new()
            .format(xarxarxar::ArchiveFormat::CpioOdc)
            .compression(xarxarxar::CompressionFormat::None)
            .compression(xarxarxar::CompressionFormat::Gzip)
            .open_file(&payload_filename)?;

        let payload_attributes = Attributes {
            uid,
            gid,
            uname,
            gname,
            atime: None,
            mtime: Some(now),
            nlink: None,
        };

        for entry in walkdir::WalkDir::new(&source_media_pth).sort_by_file_name() {
            let mut payload_attribute = payload_attributes.clone();
            let entry = entry?;
            let entry_pth = entry.path();
            let metadata = entry_pth.metadata().with_path(entry_pth)?;
            let entry_pth_relative = entry_pth.strip_prefix(&source_media_pth)?;
            let entry_pth_relative_string =
                entry_pth_relative
                    .to_str()
                    .ok_or(ChiriError::UnrenderablePathError(
                        entry_pth_relative.to_path_buf(),
                    ))?;
            payload_attribute.nlink = Some(metadata.nlink() as u32);

            if entry_pth_relative_string.is_empty() {
                add_dir_header(&mut payload_archive, ".", &payload_attribute)?;
                continue;
            }

            let target = format!("./{entry_pth_relative_string}");

            if metadata.is_dir() {
                add_dir_header(&mut payload_archive, target, &payload_attribute)?;
                continue;
            }

            if metadata.is_file() {
                add_file_with_root(&mut payload_archive, entry_pth, target, &payload_attribute)?;
            }
        }

        payload_archive.finish()?;

        //
        // Generate PackageInfo
        //

        let mut total_size_b = 0u64;

        for source in dest.values() {
            total_size_b += recursive_file_size_bytes(source)?;
        }

        let metrics_kib = ((total_size_b as f64) / 1024.0f64).ceil() as u64;
        let metrics_files = walkdir::WalkDir::new(&source_media_pth)
            .into_iter()
            .collect::<Vec<_>>()
            .len() as u64;

        let mut packageinfo_string = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>");
        packageinfo_string.push_str(&format!("\n<pkg-info auth=\"root\" format-version=\"2\" identifier=\"{identifier}\" install-location=\"/\" version=\"{version}\">"));
        packageinfo_string.push_str(&format!(
            "\n    <payload installKBytes=\"{metrics_kib}\" numberOfFiles=\"{metrics_files}\" />"
        ));
        packageinfo_string.push_str("\n    <scripts/>");
        packageinfo_string.push_str("\n</pkg-info>");
        packageinfo_string.push('\n');

        let packageinfo_filename = base_pkg_dir.join("PackageInfo");
        fs::write(&packageinfo_filename, packageinfo_string).with_path(&packageinfo_filename)?;

        let resources_dir = pkg_dir.join("Resources");
        let should_generate_eula = !copyright.is_empty() || !license.is_empty();

        if should_generate_eula {
            //
            // Generate Resources
            //

            fs::create_dir_all(&resources_dir).with_path(&resources_dir)?;

            //
            // Generate license.html
            //

            let mut license_string = String::from("<!doctype html>");
            license_string.push_str("\n<html>");
            license_string.push_str("\n<head>");
            license_string.push_str("\n    <meta charset=\"utf-8\">");
            license_string
                .push_str("\n    <meta name=\"viewport\" content=\"width=device-width\">");
            license_string.push_str("\n    <title>License</title>");
            license_string.push_str("\n</head>");
            license_string.push_str("\n<body>");

            if !copyright.is_empty() {
                license_string.push_str(&format!("\n<p>{copyright}</p>"));
            }

            if !license.is_empty() {
                license_string.push_str(&format!("\n<p>{license}</p>"));
            }

            license_string.push_str("\n</body>");
            license_string.push_str("\n</html>");
            license_string.push('\n');

            let license_filename = resources_dir.join("license.html");
            fs::write(&license_filename, license_string).with_path(license_filename)?;
        }

        //
        // Generate Distribution
        //

        let authoring_tool = "com.github.mcandre.chiri";
        let authoring_tool_version = env!("CARGO_PKG_VERSION");
        let authoring_tool_build = "1";

        let mut distribution_string = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>");
        distribution_string.push_str(&format!("\n<installer-script minSpecVersion=\"1.000000\" authoringTool=\"{authoring_tool}\" authoringToolVersion=\"{authoring_tool_version}\" authoringToolBuild=\"{authoring_tool_build}\">"));
        distribution_string.push_str(&format!("\n    <title>{name}</title>"));

        if should_generate_eula {
            distribution_string
                .push_str("\n    <license file=\"license.html\" mime-type=\"text/html\" />");
        }

        distribution_string
            .push_str("\n    <options customize=\"never\" allow-external-scripts=\"no\" />");
        distribution_string.push_str("\n    <domains enable_anywhere=\"true\" />");
        distribution_string.push_str("\n    <domains enable_localSystem=\"true\" />");
        distribution_string.push_str("\n    <choices-outline>");
        distribution_string.push_str("\n        <line choice=\"choice1\" />");
        distribution_string.push_str("\n    </choices-outline>");
        distribution_string.push_str("\n    <choice id=\"choice1\" title=\"base\">");
        distribution_string.push_str(&format!(
            "\n        <pkg-ref id=\"{identifier}.base.pkg\" />"
        ));
        distribution_string.push_str("\n    </choice>");
        distribution_string.push_str(&format!("\n    <pkg-ref id=\"{identifier}.base.pkg\" installKBytes=\"{metrics_kib}\" version=\"{version}\" auth=\"Root\">#base.pkg</pkg-ref>"));
        distribution_string.push_str("\n</installer-script>");
        distribution_string.push('\n');

        let distribution_filename = pkg_dir.join("Distribution");
        fs::write(&distribution_filename, distribution_string).with_path(&distribution_filename)?;

        //
        // Generate PKG
        //

        let pkg_artifacts_root = chiri.artifacts.clone().unwrap().join("macos");
        fs::create_dir_all(&pkg_artifacts_root).with_path(&pkg_artifacts_root)?;

        let mut basename = String::from(name);

        if !chipset.is_empty() {
            basename.push('-');
            basename.push_str(chipset);
        }

        basename.push('-');
        basename.push_str(version);

        if !rev.is_empty() {
            basename.push('-');
            basename.push_str(rev);
        }

        basename.push_str(".pkg");

        let spec_attributes = Attributes {
            uid: Some(0u32),
            gid: Some(0u32),
            uname: Some(String::from("root")),
            gname: Some(String::from("root")),
            atime: None,
            mtime: Some(now),
            nlink: None,
        };

        let artifact_filename = pkg_artifacts_root.join(basename);
        let pkg_builder = xarxarxar::WriteArchive::new()
            .format(xarxarxar::ArchiveFormat::Xar)
            .compression(xarxarxar::CompressionFormat::None)
            .format_option(xarxarxar::FormatOption::XarCompressionNone);
        let mut pkg_archive = pkg_builder.open_file(artifact_filename)?;
        add_dir_header(&mut pkg_archive, "base.pkg", &spec_attributes)?;
        add_file_with_root(
            &mut pkg_archive,
            payload_filename,
            "base.pkg/Payload",
            &spec_attributes,
        )?;
        add_file_with_root(
            &mut pkg_archive,
            bom_filename,
            "base.pkg/Bom",
            &spec_attributes,
        )?;
        add_file_with_root(
            &mut pkg_archive,
            packageinfo_filename,
            "base.pkg/PackageInfo",
            &spec_attributes,
        )?;

        if should_generate_eula {
            add_dir_with_root(
                &mut pkg_archive,
                resources_dir,
                "Resources",
                &spec_attributes,
            )?;
        }

        add_file_with_root(
            &mut pkg_archive,
            distribution_filename,
            "Distribution",
            &spec_attributes,
        )?;

        pkg_archive.finish()?;

        Ok(())
    }
}
