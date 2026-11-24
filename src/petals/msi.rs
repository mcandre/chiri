use msi::{self, ColumnBuilder, Insert, Value};
use tempfile;

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path;

use crate::{
    Attributes, Chiri, Pkg, add_dir_with_root, add_file_with_root,
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
        let manufacturer = petal.get("manufacturer").unwrap_or(&blank);
        let summary = petal.get("summary").unwrap_or(&blank);
        let targetdir = petal.get("targetdir").unwrap_or(&source_dir);
        let dest = pkg.dest.clone().unwrap_or_default();
        let t = tempfile::tempdir().with_path("<tmp>")?;
        let t_pth = t.path();

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

        let mut basename=format!("{name}-${version}");

        if !rev.is_empty() {
            basename.push('.');
            basename.push_str(rev);
        }

        basename.push('-');
        basename.push_str(chipset);
        basename.push_str(".msi");

        let artifact_filename = msi_artifacts_root.join(basename);
        let f = fs::File::create(artifact_filename)?;
        let mut package = msi::Package::create(msi::PackageType::Installer, f)?;
        let mut info = package.summary_info_mut();
        info.set_arch(chipset);
        info.set_author(maintainer);

        if !summary.is_empty() {
            info.set_comments(summary);
        }

        let mut properties = Insert::into("Property");

        let info_fields = btreemap! {
            "ProductName" => name,
            "ProductVersion" => version,
            "ProductCode" => product_code,
            "UpgradeCode" => upgrade_code,
            "Manufacturer" => manufacturer,
            "TARGETDIR" => targetdir,
        };

        for (k, v) in info_fields {
            properties.row(vec![Value::Str(k), Value::Str(v)]);
        }

        package.insert_rows(properties);

        //
        // Generate schema
        //

        package.create_table(
            "Directory",
            vec![
                ColumnBuilder::new("Directory").primary_key().string(72, None),
                ColumnBuilder::new("Directory_Parent").nullable().string(72, None),
                ColumnBuilder::new("DefaultDir").string(255, None),
            ],
        )?;
        package.create_table(
            "Component",
            vec![
                ColumnBuilder::new("Component").primary_key().string(72, None),
                ColumnBuilder::new("ComponentId").nullable().string(38, None),
                ColumnBuilder::new("Directory_").string(72, None),
                ColumnBuilder::new("Attributes").i32(),
                ColumnBuilder::new("Condition").nullable().string(255, None),
                ColumnBuilder::new("KeyPath").nullable().string(72, None),
            ],
        )?;
        package.create_table(
            "File",
            vec![
                ColumnBuilder::new("File").primary_key().string(72, None),
                ColumnBuilder::new("Component_").string(72, None),
                ColumnBuilder::new("FileName").string(255, None),
                ColumnBuilder::new("FileSize").i32(),
                ColumnBuilder::new("Version").nullable().string(72, None),
                ColumnBuilder::new("Language").nullable().string(20, None),
                ColumnBuilder::new("Attributes").nullable().i32(),
                ColumnBuilder::new("Sequence").i32(),
            ],
        )?;
        package.create_table(
            "Media",
            vec![
                ColumnBuilder::new("DiskId").primary_key().i16(),
                ColumnBuilder::new("LastSequence").i32(),
                ColumnBuilder::new("DiskPrompt").nullable().string(64, None),
                ColumnBuilder::new("Cabinet").nullable().string(255, None),
                ColumnBuilder::new("VolumeLabel").nullable().string(32, None),
                ColumnBuilder::new("Source").nullable().string(72, None),
            ],
        )?;
        package.create_table(
            "Feature",
            vec![
                ColumnBuilder::new("Feature").primary_key().string(38, None),
                ColumnBuilder::new("Feature_Parent").nullable().string(38, None),
                ColumnBuilder::new("Title").nullable().string(64, None),
                ColumnBuilder::new("Description").nullable().string(255, None),
                ColumnBuilder::new("Display").nullable().i16(),
                ColumnBuilder::new("Level").i16(),
                ColumnBuilder::new("Directory_").nullable().string(72, None),
                ColumnBuilder::new("Attributes").i32(),
            ],
        )?;
        package.create_table(
            "FeatureComponents",
            vec![
                ColumnBuilder::new("Feature_").primary_key().string(38, None),
                ColumnBuilder::new("Component_").primary_key().string(72, None),
            ],
        )?;

        //
        // Generate source media
        //

        let t = tempfile::tempdir().with_path("<tmp>")?;
        let t_pth = t.path();
        let name_dir = t_pth.join(name);
        fs::create_dir_all(name_dir).with_path(name_dir)?;

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
        package.insert_rows(
            Insert::into("Media").row(vec![
                Value::Int(disk_id),
                Value::Int(sequence_id),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ]),
        )?;

        let dir0 = "Dir0";

        package.insert_rows(
            Insert::into("Directory")
                .row(vec![
                    Value::Str("TARGETDIR".into()),
                    Value::Null,
                    Value::Str("SourceDir".into()),
                ])
                .row(vec![
                    Value::Str("ProgramFiles64Folder".into()),
                    Value::Str("TARGETDIR".into()),
                    Value::Str(".".into()),
                ])
                .row(vec![
                    Value::Str(dir0),
                    Value::Str("ProgramFiles64Folder".into()),
                    Value::Str(name.into()),
                ]),
        )?;

        let mut target_dir_to_id = btreemap!{
            name => dir0,
        };
        let mut dir_id = 1u64;

        //
        // Generate Feature
        //

        let main_feature = "Complete";
        package.insert_rows(
            Insert::into("Feature")
                .row(vec![
                    Value::Str(main_feature),
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Int(1i16),
                    dir0,
                    0i32,
                ]),
        )?;

        let attribute_file_uncompressed = 8192i32;
        let attribute_64_bit = 256i32;
        let mut file_id  = 0u64;
        let mut component_id = 0u64;

        for entry in walkdir::WalkDir(name_dir).sort_by_file_name() {
            let entry = entry?;
            let pth = entry.path();
            let pth_relative = pth.strip_prefix(name_dir)?;

            if pth_relative.is_empty() {
                continue;
            }

            let pth_relative_string = pth_relative
                .to_str()
                .ok_or(ChiriError::UnrenderablePathError(pth_relative))?;
            let basename = pth.basename();
            let parent = pth.parent();
            let parent_string = pth
                .parent()
                .strip_prefix(name_dir)?
                .to_str()
                .ok_or(ChiriError::UnrenderablePathError(pth_relative))?;
            let d_parent_id = target_dir_to_id.get(parent_string)?;

            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                let d_id = format!("Dir{dir_id}");
                target_dir_to_id.insert(pth_relative_string, d_id);
                dir_id += 1;

                package.insert_rows(
                    Insert::into("Directory")
                        .row(vec![
                            Value::Str(d_id.into()),
                            Value::Str(d_parent_id.into()),
                            Value::Str(basename.into()),
                        ]),
                )?;
            }

            if !metadata.is_file () {
                return Err(ChiriError::UnsupportedFileSystemFileTypeError(pth.to_path_buf()));
            }

            //
            // Generate File
            //

            let f_id = format!("File{file_id}");
            file_id += 1;
            let c_id = format!("Component{component_id}");
            component_id += 1;

            package.insert_rows(
                Insert::into("File").row(vec![
                    Value::Str(f_id.into()),
                    Value::Str(c_id.into()),
                    Value::Str(basename.into()),
                    Value::Int(metadata.len() as i32),
                    Value::Null,
                    Value::Null,
                    Value::Int(attribute_file_uncompressed),
                    Value::Int(sequence_id),
                ]),
            )?;

            //
            // Generate Component
            //

            let c_uuid = format!("{{}}", uuid::Uuid::new_v4());

            package.insert_rows(
                Insert::into("Component").row(vec![
                    Value::Str(c_id.into()),
                    Value::Str(c_uuid.into()),
                    Value::Str(parent_string.into()),
                    Value::Int(attribute_64_bit),
                    Value::Null,
                    Value::Str(f_id.into()),
                ]),
            )?;

            //
            // Generate FeatureComponent
            //

            package.insert_rows(
                Insert::into("FeatureComponents")
                    .row(vec![
                        Value::Str(main_feature),
                        Value::Str(c_id),
            ]))?;
        }

        package.commit()?;

        Ok(())
    }
}
