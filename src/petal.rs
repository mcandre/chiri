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

    /// Deb builds Debian family packages.
    Deb,
}

impl Petal {
    /// bloom generates an installer.
    pub fn bloom(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        match self {
            Petal::Tarball => self.emit_tarball(pkg, chiri),
            Petal::Deb => self.emit_deb(pkg, chiri),
        }
    }
}

/// Dest maps destination file paths to source media file paths.
pub type Dest = BTreeMap<String, String>;
