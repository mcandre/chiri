use msi::{self, Column, Insert, Value};
use tempfile;

use std::collections::BTreeMap;
use std::fs;
use std::path;

use crate::{
    Chiri, Pkg,
    error::{ChiriError, ResultExt},
    petal::Petal,
};

impl Petal {
    /// emit_msi generates Windows (MSI) packages.
    ///
    /// Requirements:
    ///
    /// * pkg.petal.product_code present, nonblank
    /// * pkg.petal.upgrade_code present, nonblank
    /// * pkg.petal.manufacturer present, nonblank
    pub fn emit_msi(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        let blank = String::new();
        let source_dir = String::from("SourceDir");
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let product_code = petal.get("product_code").unwrap_or(&blank);
        let upgrade_code = petal.get("upgrade_code").unwrap_or(&blank);
        let chipset = petal.get("chipset").unwrap_or(&blank);
        let copyright = petal.get("copyright").unwrap_or(&blank);
        let license = petal.get("license").unwrap_or(&blank);
        let maintainer = petal.get("maintainer").unwrap_or(&blank);
        let manufacturer = petal.get("manufacturer").unwrap_or(&blank);
        let summary = petal.get("summary").unwrap_or(&blank);
        let targetdir = petal.get("targetdir").unwrap_or(&source_dir);
        let dest = pkg.dest.clone().unwrap_or_default();

        if product_code.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.product_code blank/missing",
            )));
        }

        if upgrade_code.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.upgrade_code blank/missing",
            )));
        }

        if manufacturer.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.manufacturer blank/missing",
            )));
        }

        if targetdir.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.targetdir blank",
            )));
        }

        let msi_artifacts_root = chiri.artifacts.clone().unwrap().join("windows");
        fs::create_dir_all(&msi_artifacts_root).with_path(&msi_artifacts_root)?;

        let mut basename = format!("{name}-${version}");

        if !rev.is_empty() {
            basename.push('.');
            basename.push_str(rev);
        }

        basename.push('-');
        basename.push_str(chipset);
        basename.push_str(".msi");

        let artifact_filename = msi_artifacts_root.join(basename);
        let f = fs::File::create(&artifact_filename).with_path(&artifact_filename)?;
        let mut package =
            msi::Package::create(msi::PackageType::Installer, f).with_path(&artifact_filename)?;
        let info = package.summary_info_mut();
        info.set_arch(chipset);

        if !maintainer.is_empty() {
            info.set_author(maintainer);
        }

        if !summary.is_empty() {
            info.set_comments(summary);
        }

        let info_fields: BTreeMap<String, String> = [
            ("ProductName", name),
            ("ProductVersion", version),
            ("ProductCode", product_code),
            ("UpgradeCode", upgrade_code),
            ("Manufacturer", manufacturer),
            ("TARGETDIR", targetdir),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();

        let mut properties = Insert::into("Property");

        for (k, v) in info_fields {
            properties = properties.row(vec![Value::Str(k.to_string()), Value::Str(v.to_string())]);
        }

        let _ = package.insert_rows(properties);

        //
        // Generate schema
        //

        package
            .create_table(
                "Directory",
                vec![
                    Column::build("Directory").primary_key().id_string(72),
                    Column::build("Directory_Parent")
                        .nullable()
                        .formatted_string(72),
                    Column::build("DefaultDir").formatted_string(255),
                ],
            )
            .with_path(&artifact_filename)?;
        package
            .create_table(
                "Component",
                vec![
                    Column::build("Component").primary_key().id_string(72),
                    Column::build("ComponentId").nullable().formatted_string(38),
                    Column::build("Directory_").formatted_string(72),
                    Column::build("Attributes").int32(),
                    Column::build("Condition").nullable().formatted_string(255),
                    Column::build("KeyPath").nullable().formatted_string(72),
                ],
            )
            .with_path(&artifact_filename)?;
        package
            .create_table(
                "File",
                vec![
                    Column::build("File").primary_key().id_string(72),
                    Column::build("Component_").formatted_string(72),
                    Column::build("FileName").formatted_string(255),
                    Column::build("FileSize").int32(),
                    Column::build("Version").nullable().formatted_string(72),
                    Column::build("Language").nullable().formatted_string(20),
                    Column::build("Attributes").nullable().int32(),
                    Column::build("Sequence").int32(),
                ],
            )
            .with_path(&artifact_filename)?;
        package
            .create_table(
                "Media",
                vec![
                    Column::build("DiskId").primary_key().int16(),
                    Column::build("LastSequence").int32(),
                    Column::build("DiskPrompt").nullable().formatted_string(64),
                    Column::build("Cabinet").nullable().formatted_string(255),
                    Column::build("VolumeLabel").nullable().formatted_string(32),
                    Column::build("Source").nullable().formatted_string(72),
                ],
            )
            .with_path(&artifact_filename)?;
        package
            .create_table(
                "Feature",
                vec![
                    Column::build("Feature").primary_key().id_string(38),
                    Column::build("Feature_Parent")
                        .nullable()
                        .formatted_string(38),
                    Column::build("Title").nullable().formatted_string(64),
                    Column::build("Description")
                        .nullable()
                        .formatted_string(255),
                    Column::build("Display").nullable().int16(),
                    Column::build("Level").int16(),
                    Column::build("Directory_").nullable().formatted_string(72),
                    Column::build("Attributes").int32(),
                ],
            )
            .with_path(&artifact_filename)?;
        package
            .create_table(
                "FeatureComponents",
                vec![
                    Column::build("Feature_").primary_key().id_string(38),
                    Column::build("Component_")
                        .primary_key()
                        .formatted_string(72),
                ],
            )
            .with_path(&artifact_filename)?;

        //
        // Generate source media
        //

        let t = tempfile::tempdir().with_path("<tmp>")?;
        let t_pth = t.path();
        let name_dir = t_pth.join(name);
        fs::create_dir_all(&name_dir).with_path(&name_dir)?;

        //
        // Generate LICENSE.txt
        //

        if !copyright.is_empty() || !license.is_empty() {
            let mut eula = String::new();

            if !copyright.is_empty() {
                eula.push_str(copyright);
                eula.push('\n');
            }

            if !license.is_empty() {
                eula.push_str(license);
                eula.push('\n');
            }

            let license_pth = name_dir.join("LICENSE.txt");
            fs::write(&license_pth, eula).with_path(&license_pth)?;
        }

        for (target, source) in &dest {
            let target = name_dir.join(path::Path::new(&target));
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

        //
        // Generate CAB
        //

        let disk_id = 1i32;
        let sequence_id = 1i32;
        package
            .insert_rows(Insert::into("Media").row(vec![
                Value::Int(disk_id),
                Value::Int(sequence_id),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ]))
            .with_path(&artifact_filename)?;

        let dir0 = "Dir0";

        package
            .insert_rows(
                Insert::into("Directory")
                    .row(vec![
                        Value::Str(String::from("TARGETDIR")),
                        Value::Null,
                        Value::Str(String::from("SourceDir")),
                    ])
                    .row(vec![
                        Value::Str(String::from("ProgramFiles64Folder")),
                        Value::Str(String::from("TARGETDIR")),
                        Value::Str(String::from(".")),
                    ])
                    .row(vec![
                        Value::Str(dir0.to_string()),
                        Value::Str(String::from("ProgramFiles64Folder")),
                        Value::Str(name.to_string()),
                    ]),
            )
            .with_path(&artifact_filename)?;

        let mut target_dir_to_id: BTreeMap<String, String> =
            [(name.into(), dir0.into())].into_iter().collect();
        let mut dir_id = 1u64;

        //
        // Generate Feature
        //

        let main_feature = String::from("Complete");
        package
            .insert_rows(Insert::into("Feature").row(vec![
                Value::Str(main_feature.clone()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Int(1i32),
                Value::Str(dir0.to_string()),
                Value::Int(0i32),
            ]))
            .with_path(&artifact_filename)?;

        let attribute_file_uncompressed = 8192i32;
        let attribute_64_bit = 256i32;
        let mut file_id = 0u64;
        let mut component_id = 0u64;

        for entry in walkdir::WalkDir::new(&name_dir).sort_by_file_name() {
            let entry = entry?;
            let pth = entry.path();
            let pth_relative = pth.strip_prefix(&name_dir)?;

            if pth_relative.is_empty() {
                continue;
            }

            let pth_relative_string = pth_relative
                .to_str()
                .ok_or(ChiriError::UnrenderablePathError(
                    pth_relative.to_path_buf(),
                ))?
                .to_owned();
            let basename = pth
                .file_name()
                .ok_or(ChiriError::ParentLookupError(pth.to_path_buf()))?;
            let basename_str = basename
                .to_str()
                .ok_or(ChiriError::UnrenderablePathError(basename.into()))?;
            let parent = pth
                .parent()
                .ok_or(ChiriError::ParentLookupError(pth.to_path_buf()))?;
            let parent_relative = parent.strip_prefix(&name_dir)?;
            let parent_relative_string = parent_relative
                .to_str()
                .ok_or(ChiriError::UnrenderablePathError(
                    parent_relative.to_path_buf(),
                ))?
                .to_owned();
            let d_parent_id = target_dir_to_id
                .get(&parent_relative_string)
                .unwrap()
                .clone();
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                let d_id = format!("Dir{dir_id}");
                target_dir_to_id.insert(pth_relative_string, d_id.clone());
                dir_id += 1;

                package
                    .insert_rows(Insert::into("Directory").row(vec![
                        Value::Str(d_id),
                        Value::Str(d_parent_id.to_string()),
                        Value::Str(basename_str.to_string()),
                    ]))
                    .with_path(&artifact_filename)?;
            }

            if !metadata.is_file() {
                return Err(ChiriError::UnsupportedFileSystemFileTypeError(
                    pth.to_path_buf(),
                ));
            }

            //
            // Generate File
            //

            let f_id = format!("File{file_id}");
            file_id += 1;
            let c_id = format!("Component{component_id}");
            component_id += 1;

            package
                .insert_rows(Insert::into("File").row(vec![
                    Value::Str(f_id.clone()),
                    Value::Str(c_id.clone()),
                    Value::Str(basename_str.to_string()),
                    Value::Int(metadata.len() as i32),
                    Value::Null,
                    Value::Null,
                    Value::Int(attribute_file_uncompressed),
                    Value::Int(sequence_id),
                ]))
                .with_path(&artifact_filename)?;

            //
            // Generate Component
            //

            let c_uuid = format!("{{{}}}", uuid::Uuid::new_v4());

            package
                .insert_rows(Insert::into("Component").row(vec![
                    Value::Str(c_id.clone()),
                    Value::Str(c_uuid.to_string()),
                    Value::Str(parent_relative_string),
                    Value::Int(attribute_64_bit),
                    Value::Null,
                    Value::Str(f_id),
                ]))
                .with_path(&artifact_filename)?;

            //
            // Generate FeatureComponent
            //

            package
                .insert_rows(
                    Insert::into("FeatureComponents")
                        .row(vec![Value::Str(main_feature.clone()), Value::Str(c_id)]),
                )
                .with_path(&artifact_filename)?;
        }

        package.flush().with_path(artifact_filename)?;

        Ok(())
    }
}
