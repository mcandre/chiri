// Preserve metadata
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use libarchive2;

use std::fs;
use std::path;
use std::time;

use crate::{ error::ChiriError, petal::Petal, Chiri, Pkg};

impl Petal {
    /// emit_tarball generates generic UNIX tarballs.
    pub fn emit_tarball(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        #[cfg(not(unix))]
        eprintln!("warning: metadata preservation requires a unix environment");

        let blank = String::new();
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let chipset = petal.get("chipset").unwrap_or(&blank);
        let mut basename = format!("{name}-{version}");

        if !rev.is_empty() {
            basename.push_str("-");
            basename.push_str(rev);
        }

        if !chipset.is_empty() {
            basename.push_str("-");
            basename.push_str(chipset);
        }

        basename.push_str(".tgz");
        let tarball_artifacts_root = chiri.artifacts
            .clone()
            .unwrap()
            .join("tarball");
        let dest = pkg.dest.clone().unwrap_or_default();
        fs::create_dir_all(&tarball_artifacts_root)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;
        let artifact_filename = tarball_artifacts_root.join(basename);
        let mut archive = libarchive2::WriteArchive::new()
            .format(libarchive2::ArchiveFormat::TarPax)
            .compression(libarchive2::CompressionFormat::Gzip)
            .open_file(artifact_filename)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

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

/// add_dir_with_root appends a directory recursively with a custom root directory.
fn add_dir_with_root(archive: &mut libarchive2::WriteArchive, source: &path::Path, target: &path::Path) -> Result<(), ChiriError> {
    let mut disk = libarchive2::ReadDisk::new()
        .map_err(|e| ChiriError::IOError(e.to_string()))?;
    disk.set_behavior(libarchive2::ReadDiskFlags::NO_XATTR)
        .map_err(|e| ChiriError::IOError(e.to_string()))?;

    // Preserve symlinks
    disk.set_symlink_mode(libarchive2::SymlinkMode::Physical).map_err(|e| ChiriError::IOError(e.to_string()))?;

    // Query unames/gnames
    disk.set_standard_lookup().map_err(|e| ChiriError::IOError(e.to_string()))?;

    disk.open(source).map_err(|e| ChiriError::IOError(e.to_string()))?;

    while let Some(disk_entry_mut) = disk.next_entry().map_err(|e| ChiriError::IOError(e.to_string()))? {
        if disk.can_descend() {
            disk
                .descend()
                .map_err(|e| ChiriError::IOError(e.to_string()))?;
        }

        let orig_path = path::PathBuf::from(
            disk_entry_mut
                .as_entry()
                .pathname()
                .ok_or(ChiriError::IOError("disk entry missing pathname".to_string()))?,
        );

        let rel_path = orig_path.strip_prefix(source).unwrap_or(path::Path::new(""));

        let custom_archive_path = if rel_path.as_os_str().is_empty() {
            target.to_path_buf()
        } else {
            target.join(rel_path)
        };

        let disk_entry = disk_entry_mut.as_entry();

        //
        // Strip libarchive creation header
        //
        let mut disk_entry2 = libarchive2::EntryMut::new();
        disk_entry2.set_file_type(disk_entry.file_type());
        disk_entry2.set_pathname(&custom_archive_path).map_err(|e| ChiriError::IOError(e.to_string()))?;
        disk_entry2.set_size(disk_entry.size());
        disk_entry2.set_perm(disk_entry.mode()).map_err(|e| ChiriError::IOError(e.to_string()))?;
        disk_entry2.set_uid(disk_entry.uid().unwrap_or(0u64));
        disk_entry2.set_gid(disk_entry.gid().unwrap_or(0u64));
        disk_entry2.set_atime(disk_entry.atime().unwrap_or(time::SystemTime::UNIX_EPOCH));
        disk_entry2.set_mtime(disk_entry.mtime().unwrap_or(time::SystemTime::UNIX_EPOCH));

        archive.write_header(&disk_entry2).map_err(|e| ChiriError::IOError(e.to_string()))?;

        if disk_entry.file_type() == libarchive2::FileType::RegularFile {
            let data = fs::read(orig_path).map_err(|e| ChiriError::IOError(e.to_string()))?;
            archive.write_data(&data).map_err(|e| ChiriError::IOError(e.to_string()))?;
        }
    }

    Ok(())
}

/// add_file_with_root appends a file with a custom path.
fn add_file_with_root(archive: &mut libarchive2::WriteArchive, source: &path::Path, target: &path::Path) -> Result<(), ChiriError> {
    let metadata = fs::symlink_metadata(source).map_err(|e| ChiriError::IOError(e.to_string()))?;
    let mut entry = libarchive2::EntryMut::new();
    entry.set_pathname(target).map_err(|e| ChiriError::IOError(e.to_string()))?;
    entry.set_size(metadata.len() as i64);
    entry.set_perm(metadata.mode()).map_err(|e| ChiriError::IOError(e.to_string()))?;
    entry.set_uid(metadata.uid() as u64);
    entry.set_gid(metadata.gid() as u64);
    entry.set_atime(i64_to_systemtime(metadata.atime())?);
    entry.set_mtime(i64_to_systemtime(metadata.mtime())?);
    archive.write_header(&entry).map_err(|e| ChiriError::IOError(e.to_string()))?;
    let data = fs::read(source).map_err(|e| ChiriError::IOError(e.to_string()))?;
    let _ = archive.write_data(&data).map_err(|e| ChiriError::IOError(e.to_string()))?;
    Ok(())
}

/// i64_to_systemtime converts UNIX epochs to SystemTime.
fn i64_to_systemtime(ts: i64) -> Result<time::SystemTime, ChiriError> {
    let duration = time::Duration::from_secs(ts.unsigned_abs());

    let ts_systemtime = if ts >= 0 {
        time::UNIX_EPOCH.checked_add(duration)
    } else {
        time::UNIX_EPOCH.checked_sub(duration)
    };

    ts_systemtime.ok_or(ChiriError::IOError(format!("invalid timestamp: {ts}")))
}
