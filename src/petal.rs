use serde::{self, Deserialize, Serialize};

use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::{ error::ChiriError, Chiri, Pkg };

/// Petal generates software packages for various package managers.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Petal {
    /// Tarball builds generic UNIX tape archives.
    #[default]
    Tarball,
}

impl Petal {
    /// bloom generates an installer.
    pub fn bloom(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        match self {
            Petal::Tarball => self.emit_tarball(pkg, chiri),
        }
    }
}

/// Dest maps destination file paths to source media file paths.
pub type Dest = BTreeMap<String, String>;
