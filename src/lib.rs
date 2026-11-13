//! chiri provides predicates for generating software packages.
extern crate toml;
extern crate walkdir;

use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path;

pub mod error;
pub mod petal;
pub mod petals;

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
        let s = toml::to_string_pretty(self)
            .map_err(|_| fmt::Error)?;
        write!(f, "{s}")
    }
}

impl Pkg {
    /// validate performs a basic data integrity check.
    ///
    /// Checks:
    ///
    /// * dest table present, nonempty
    pub fn validate(self) -> Result<(), error::ChiriError> {
        let dest = self.dest.unwrap_or_default();

        if dest.is_empty() {
            return Err(error::ChiriError::IOError("dest table missing/empty".to_string()));
        }

        Ok(())
    }

    /// bloom generates an installer.
    pub fn bloom(self, chiri: &Chiri) -> Result<(), error::ChiriError> {
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
        let s = toml::to_string_pretty(self)
            .map_err(|_| fmt::Error)?;
        write!(f, "{s}")
    }
}

impl Chiri {
    /// load generates a Chiri.
    pub fn load(pth: &str) -> Result<Self, error::ChiriError> {
        let toml_string = fs::read_to_string(pth)
            .map_err(|_| error::ChiriError::IOError(format!("unable to read file: {pth}")))?;
        let c: Chiri = toml::from_str(&toml_string)
            .map_err(|e| error::ChiriError::TOMLParseError(e.message().to_string()))?;
        Ok(c)
    }

    /// validates performs a basic data integrity check.
    ///
    /// Checks:
    ///
    /// * petal table present
    /// * petal.name present, nonblank
    /// * petal.version present, nonblank
    pub fn validate(self: &Chiri) -> Result<(), error::ChiriError> {
        let petal = self.petal.clone().unwrap_or_default();

        if petal.is_empty() {
            return Err(error::ChiriError::IOError("petal table missing".to_string()));
        }

        let blank = String::new();

        let name = petal.get("name").unwrap_or(&blank);

        if name.is_empty() {
            return Err(error::ChiriError::IOError("petal.name blank/missing".to_string()));
        }

        let version = petal.get("version").unwrap_or(&blank);

        if version.is_empty() {
            return Err(error::ChiriError::IOError("petal.version blank/missing".to_string()));
        }

        Ok(())
    }

    /// bloom generates install media.
    pub fn bloom(self: &mut Chiri) -> Result<(), error::ChiriError> {
        self.validate()?;

        let pkgs = self.pkg
            .clone()
            .unwrap_or(Vec::new());
        let artifacts = artifacts_dir();
        self.artifacts = Some(artifacts.clone());
        fs::create_dir_all(&artifacts)
            .map_err(|e| error::ChiriError::IOError(e.to_string()))?;

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
    pub fn clean(self: &Chiri) -> Result<(), error::ChiriError> {
        if path::Path::new(DEFAULT_CACHE).exists() {
            fs::remove_dir_all(DEFAULT_CACHE)
                .map_err(|e| error::ChiriError::IOError(e.to_string()))?;
        }

        Ok(())
    }
}
