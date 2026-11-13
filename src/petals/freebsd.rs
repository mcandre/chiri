use hex;
use itertools::Itertools;
use libarchive2;
use sha2::Digest;

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path;

use crate::{
    Chiri, Pkg, add_dir_with_root, add_file_with_root, error::ChiriError, petal::Petal,
    recursive_file_size_bytes,
};

impl Petal {
    /// emit_freebsd generates FreeBSD family packages.
    ///
    /// Requirements:
    ///
    /// * pkg.petal.chipset present, nonblank
    /// * pkg.petal.maintainer present, nonblank
    /// * pkg.petal.summary present, nonblank
    /// * pkg.petal.description present, nonblank
    /// * pkg.petal.origin present, nonblank
    pub fn emit_freebsd(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        let blank = String::new();
        let prefix = String::from("/usr/local");
        let single = String::from("single");
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let chipset_colon_delimited = petal.get("chipset").unwrap_or(&blank);
        let license = petal.get("license").unwrap_or(&blank);
        let license_logic = petal.get("license_logic").unwrap_or(&single);
        let maintainer = petal.get("maintainer").unwrap_or(&blank);
        let origin = petal.get("origin").unwrap_or(&blank);
        let prefix = petal.get("prefix").unwrap_or(&prefix);
        let summary = petal.get("summary").unwrap_or(&blank);
        let description = petal.get("description").unwrap_or(&blank);
        let dependencies = petal.get("dependencies").unwrap_or(&blank);
        let url = petal.get("url").unwrap_or(&blank);
        let dest = pkg
            .dest
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|(target, source)| (format!("/{target}"), source))
            .collect::<BTreeMap<String, String>>();

        if chipset_colon_delimited.is_empty() {
            return Err(ChiriError::IOError(
                "petal.chipset blank/missing".to_string(),
            ));
        }

        let chipset_parts = chipset_colon_delimited.split(':').collect::<Vec<&str>>();

        if chipset_parts.len() < 2 {
            return Err(ChiriError::IOError(format!(
                "invalid FreeBSD chipset: {chipset_colon_delimited}"
            )));
        }

        let chipset_distro = chipset_parts[0];
        let chipset_version = chipset_parts[1];
        let chipset_abi = if chipset_parts.len() >= 3 {
            chipset_parts[2]
        } else {
            ""
        };

        if maintainer.is_empty() {
            return Err(ChiriError::IOError(
                "petal.maintainer blank/missing".to_string(),
            ));
        }

        if summary.is_empty() {
            return Err(ChiriError::IOError(
                "petal.summary blank/missing".to_string(),
            ));
        }

        if description.is_empty() {
            return Err(ChiriError::IOError(
                "petal.description blank/missing".to_string(),
            ));
        }

        if origin.is_empty() {
            return Err(ChiriError::IOError(
                "petal.origin blank/missing".to_string(),
            ));
        }

        //
        // Generate PKG
        //

        let mut artifacts_root_basename = String::from(chipset_distro);

        if chipset_version != "*" {
            artifacts_root_basename.push('-');
            artifacts_root_basename.push_str(chipset_version);
        }

        if !chipset_abi.is_empty() {
            artifacts_root_basename.push('-');
            artifacts_root_basename.push_str(chipset_abi);
        }

        let freebsd_artifacts_root = chiri
            .artifacts
            .clone()
            .unwrap()
            .join(artifacts_root_basename);
        fs::create_dir_all(&freebsd_artifacts_root)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        let mut basename = format!("{name}-{version}");

        if !rev.is_empty() {
            basename.push('_');
            basename.push_str(rev);
        }

        basename.push_str(".pkg");

        let artifact_filename = freebsd_artifacts_root.join(basename);
        let mut pkg_archive = libarchive2::WriteArchive::new()
            .format(libarchive2::ArchiveFormat::TarPax)
            .compression(libarchive2::CompressionFormat::Zstd)
            .open_file(artifact_filename)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        //
        // Generate +MANIFEST
        //

        let mut manifest = format!("name: {name}");
        manifest.push_str("\nversion: ");
        manifest.push_str(version);

        if !rev.is_empty() {
            manifest.push_str("\nrevision: ");
            manifest.push_str(rev);
            manifest.push('\n');
        }

        manifest.push_str("\nmaintainer: ");
        manifest.push_str(maintainer);
        manifest.push_str("\norigin: ");
        manifest.push_str(origin);
        manifest.push_str("\ncomment: ");
        manifest.push_str(summary);
        manifest.push_str("\ndesc: ");
        manifest.push_str(description);
        manifest.push_str("\narch: ");
        manifest.push_str(chipset_colon_delimited);
        manifest.push_str("\nprefix: ");
        manifest.push_str(prefix);

        if chipset_abi.is_empty() {
            manifest.push_str("\nNO_ARCH: yes");
        }

        if !dependencies.is_empty() {
            manifest.push_str("\ndeps: { ");
            manifest.push_str(dependencies);
            manifest.push_str(" }");
        }

        manifest.push_str("\nlicenselogic: ");
        manifest.push_str(license_logic);

        if !license.is_empty() {
            manifest.push_str("\nlicenses: [");
            manifest.push_str(license);
            manifest.push(']');
        }

        let mut total_size_b = 0u64;

        for source in dest.values() {
            let size_b = recursive_file_size_bytes(source)
                .map_err(|e| ChiriError::IOError(e.to_string()))?;
            total_size_b += size_b;
        }

        manifest.push_str("\nflatsize: ");
        manifest.push_str(&total_size_b.to_string());
        let mut directories_set = HashSet::<String>::new();

        for (target, source) in &dest {
            let target = path::Path::new(target);
            let source = path::Path::new(source);

            let metadata = source
                .metadata()
                .map_err(|e| ChiriError::IOError(e.to_string()))?;

            if !metadata.is_dir() {
                continue;
            }

            for entry in walkdir::WalkDir::new(source) {
                let entry = entry.map_err(|e| ChiriError::IOError(e.to_string()))?;

                let md = entry
                    .metadata()
                    .map_err(|e| ChiriError::IOError(e.to_string()))?;

                if !md.is_dir() {
                    continue;
                }

                let entry_pth = entry.path();
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

                directories_set.insert(entry_adopted_string);
            }
        }

        let directories_comma_separated: String = directories_set
            .iter()
            .map(|e| format!("\"{e}\": y"))
            .join(",");

        manifest.push_str("\ndirectories: { ");
        manifest.push_str(&directories_comma_separated);
        manifest.push_str(" }");

        let mut checksums_list = Vec::<String>::new();

        for (target, source) in &dest {
            let target = path::Path::new(target);
            let source = path::Path::new(source);

            let metadata = source
                .metadata()
                .map_err(|e| ChiriError::IOError(e.to_string()))?;

            if metadata.is_dir() {
                for entry in walkdir::WalkDir::new(source) {
                    let entry = entry.map_err(|e| ChiriError::IOError(e.to_string()))?;
                    let md = entry
                        .metadata()
                        .map_err(|e| ChiriError::IOError(e.to_string()))?;

                    if md.is_dir() {
                        continue;
                    }

                    if !md.is_file() {
                        return Err(ChiriError::IOError(format!(
                            "unsupported file system file type for path: {:?}",
                            source
                        )));
                    }

                    let entry_pth = entry.path();
                    let content =
                        fs::read(entry_pth).map_err(|e| ChiriError::IOError(e.to_string()))?;
                    let sha256_bytes = sha2::Sha256::digest(content);
                    let sha256_hex = hex::encode(sha256_bytes);
                    let target_entry_pathbuf = target.join(
                        entry_pth
                            .strip_prefix(source)
                            .map_err(|e| ChiriError::IOError(e.to_string()))?,
                    );
                    let target_entry_str =
                        target_entry_pathbuf
                            .to_str()
                            .ok_or(ChiriError::IOError(format!(
                                "unable to process path: {:?}",
                                target_entry_pathbuf
                            )))?;
                    let checksum = format!("\"{target_entry_str}\": \"{sha256_hex}\"");
                    checksums_list.push(checksum);
                }

                continue;
            }

            let content = fs::read(source).map_err(|e| ChiriError::IOError(e.to_string()))?;
            let sha256_bytes = sha2::Sha256::digest(content);
            let sha256_hex = hex::encode(sha256_bytes);
            let target_str = target.to_str().ok_or(ChiriError::IOError(format!(
                "unable to process path: {:?}",
                target
            )))?;
            let checksum = format!("\"{target_str}\": \"{sha256_hex}\"");
            checksums_list.push(checksum);
        }

        let checksums_comma_separated = checksums_list.join(",");

        manifest.push_str("\nfiles: { ");
        manifest.push_str(&checksums_comma_separated);
        manifest.push_str(" }");

        if !url.is_empty() {
            manifest.push_str("\nwww: ");
            manifest.push_str(url);
        }

        manifest.push('\n');

        pkg_archive
            .add_file("+MANIFEST", manifest.as_bytes())
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        for (target, source) in dest {
            let metadata = fs::metadata(&source).map_err(|e| ChiriError::IOError(e.to_string()))?;
            let source_pth = path::Path::new(&source);
            let target_pth = path::Path::new(&target);

            if metadata.is_dir() {
                add_dir_with_root(&mut pkg_archive, source_pth, target_pth)
                    .map_err(|e| ChiriError::IOError(e.to_string()))?;
            } else {
                add_file_with_root(&mut pkg_archive, source_pth, target_pth)
                    .map_err(|e| ChiriError::IOError(e.to_string()))?;
            }
        }

        pkg_archive
            .finish()
            .map_err(|e| ChiriError::IOError(e.to_string()))
    }
}
