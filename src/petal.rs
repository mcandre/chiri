use serde::{self, Deserialize, Serialize};

use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::{Chiri, Pkg, error::ChiriError};

/// Petal generates software packages for various package managers.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Petal {
    /// Tarball builds generic UNIX tape archives.
    #[default]
    Tarball,

    /// AppImage builds AppImage packages.
    #[serde(rename = "appimage")]
    AppImage,

    /// Deb builds Debian family packages.
    Deb,

    /// FreeBSD builds FreeBSD family packages.
    #[serde(rename = "freebsd")]
    FreeBSD,

    /// Rpm builds RHEL family packages.
    Rpm,
}

impl Petal {
    /// bloom generates an installer.
    ///
    /// Requirements:
    ///
    /// * pkg.petal.name present, nonblank
    /// * pkg.petal.version present, nonblank
    pub fn bloom(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        match self {
            Petal::AppImage => self.emit_appimage(pkg, chiri),
            Petal::Deb => self.emit_deb(pkg, chiri),
            Petal::FreeBSD => self.emit_freebsd(pkg, chiri),
            Petal::Rpm => self.emit_rpm(pkg, chiri),
            Petal::Tarball => self.emit_tarball(pkg, chiri),
        }
    }
}

/// Dest maps destination file paths to source media file paths.
pub type Dest = BTreeMap<String, String>;
