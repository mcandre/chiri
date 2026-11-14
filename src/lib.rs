//! chiri provides predicates for generating software packages.

use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;
use std::fmt;
use std::fs;

// Preserve metadata
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use std::path;
use std::time;

pub mod error;
pub mod petal;
pub mod petals;

use crate::error::{ChiriError, ResultExt};

/// CONFIGURATION_FILENAME denotes the file path to an optional TOML configuration file,
/// relative to the current working directory.
pub static CONFIGURATION_FILENAME: &str = "chiri.toml";

/// DEFAULT_CACHE denotes the default directory root for chiri managed data.
pub static DEFAULT_CACHE: &str = ".chiri";

/// artifcts_dir denotes the standard directory for install media artifacts.
pub fn artifacts_dir() -> path::PathBuf {
    path::Path::new(DEFAULT_CACHE).join("artifacts")
}

/// Pkg builds software packages.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pkg {
    /// kind denotes a package type.
    pub kind: petal::Petal,

    /// petal overrides global petal entries.
    pub petal: Option<BTreeMap<String, String>>,

    /// dest overrides global dest entries.
    pub dest: Option<petal::Dest>,
}

impl fmt::Display for Pkg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = toml::to_string_pretty(self).map_err(|_| fmt::Error)?;
        write!(f, "{s}")
    }
}

impl Pkg {
    /// validate performs a basic data integrity check.
    ///
    /// Checks:
    ///
    /// * dest table present, nonempty
    pub fn validate(self) -> Result<(), ChiriError> {
        let dest = self.dest.unwrap_or_default();

        if dest.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "dest table missing/empty",
            )));
        }

        Ok(())
    }

    /// bloom generates an installer.
    pub fn bloom(self, chiri: &Chiri) -> Result<(), ChiriError> {
        self.kind.bloom(&self, chiri)
    }
}

/// Chiri builds software packages.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Chiri {
    /// debug enables additional logs.
    pub debug: Option<bool>,

    /// petal supplies additional build configuration to Petals (requirements vary by distro).
    pub petal: Option<BTreeMap<String, String>>,

    /// dest maps destination file paths to source file paths.
    pub dest: Option<petal::Dest>,

    /// pkg collects the packages to be built.
    pub pkg: Option<Vec<Pkg>>,

    #[serde(skip)]
    /// artifacts caches the artifacts directory root.
    artifacts: Option<path::PathBuf>,
}

#[test]
fn test_chiri_codec() {
    let chiri: Chiri = Chiri::default();
    let chiri_toml: String = toml::to_string(&chiri).unwrap();
    let chiri2: Chiri = toml::from_str(&chiri_toml).unwrap();
    assert_eq!(chiri2, chiri);
}

impl fmt::Display for Chiri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = toml::to_string_pretty(self).map_err(|_| fmt::Error)?;
        write!(f, "{s}")
    }
}

impl Chiri {
    /// load generates a Chiri.
    pub fn load(pth: &str) -> Result<Self, ChiriError> {
        let pth = path::PathBuf::from(pth);
        let toml_string = fs::read_to_string(&pth).with_path(&pth)?;
        let c: Chiri = toml::from_str(&toml_string)?;
        Ok(c)
    }

    /// validates performs a basic data integrity check.
    ///
    /// Checks:
    ///
    /// * petal table present
    /// * petal.name present, nonblank
    /// * petal.version present, nonblank
    pub fn validate(self: &Chiri) -> Result<(), ChiriError> {
        let petal = self.petal.clone().unwrap_or_default();

        if petal.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal table missing",
            )));
        }

        let blank = String::new();
        let name = petal.get("name").unwrap_or(&blank);

        if name.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.name blank/missing",
            )));
        }

        let version = petal.get("version").unwrap_or(&blank);

        if version.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.version blank/missing",
            )));
        }

        Ok(())
    }

    /// bloom generates install media.
    pub fn bloom(self: &mut Chiri) -> Result<(), ChiriError> {
        self.validate()?;

        #[cfg(not(unix))]
        eprintln!("warning: lossy source media file system metadata in non-unix environments");

        let pkgs = self.pkg.clone().unwrap_or_default();
        let artifacts = artifacts_dir();
        self.artifacts = Some(artifacts.clone());
        fs::create_dir_all(&artifacts).with_path(&artifacts)?;

        for pkg in pkgs {
            let mut petal2 = self.petal.clone().unwrap_or_default();
            let pkg_petal = pkg.petal.clone().unwrap_or_default();
            petal2.extend(pkg_petal);

            let mut dest2 = self.dest.clone().unwrap_or_default();
            let pkg_dest = pkg.dest.clone().unwrap_or_default();
            dest2.extend(pkg_dest);

            let mut pkg2 = pkg.clone();
            pkg2.petal = Some(petal2);
            pkg2.dest = Some(dest2);

            if let Some(true) = self.debug {
                eprintln!("pkg: {:?}", pkg2);
            }

            pkg2.clone().validate()?;
            pkg2.bloom(self)?;
        }

        eprintln!("info: packages written to {:?}", artifacts);

        Ok(())
    }

    /// clean removes artifacts.
    pub fn clean(self: &Chiri) -> Result<(), ChiriError> {
        if path::Path::new(DEFAULT_CACHE).exists() {
            fs::remove_dir_all(DEFAULT_CACHE).with_path(DEFAULT_CACHE)?;
        }

        Ok(())
    }
}

/// add_dir_with_root appends a directory recursively with a custom root directory.
pub fn add_dir_with_root<S: AsRef<path::Path>, T: AsRef<path::Path>>(
    archive: &mut libarchive2::WriteArchive,
    source: S,
    target: T,
) -> Result<(), ChiriError> {
    let source = source.as_ref();
    let target = target.as_ref();
    let mut disk = libarchive2::ReadDisk::new()?;
    disk.set_behavior(libarchive2::ReadDiskFlags::NO_XATTR)?;

    // Preserve symlinks
    disk.set_symlink_mode(libarchive2::SymlinkMode::Physical)?;

    // Query unames/gnames
    disk.set_standard_lookup()?;

    disk.open(source)?;

    while let Some(disk_entry_mut) = disk.next_entry()? {
        if disk.can_descend() {
            disk.descend()?;
        }

        let orig_string =
            disk_entry_mut
                .as_entry()
                .pathname()
                .ok_or(ChiriError::UnrenderablePathError(path::PathBuf::from(
                    "<null>",
                )))?;

        let orig_pathbuf = path::PathBuf::from(orig_string);

        let rel_path = orig_pathbuf
            .strip_prefix(source)
            .unwrap_or(path::Path::new(""));

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
        disk_entry2.set_pathname(&custom_archive_path)?;
        disk_entry2.set_size(disk_entry.size());
        disk_entry2.set_perm(disk_entry.mode())?;
        disk_entry2.set_uid(disk_entry.uid().unwrap_or(0u64));
        disk_entry2.set_gid(disk_entry.gid().unwrap_or(0u64));
        disk_entry2.set_atime(disk_entry.atime().unwrap_or(time::SystemTime::UNIX_EPOCH));
        disk_entry2.set_mtime(disk_entry.mtime().unwrap_or(time::SystemTime::UNIX_EPOCH));

        archive.write_header(&disk_entry2)?;

        if disk_entry.file_type() == libarchive2::FileType::RegularFile {
            let data = fs::read(&orig_pathbuf).with_path(&orig_pathbuf)?;
            archive.write_data(&data)?;
        }
    }

    Ok(())
}

/// add_file_with_root appends a file with a custom path.
pub fn add_file_with_root<S: AsRef<path::Path>, T: AsRef<path::Path>>(
    archive: &mut libarchive2::WriteArchive,
    source: S,
    target: T,
) -> Result<(), ChiriError> {
    let source = source.as_ref();
    let target = target.as_ref();

    let metadata = fs::metadata(source).with_path(source)?;

    let mut entry = libarchive2::EntryMut::new();
    entry.set_pathname(target)?;
    entry.set_file_type(std_filetype_to_libarchive(metadata.file_type()));
    entry.set_size(metadata.len() as i64);
    entry.set_perm(metadata.mode())?;
    entry.set_uid(metadata.uid() as u64);
    entry.set_gid(metadata.gid() as u64);
    entry.set_atime(i64_to_systemtime(metadata.atime())?);
    entry.set_mtime(i64_to_systemtime(metadata.mtime())?);
    archive.write_header(&entry)?;
    let data = fs::read(source).with_path(source)?;
    let _ = archive.write_data(&data)?;

    Ok(())
}

/// std_filetype_to_libarchive converts a standard FileType to a libarchive representation.
pub fn std_filetype_to_libarchive(ft: std::fs::FileType) -> libarchive2::FileType {
    if ft.is_dir() {
        return libarchive2::FileType::Directory;
    }

    if ft.is_file() {
        return libarchive2::FileType::RegularFile;
    }

    if ft.is_symlink() {
        return libarchive2::FileType::SymbolicLink;
    }

    libarchive2::FileType::Unknown
}

/// i64_to_systemtime converts UNIX epochs to SystemTime.
pub fn i64_to_systemtime(ts: i64) -> Result<time::SystemTime, ChiriError> {
    let duration = time::Duration::from_secs(ts.unsigned_abs());

    let ts_systemtime = if ts >= 0 {
        time::UNIX_EPOCH.checked_add(duration)
    } else {
        time::UNIX_EPOCH.checked_sub(duration)
    };

    ts_systemtime.ok_or(ChiriError::ValidationError(format!(
        "invalid timestamp: {ts}"
    )))
}

/// recursive_file_size_bytes calculates byte size of a given path.
pub fn recursive_file_size_bytes<P: AsRef<path::Path>>(pth: P) -> Result<u64, ChiriError> {
    let p = pth.as_ref();
    let metadata = p.metadata().with_path(p)?;

    if metadata.is_dir() {
        let mut size_b = 0u64;

        for entry in walkdir::WalkDir::new(p) {
            let entry = entry?;
            let md = entry.metadata()?;
            size_b += md.len();
        }

        return Ok(size_b);
    }

    if metadata.is_file() {
        return Ok(metadata.len());
    }

    Err(ChiriError::UnsupportedFileSystemFileTypeError(
        p.to_path_buf(),
    ))
}
