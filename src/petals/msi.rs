use cab;
use msi::{self, CodePage, Column, Insert, LanguageId, Value};
use tempfile;
use uuid;

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path;

use crate::{
    Chiri, Pkg,
    dos::generate_8_3_map,
    error::{ChiriError, ResultExt},
    petal::Petal,
};

impl Petal {
    /// emit_msi generates Windows (MSI) packages.
    ///
    /// Requirements:
    ///
    /// * pkg.petal.maintainer present, nonblank
    /// * pkg.petal.product_code present, nonblank
    /// * pkg.petal.upgrade_code present, nonblank
    /// * pkg.petal.manufacturer present, nonblank
    pub fn emit_msi(self: &Petal, pkg: &Pkg, chiri: &Chiri) -> Result<(), ChiriError> {
        let blank = String::new();
        let windows = String::from("windows");
        let code_page_windows1252 = CodePage::Windows1252;
        let en_us = String::from("en-US");
        let lcid1033 = String::from("1033");
        let source_dir = String::from("SourceDir");
        let petal = pkg.petal.clone().unwrap_or_default();
        let name = petal.get("name").unwrap_or(&blank);
        let name_title = format!("{name} Installer");
        let version = petal.get("version").unwrap_or(&blank);
        let rev = petal.get("rev").unwrap_or(&blank);
        let base_dir = petal.get("base_dir").unwrap_or(&windows);
        let dist = petal.get("dist").unwrap_or(&blank);
        let product_language = petal.get("product_language").unwrap_or(&lcid1033);
        let product_code = petal.get("product_code").unwrap_or(&blank);
        let upgrade_code = petal.get("upgrade_code").unwrap_or(&blank);
        let chipset = petal.get("chipset").unwrap_or(&blank);
        let codepage = match petal.get("codepage") {
            Some(e) => CodePage::from_id(
                e.parse::<i32>()
                    .map_err(|_| ChiriError::ValidationError(format!("invalid i32: {e}")))?,
            )
            .ok_or(ChiriError::ValidationError(format!(
                "invalid codepage: {e}"
            )))?,
            _ => code_page_windows1252,
        };
        let copyright = petal.get("copyright").unwrap_or(&blank);
        let languages = petal
            .get("languages")
            .unwrap_or(&en_us)
            .split(',')
            .map(LanguageId::from_tag)
            .collect::<Vec<LanguageId>>();
        let license = petal.get("license").unwrap_or(&blank);
        let maintainer = petal.get("maintainer").unwrap_or(&blank);
        let manufacturer = petal.get("manufacturer").unwrap_or(&blank);
        let summary = petal.get("summary").unwrap_or(&blank);
        let targetdir = petal.get("targetdir").unwrap_or(&source_dir);
        let title = petal.get("title").unwrap_or(&name_title);
        let dest = pkg.dest.clone().unwrap_or_default();

        if languages.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.languages blank",
            )));
        }

        if maintainer.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.maintainer blank/missing",
            )));
        }

        if product_language.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.product_language blank",
            )));
        }

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

        if title.is_empty() {
            return Err(ChiriError::ValidationError(String::from(
                "petal.title blank",
            )));
        }

        let msi_artifacts_root = chiri.artifacts.clone().unwrap().join(base_dir);
        fs::create_dir_all(&msi_artifacts_root).with_path(&msi_artifacts_root)?;

        let mut basename = format!("{name}-{version}");

        if !rev.is_empty() {
            basename.push('.');
            basename.push_str(rev);
        }

        if !dist.is_empty() {
            basename.push('-');
            basename.push_str(dist);
        }

        basename.push('-');
        basename.push_str(chipset);
        basename.push_str(".msi");

        let artifact_filename = msi_artifacts_root.join(basename);
        let f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(&artifact_filename)
            .with_path(&artifact_filename)?;
        let mut package =
            msi::Package::create(msi::PackageType::Installer, f).with_path(&artifact_filename)?;

        //
        // Generate top level metadata
        //

        let page_count = match chipset.as_str() {
            "Arm64" => 500,
            _ => 200,
        };
        let data_from_cab_word_count = 2;
        let info = package.summary_info_mut();
        info.set_arch(chipset);
        info.set_title(title);
        info.set_subject(name);
        info.set_author(maintainer);
        info.set_creating_application("chiri");
        info.set_codepage(codepage);
        info.set_languages(&languages);
        info.set_uuid(uuid::Uuid::new_v4());
        info.set_page_count(page_count);
        info.set_word_count(data_from_cab_word_count);

        if !summary.is_empty() {
            info.set_comments(summary);
        }

        //
        // Generate schema
        //

        package
            .create_table(
                "Property",
                vec![
                    Column::build("Property").primary_key().id_string(72),
                    Column::build("Value").localizable().text_string(0),
                ],
            )
            .with_path(&artifact_filename)?;
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
                    Column::build("Directory_").id_string(72),
                    Column::build("Attributes").int32(),
                    Column::build("Condition").nullable().formatted_string(255),
                    Column::build("KeyPath").nullable().id_string(72),
                ],
            )
            .with_path(&artifact_filename)?;
        package
            .create_table(
                "File",
                vec![
                    Column::build("File").primary_key().id_string(72),
                    Column::build("Component_").id_string(72),
                    Column::build("FileName").formatted_string(255),
                    Column::build("FileSize").int32(),
                    Column::build("Version").nullable().id_string(72),
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
                    Column::build("Source").nullable().id_string(72),
                ],
            )
            .with_path(&artifact_filename)?;
        package
            .create_table(
                "Feature",
                vec![
                    Column::build("Feature").primary_key().id_string(38),
                    Column::build("Feature_Parent").nullable().id_string(38),
                    Column::build("Title").nullable().formatted_string(64),
                    Column::build("Description")
                        .nullable()
                        .formatted_string(255),
                    Column::build("Display").nullable().int16(),
                    Column::build("Level").int16(),
                    Column::build("Directory_").nullable().id_string(72),
                    Column::build("Attributes").int32(),
                ],
            )
            .with_path(&artifact_filename)?;
        package
            .create_table(
                "FeatureComponents",
                vec![
                    Column::build("Feature_").primary_key().id_string(38),
                    Column::build("Component_").primary_key().id_string(72),
                ],
            )
            .with_path(&artifact_filename)?;
        package
            .create_table(
                "InstallExecuteSequence",
                vec![
                    Column::build("Action").primary_key().id_string(72),
                    Column::build("Condition").nullable().formatted_string(255),
                    Column::build("Sequence").nullable().int16(),
                ],
            )
            .with_path(&artifact_filename)?;

        let info_fields: BTreeMap<String, String> = [
            ("ProductName", name),
            ("ProductVersion", version),
            ("ProductLanguage", product_language),
            ("ProductCode", &format!("{{{}}}", product_code)),
            ("UpgradeCode", &format!("{{{}}}", upgrade_code)),
            ("Manufacturer", manufacturer),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();

        let mut properties = Insert::into("Property");

        for (k, v) in info_fields {
            properties = properties.row(vec![Value::Str(k.to_string()), Value::Str(v.to_string())]);
        }

        package
            .insert_rows(properties)
            .with_path(&artifact_filename)?;

        //
        // Generate InstallExecuteSequence
        //

        package
            .insert_rows(Insert::into("InstallExecuteSequence").rows(vec![
                vec![
                    Value::Str(String::from("LaunchConditions")),
                    Value::Null,
                    Value::Int(100),
                ],
                vec![
                    Value::Str(String::from("FindRelatedProducts")),
                    Value::Null,
                    Value::Int(200),
                ],
                vec![
                    Value::Str(String::from("AppSearch")),
                    Value::Null,
                    Value::Int(400),
                ],
                vec![
                    Value::Str(String::from("ValidateProductID")),
                    Value::Null,
                    Value::Int(700),
                ],
                vec![
                    Value::Str(String::from("CostInitialize")),
                    Value::Null,
                    Value::Int(800),
                ],
                vec![
                    Value::Str(String::from("FileCost")),
                    Value::Null,
                    Value::Int(900),
                ],
                vec![
                    Value::Str(String::from("CostFinalize")),
                    Value::Null,
                    Value::Int(1000),
                ],
                vec![
                    Value::Str(String::from("InstallValidate")),
                    Value::Null,
                    Value::Int(1400),
                ],
                vec![
                    Value::Str(String::from("InstallInitialize")),
                    Value::Null,
                    Value::Int(1500),
                ],
                vec![
                    Value::Str(String::from("ProcessComponents")),
                    Value::Null,
                    Value::Int(1600),
                ],
                vec![
                    Value::Str(String::from("UnpublishFeatures")),
                    Value::Null,
                    Value::Int(1800),
                ],
                vec![
                    Value::Str(String::from("RemoveRegistryValues")),
                    Value::Null,
                    Value::Int(2600),
                ],
                vec![
                    Value::Str(String::from("RemoveShortcuts")),
                    Value::Null,
                    Value::Int(3200),
                ],
                vec![
                    Value::Str(String::from("RemoveFiles")),
                    Value::Null,
                    Value::Int(3500),
                ],
                vec![
                    Value::Str(String::from("InstallFiles")),
                    Value::Null,
                    Value::Int(4000),
                ],
                vec![
                    Value::Str(String::from("CreateShortcuts")),
                    Value::Null,
                    Value::Int(4500),
                ],
                vec![
                    Value::Str(String::from("WriteRegistryValues")),
                    Value::Null,
                    Value::Int(5000),
                ],
                vec![
                    Value::Str(String::from("RegisterUser")),
                    Value::Null,
                    Value::Int(6000),
                ],
                vec![
                    Value::Str(String::from("RegisterProduct")),
                    Value::Null,
                    Value::Int(6100),
                ],
                vec![
                    Value::Str(String::from("PublishFeatures")),
                    Value::Null,
                    Value::Int(6300),
                ],
                vec![
                    Value::Str(String::from("PublishProduct")),
                    Value::Null,
                    Value::Int(6400),
                ],
                vec![
                    Value::Str(String::from("InstallFinalize")),
                    Value::Null,
                    Value::Int(6600),
                ],
            ]))
            .with_path(&artifact_filename)?;

        //
        // Generate source media
        //

        let t = tempfile::tempdir().with_path("<tmp>")?;
        let t_pth = t.path();
        let name_dir = t_pth.join(name);
        let name_dir_string = name_dir
            .to_str()
            .ok_or(ChiriError::UnrenderablePathError(name_dir.to_path_buf()))?;
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
        // Generate file tree
        //

        let dir0 = "Dir0";
        let path_to_short_name = generate_8_3_map(&name_dir)?;
        let name_dir_short = path_to_short_name.get(name_dir_string).unwrap();
        let mut sequence_id = 1i32;

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
                        Value::Str(format!("{name_dir_short}|{name}")),
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

        let attribute_file_compressed = 0i32;
        let attribute_64_bit = 256i32;
        let mut file_keys = Vec::<String>::new();
        let mut file_id = 0u64;
        let mut component_id = 0u64;

        for entry in walkdir::WalkDir::new(&name_dir).sort_by_file_name() {
            let entry = entry?;
            let pth = entry.path();

            if pth == name_dir {
                continue;
            }

            let pth_string = pth
                .to_str()
                .ok_or(ChiriError::UnrenderablePathError(pth.to_path_buf()))?;
            let pth_relative = pth.strip_prefix(t_pth)?;
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
            let parent = pth_relative
                .parent()
                .ok_or(ChiriError::ParentLookupError(pth.to_path_buf()))?;
            let parent_string = parent
                .to_str()
                .ok_or(ChiriError::UnrenderablePathError(parent.to_path_buf()))?
                .to_owned();

            let d_parent_id = target_dir_to_id.get(&parent_string).unwrap().clone();
            let name_short = path_to_short_name.get(pth_string).unwrap();
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                let d_id = format!("Dir{dir_id}");
                target_dir_to_id.insert(pth_relative_string, d_id.clone());
                dir_id += 1;

                package
                    .insert_rows(Insert::into("Directory").row(vec![
                        Value::Str(d_id),
                        Value::Str(d_parent_id.to_string()),
                        Value::Str(format!("{name_short}|{basename_str}")),
                    ]))
                    .with_path(&artifact_filename)?;

                continue;
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

            file_keys.push(f_id.clone());
            package
                .insert_rows(Insert::into("File").row(vec![
                    Value::Str(f_id.clone()),
                    Value::Str(c_id.clone()),
                    Value::Str(format!("{name_short}|{basename_str}")),
                    Value::Int(metadata.len() as i32),
                    Value::Null,
                    Value::Null,
                    Value::Int(attribute_file_compressed),
                    Value::Int(sequence_id),
                ]))
                .with_path(&artifact_filename)?;

            sequence_id += 1;

            //
            // Generate Component
            //

            let c_uuid = format!("{{{}}}", uuid::Uuid::new_v4());

            package
                .insert_rows(Insert::into("Component").row(vec![
                    Value::Str(c_id.clone()),
                    Value::Str(c_uuid.to_string()),
                    Value::Str(d_parent_id),
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

        //
        // Generate CAB
        //

        let mut builder = cab::CabinetBuilder::new();
        let folder = builder.add_folder(cab::CompressionType::MsZip);

        for file_key in file_keys {
            folder.add_file(file_key);
        }

        let mut cab_buffer = io::Cursor::new(Vec::new());
        let mut cab_writer = builder.build(&mut cab_buffer).with_path("<ram>")?;

        for entry in walkdir::WalkDir::new(name_dir).sort_by_file_name() {
            let entry = entry?;
            let pth = entry.path();
            let metadata = pth.metadata().with_path(pth)?;

            if metadata.is_dir() {
                continue;
            }

            if !metadata.is_file() {
                return Err(ChiriError::UnsupportedFileSystemFileTypeError(
                    pth.to_path_buf(),
                ));
            }

            let mut entry_writer = cab_writer
                .next_file()
                .with_path("<ram>")?
                .ok_or(ChiriError::CABError(String::from("missing cab entry")))?;
            let mut f = fs::File::open(pth).with_path(pth)?;
            io::copy(&mut f, &mut entry_writer).with_path(pth)?;
        }

        cab_writer.finish().with_path("<ram>")?;
        let cab_filename = "cab1.cab";
        let mut stream = package
            .write_stream(cab_filename)
            .with_path(&artifact_filename)?;
        stream
            .write_all(&cab_buffer.into_inner())
            .with_path(&artifact_filename)?;
        stream.flush().with_path(&artifact_filename)?;

        //
        // Generate Media
        //

        let disk_id = 1i32;
        package
            .insert_rows(Insert::into("Media").row(vec![
                Value::Int(disk_id),
                Value::Int(sequence_id - 1),
                Value::Null,
                Value::Str(format!("#{cab_filename}")),
                Value::Null,
                Value::Null,
            ]))
            .with_path(&artifact_filename)?;

        package.flush().with_path(artifact_filename)?;

        Ok(())
    }
}
