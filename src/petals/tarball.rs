use std::fs;

use crate::{ error::ChiriError, petal::Petal, Chiri, Pkg};

impl Petal {
    /// emit_tarball generates generic UNIX tarballs.
    pub fn emit_tarball(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        let tarball_artifacts_root = chiri.artifacts
            .clone()
            .unwrap()
            .join("tarball");
        fs::create_dir_all(tarball_artifacts_root)
            .map_err(|e| ChiriError::IOError(e.to_string()))?;

        let mut basename = format!("{name}-{version}");

        if !revision.is_empty() {
            basename.push_str("-");
            basename.push_str(revision);
        }

        basename.push_str(".tgz");
        let tarball_pth = tarball_artifacts_root.join(basename);

        // ...

        Err(ChiriError::IOError("stub".to_string()))
    }
}
