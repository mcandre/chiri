// Gemini
// https://share.gemini.google/sFortzGykMaY

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(not(unix))]
use std::time::UNIX_EPOCH;

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    InvalidFormat(String),
    UnsupportedNodeType(PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::InvalidFormat(msg) => write!(f, "Invalid BOM format: {}", msg),
            Error::UnsupportedNodeType(p) => write!(f, "Unsupported file type at {:?}", p),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

/// Entry type corresponding to `TYPE_FILE`, `TYPE_DIR`, `TYPE_LINK`, `TYPE_DEV` in `bom.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EntryType {
    File = 1,
    Dir = 2,
    Link = 3,
    Dev = 4,
}

impl TryFrom<u8> for EntryType {
    type Error = Error;

    fn try_from(val: u8) -> Result<Self> {
        match val {
            1 => Ok(EntryType::File),
            2 => Ok(EntryType::Dir),
            3 => Ok(EntryType::Link),
            4 => Ok(EntryType::Dev),
            _ => Err(Error::InvalidFormat(format!("Unknown entry type: {}", val))),
        }
    }
}

/// Represents a single file, directory, symlink, or device node in the Bill of Materials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomEntry {
    /// Relative path inside the BOM (e.g. `"."`, `"./usr"`, `"./usr/bin/foo"`).
    pub path: String,
    pub entry_type: EntryType,
    pub architecture: u16,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub modtime: u32,
    pub size: u32,
    /// CRC-32 checksum (for files) or `rdev` device ID (for devices).
    pub checksum_or_dev: u32,
    /// Target path if `entry_type == EntryType::Link` (without trailing NUL).
    pub link_name: Option<String>,
}

/// In-memory representation of an Apple Bill of Materials (`.bom`) archive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bom {
    pub entries: Vec<BomEntry>,
}

impl Bom {
    /// Reads and parses a `.bom` file from disk.
    pub fn read<P: AsRef<Path>>(input: P) -> Result<Bom> {
        read(input)
    }

    /// Scans `root_dir` on disk and constructs a `Bom` structure matching `mkbom` behavior.
    pub fn create_bom<P: AsRef<Path>>(
        root_dir: P,
        uid_override: Option<u32>,
        gid_override: Option<u32>,
    ) -> Result<Bom> {
        create_bom(root_dir, uid_override, gid_override)
    }

    /// Serializes this `Bom` into a binary `.bom` file at `output`.
    pub fn write<P: AsRef<Path>>(&self, output: P) -> Result<()> {
        let mut storage = BomStorage::new();
        let num = self.entries.len() as u32;

        // 1. Build BomInfo variable
        let mut bom_info = Vec::with_capacity(12 + if num > 0 { 16 } else { 0 });
        bom_info.extend_from_slice(&1u32.to_be_bytes()); // version = 1
        bom_info.extend_from_slice(&(num + 1).to_be_bytes()); // numberOfPaths = num + 1
        bom_info.extend_from_slice(&(if num > 0 { 1u32 } else { 0u32 }).to_be_bytes());
        if num > 0 {
            bom_info.extend_from_slice(&[0u8; 16]); // BOMInfoEntry (all zeros)
        }
        storage.add_var("BomInfo", &bom_info);

        // 2. Build hierarchical tree matching mkbom's BFS traversal order
        let mut root = BuildNode::default();
        for entry in &self.entries {
            let clean = entry.path.trim_end_matches('/');
            let components: Vec<&str> = if clean == "." {
                vec!["."]
            } else {
                clean.split('/').filter(|s| !s.is_empty()).collect()
            };

            let mut curr = &mut root;
            for (idx, comp) in components.iter().enumerate() {
                curr = curr.children.entry((*comp).to_string()).or_default();
                if idx + 1 == components.len() {
                    curr.entry = Some(entry.clone());
                }
            }
        }

        // Flatten tree in breadth-first order identical to mkbom.cpp
        struct FlatNode<'a> {
            id: u32,
            parent_id: u32,
            name: String,
            entry: &'a BomEntry,
        }

        let mut flat_nodes = Vec::with_capacity(self.entries.len());
        let mut queue: VecDeque<(u32, &BuildNode)> = VecDeque::new();
        queue.push_back((0, &root));
        let mut next_id = 1u32;

        while let Some((parent_id, node)) = queue.pop_front() {
            for (name, child) in &node.children {
                let entry = child.entry.as_ref().ok_or_else(|| {
                    Error::InvalidFormat(format!(
                        "Missing intermediate directory entry for '{}'",
                        name
                    ))
                })?;
                let id = next_id;
                next_id += 1;
                flat_nodes.push(FlatNode {
                    id,
                    parent_id,
                    name: name.clone(),
                    entry,
                });
                queue.push_back((id, child));
            }
        }

        // 3. Serialize Paths B-Tree (256 entries per leaf, matching mkbom)
        let total_paths = flat_nodes.len();
        let num_leaves = if total_paths == 0 {
            1
        } else {
            total_paths.div_ceil(256)
        };

        let mut leaf_block_ids = Vec::with_capacity(num_leaves);
        let mut leaf_last_file_ids = Vec::with_capacity(num_leaves);
        let mut leaf_buffers: Vec<Vec<u8>> = Vec::with_capacity(num_leaves);

        if total_paths == 0 {
            let mut empty_leaf = Vec::with_capacity(12);
            empty_leaf.extend_from_slice(&1u16.to_be_bytes()); // isLeaf = 1
            empty_leaf.extend_from_slice(&0u16.to_be_bytes()); // count = 0
            empty_leaf.extend_from_slice(&0u32.to_be_bytes()); // forward = 0
            empty_leaf.extend_from_slice(&0u32.to_be_bytes()); // backward = 0
            leaf_buffers.push(empty_leaf);
            leaf_last_file_ids.push(0);
        } else {
            for chunk in flat_nodes.chunks(256) {
                let mut leaf = Vec::with_capacity(12 + chunk.len() * 8);
                leaf.extend_from_slice(&1u16.to_be_bytes()); // isLeaf = 1
                leaf.extend_from_slice(&(chunk.len() as u16).to_be_bytes());
                leaf.extend_from_slice(&0u32.to_be_bytes()); // forward (patched after allocation)
                leaf.extend_from_slice(&0u32.to_be_bytes()); // backward (patched after allocation)

                let mut last_file_id = 0u32;
                for item in chunk {
                    let e = item.entry;
                    let link_bytes = e.link_name.as_deref().unwrap_or("");
                    let link_len_with_nul = if e.entry_type == EntryType::Link {
                        (link_bytes.len() + 1) as u32
                    } else {
                        0
                    };

                    // BOMPathInfo2
                    let mut info2 = Vec::with_capacity(33 + link_len_with_nul as usize);
                    info2.push(e.entry_type as u8);
                    info2.push(1); // unknown0 = 1
                    info2.extend_from_slice(&e.architecture.to_be_bytes());
                    info2.extend_from_slice(&e.mode.to_be_bytes());
                    info2.extend_from_slice(&e.uid.to_be_bytes());
                    info2.extend_from_slice(&e.gid.to_be_bytes());
                    info2.extend_from_slice(&e.modtime.to_be_bytes());
                    info2.extend_from_slice(&e.size.to_be_bytes());
                    info2.push(1); // unknown1 = 1
                    info2.extend_from_slice(&e.checksum_or_dev.to_be_bytes());
                    info2.extend_from_slice(&link_len_with_nul.to_be_bytes());
                    if link_len_with_nul > 0 {
                        info2.extend_from_slice(link_bytes.as_bytes());
                        info2.push(0);
                    }
                    let info2_block = storage.add_block(&info2);

                    // BOMPathInfo1
                    let mut info1 = Vec::with_capacity(8);
                    info1.extend_from_slice(&item.id.to_be_bytes());
                    info1.extend_from_slice(&info2_block.to_be_bytes());
                    let info1_block = storage.add_block(&info1);

                    // BOMFile
                    let mut bom_file = Vec::with_capacity(4 + item.name.len() + 1);
                    bom_file.extend_from_slice(&item.parent_id.to_be_bytes());
                    bom_file.extend_from_slice(item.name.as_bytes());
                    bom_file.push(0);
                    let file_block = storage.add_block(&bom_file);
                    last_file_id = file_block;

                    // BOMPathIndices
                    leaf.extend_from_slice(&info1_block.to_be_bytes());
                    leaf.extend_from_slice(&file_block.to_be_bytes());
                }

                leaf_buffers.push(leaf);
                leaf_last_file_ids.push(last_file_id);
            }
        }

        // Allocate leaf blocks and patch forward/backward pointers
        for buf in &leaf_buffers {
            leaf_block_ids.push(storage.add_block(buf));
        }
        for i in 0..leaf_block_ids.len() {
            let prev_id = if i > 0 { leaf_block_ids[i - 1] } else { 0 };
            let next_id = if i + 1 < leaf_block_ids.len() {
                leaf_block_ids[i + 1]
            } else {
                0
            };
            let blk = storage.get_block_mut(leaf_block_ids[i]);
            blk[4..8].copy_from_slice(&next_id.to_be_bytes());
            blk[8..12].copy_from_slice(&prev_id.to_be_bytes());
        }

        let paths_root_block = if leaf_block_ids.len() > 1 {
            let mut branch = Vec::with_capacity(12 + leaf_block_ids.len() * 8);
            branch.extend_from_slice(&0u16.to_be_bytes()); // isLeaf = 0
            branch.extend_from_slice(&(leaf_block_ids.len() as u16).to_be_bytes());
            branch.extend_from_slice(&0u32.to_be_bytes()); // forward = 0
            branch.extend_from_slice(&0u32.to_be_bytes()); // backward = 0
            for (blk_id, last_file_id) in leaf_block_ids.iter().zip(leaf_last_file_ids.iter()) {
                branch.extend_from_slice(&blk_id.to_be_bytes());
                branch.extend_from_slice(&last_file_id.to_be_bytes());
            }
            storage.add_block(&branch)
        } else {
            leaf_block_ids[0]
        };

        let paths_tree = make_bom_tree(paths_root_block, 4096, total_paths as u32);
        storage.add_var("Paths", &paths_tree);

        // 4. Write HLIndex, VIndex, and Size64 variables (matching mkbom.cpp)
        let mut empty_paths = Vec::with_capacity(12);
        empty_paths.extend_from_slice(&1u16.to_be_bytes()); // isLeaf = 1
        empty_paths.extend_from_slice(&0u16.to_be_bytes()); // count = 0
        empty_paths.extend_from_slice(&0u32.to_be_bytes()); // forward = 0
        empty_paths.extend_from_slice(&0u32.to_be_bytes()); // backward = 0

        let hl_child = storage.add_block(&empty_paths);
        storage.add_var("HLIndex", &make_bom_tree(hl_child, 4096, 0));

        let v_child = storage.add_block(&empty_paths);
        let v_tree_block = storage.add_block(&make_bom_tree(v_child, 128, 0));
        let mut vindex = Vec::with_capacity(13);
        vindex.extend_from_slice(&1u32.to_be_bytes()); // unknown0 = 1
        vindex.extend_from_slice(&v_tree_block.to_be_bytes()); // indexToVTree
        vindex.extend_from_slice(&0u32.to_be_bytes()); // unknown2 = 0
        vindex.push(0); // unknown3 = 0
        storage.add_var("VIndex", &vindex);

        let s64_child = storage.add_block(&empty_paths);
        storage.add_var("Size64", &make_bom_tree(s64_child, 4096, 0));

        // 5. Write entire BOMStore to output file
        let mut file = File::create(output)?;
        storage.write_to(&mut file)?;
        Ok(())
    }
}

/// Reads and parses a `.bom` file from `input`.
pub fn read<P: AsRef<Path>>(input: P) -> Result<Bom> {
    let data = fs::read(input)?;
    if data.len() < 32 {
        return Err(Error::InvalidFormat("File too short for BOMHeader".into()));
    }
    if &data[0..8] != b"BOMStore" {
        return Err(Error::InvalidFormat("Missing BOMStore magic header".into()));
    }

    let version = read_u32_be(&data, 8)?;
    if version != 1 {
        return Err(Error::InvalidFormat(format!(
            "Unsupported BOM version: {}",
            version
        )));
    }

    let index_offset = read_u32_be(&data, 16)? as usize;
    let index_length = read_u32_be(&data, 20)? as usize;
    let vars_offset = read_u32_be(&data, 24)? as usize;
    let vars_length = read_u32_be(&data, 28)? as usize;

    // Parse BOMBlockTable
    let index_end = index_offset
        .checked_add(index_length)
        .ok_or_else(|| Error::InvalidFormat("Index offset overflow".into()))?;
    if index_end > data.len() || index_offset + 4 > data.len() {
        return Err(Error::InvalidFormat("Block table out of bounds".into()));
    }

    let num_block_pointers = read_u32_be(&data, index_offset)? as usize;
    let mut blocks: Vec<Option<(usize, usize)>> = Vec::with_capacity(num_block_pointers);
    let mut cursor = index_offset + 4;
    for _ in 0..num_block_pointers {
        if cursor + 8 > index_end {
            return Err(Error::InvalidFormat("Truncated block pointer table".into()));
        }
        let addr = read_u32_be(&data, cursor)? as usize;
        let len = read_u32_be(&data, cursor + 4)? as usize;
        cursor += 8;
        if len == 0 {
            blocks.push(None);
        } else {
            let end = addr
                .checked_add(len)
                .ok_or_else(|| Error::InvalidFormat("Block address overflow".into()))?;
            if end > data.len() {
                return Err(Error::InvalidFormat("Block points past EOF".into()));
            }
            blocks.push(Some((addr, len)));
        }
    }

    let get_block = |idx: u32| -> Result<&[u8]> {
        let entry = blocks
            .get(idx as usize)
            .and_then(|b| *b)
            .ok_or_else(|| Error::InvalidFormat(format!("Invalid or null block index: {}", idx)))?;
        Ok(&data[entry.0..entry.0 + entry.1])
    };

    // Parse BOMVars to locate "Paths"
    let vars_end = vars_offset
        .checked_add(vars_length)
        .ok_or_else(|| Error::InvalidFormat("Vars offset overflow".into()))?;
    if vars_end > data.len() || vars_offset + 4 > data.len() {
        return Err(Error::InvalidFormat("Vars table out of bounds".into()));
    }

    let var_count = read_u32_be(&data, vars_offset)? as usize;
    let mut vcursor = vars_offset + 4;
    let mut paths_tree_block = None;

    for _ in 0..var_count {
        if vcursor + 5 > vars_end {
            return Err(Error::InvalidFormat("Truncated BOMVar entry".into()));
        }
        let block_idx = read_u32_be(&data, vcursor)?;
        let name_len = data[vcursor + 4] as usize;
        vcursor += 5;
        if vcursor + name_len > vars_end {
            return Err(Error::InvalidFormat("Truncated BOMVar name".into()));
        }
        let name = &data[vcursor..vcursor + name_len];
        vcursor += name_len;
        if name == b"Paths" {
            paths_tree_block = Some(block_idx);
        }
    }

    let Some(tree_idx) = paths_tree_block else {
        return Ok(Bom {
            entries: Vec::new(),
        });
    };

    // Parse BOMTree
    let tree_bytes = get_block(tree_idx)?;
    if tree_bytes.len() < 21 || &tree_bytes[0..4] != b"tree" {
        return Err(Error::InvalidFormat("Invalid Paths BOMTree header".into()));
    }
    let root_paths_idx = read_u32_be(tree_bytes, 8)?;

    // Traverse BOMPaths tree to collect leaf records
    struct RawRecord {
        id: u32,
        parent_id: u32,
        name: String,
        entry_type: EntryType,
        architecture: u16,
        mode: u16,
        uid: u32,
        gid: u32,
        modtime: u32,
        size: u32,
        checksum_or_dev: u32,
        link_name: Option<String>,
    }

    let mut records = Vec::new();
    let mut stack = vec![root_paths_idx];

    while let Some(node_idx) = stack.pop() {
        let node = get_block(node_idx)?;
        if node.len() < 12 {
            return Err(Error::InvalidFormat("Truncated BOMPaths node".into()));
        }
        let is_leaf = read_u16_be(node, 0)?;
        let count = read_u16_be(node, 2)? as usize;

        if node.len() < 12 + count * 8 {
            return Err(Error::InvalidFormat("Truncated BOMPathIndices".into()));
        }

        if is_leaf == 0 {
            // Push branch children in reverse so leftmost is popped first
            for i in (0..count).rev() {
                let child_idx = read_u32_be(node, 12 + i * 8)?;
                stack.push(child_idx);
            }
        } else {
            for i in 0..count {
                let info1_idx = read_u32_be(node, 12 + i * 8)?;
                let file_idx = read_u32_be(node, 12 + i * 8 + 4)?;

                // BOMPathInfo1
                let info1 = get_block(info1_idx)?;
                if info1.len() < 8 {
                    return Err(Error::InvalidFormat("Truncated BOMPathInfo1".into()));
                }
                let id = read_u32_be(info1, 0)?;
                let info2_idx = read_u32_be(info1, 4)?;

                // BOMPathInfo2
                let info2 = get_block(info2_idx)?;
                if info2.len() < 33 {
                    return Err(Error::InvalidFormat("Truncated BOMPathInfo2".into()));
                }
                let entry_type = EntryType::try_from(info2[0])?;
                let architecture = read_u16_be(info2, 2)?;
                let mode = read_u16_be(info2, 4)?;
                let uid = read_u32_be(info2, 6)?;
                let gid = read_u32_be(info2, 10)?;
                let modtime = read_u32_be(info2, 14)?;
                let size = read_u32_be(info2, 18)?;
                let checksum_or_dev = read_u32_be(info2, 23)?;
                let link_len = read_u32_be(info2, 27)? as usize;

                let link_name = if link_len > 0 {
                    if info2.len() < 31 + link_len {
                        return Err(Error::InvalidFormat(
                            "Truncated BOMPathInfo2 linkName".into(),
                        ));
                    }
                    let raw_link = &info2[31..31 + link_len];
                    let trimmed = raw_link.strip_suffix(&[0]).unwrap_or(raw_link);
                    Some(String::from_utf8_lossy(trimmed).into_owned())
                } else {
                    None
                };

                // BOMFile
                let file_blk = get_block(file_idx)?;
                if file_blk.len() < 5 {
                    return Err(Error::InvalidFormat("Truncated BOMFile".into()));
                }
                let parent_id = read_u32_be(file_blk, 0)?;
                let raw_name = &file_blk[4..];
                let trimmed_name = raw_name.strip_suffix(&[0]).unwrap_or(raw_name);
                let name = String::from_utf8_lossy(trimmed_name).into_owned();

                records.push(RawRecord {
                    id,
                    parent_id,
                    name,
                    entry_type,
                    architecture,
                    mode,
                    uid,
                    gid,
                    modtime,
                    size,
                    checksum_or_dev,
                    link_name,
                });
            }
        }
    }

    // Reconstruct full paths from parent IDs
    let mut id_to_parent_name: BTreeMap<u32, (u32, String)> = BTreeMap::new();
    for rec in &records {
        id_to_parent_name.insert(rec.id, (rec.parent_id, rec.name.clone()));
    }

    let mut entries = Vec::with_capacity(records.len());
    for rec in records {
        let mut parts = vec![rec.name.clone()];
        let mut curr_parent = rec.parent_id;
        let mut guard = 0usize;
        while curr_parent != 0 {
            guard += 1;
            if guard > id_to_parent_name.len() + 1 {
                return Err(Error::InvalidFormat(
                    "Cycle detected in BOM parent IDs".into(),
                ));
            }
            let (next_parent, pname) = id_to_parent_name.get(&curr_parent).ok_or_else(|| {
                Error::InvalidFormat(format!("Dangling parent ID: {}", curr_parent))
            })?;
            parts.push(pname.clone());
            curr_parent = *next_parent;
        }
        parts.reverse();
        let full_path = parts.join("/");

        entries.push(BomEntry {
            path: full_path,
            entry_type: rec.entry_type,
            architecture: rec.architecture,
            mode: rec.mode,
            uid: rec.uid,
            gid: rec.gid,
            modtime: rec.modtime,
            size: rec.size,
            checksum_or_dev: rec.checksum_or_dev,
            link_name: rec.link_name,
        });
    }

    Ok(Bom { entries })
}

/// Scans `root_dir` on disk and creates a `Bom` instance with CRC-32 checksums, permissions, and symlinks.
pub fn create_bom<P: AsRef<Path>>(
    root_dir: P,
    uid_override: Option<u32>,
    gid_override: Option<u32>,
) -> Result<Bom> {
    let root_path = root_dir.as_ref();
    let mut entries = Vec::new();
    collect_entries(root_path, ".", uid_override, gid_override, &mut entries)?;
    Ok(Bom { entries })
}

/// Serializes `bom` into a `.bom` file at `output`.
pub fn write<P: AsRef<Path>>(bom: &Bom, output: P) -> Result<()> {
    bom.write(output)
}

// ============================================================================
// Internal Helpers
// ============================================================================

#[derive(Default)]
struct BuildNode {
    entry: Option<BomEntry>,
    children: BTreeMap<String, BuildNode>,
}

struct BomStorage {
    vars: Vec<u8>,
    num_vars: u32,
    entries: Vec<u8>,
    /// `(relative_offset_in_entries, length)` where index 0 is the mandatory null block.
    block_table: Vec<(u32, u32)>,
}

impl BomStorage {
    fn new() -> Self {
        Self {
            vars: Vec::new(),
            num_vars: 0,
            entries: Vec::new(),
            block_table: vec![(0, 0)],
        }
    }

    fn add_block(&mut self, data: &[u8]) -> u32 {
        let offset = self.entries.len() as u32;
        let len = data.len() as u32;
        self.entries.extend_from_slice(data);
        let id = self.block_table.len() as u32;
        self.block_table.push((offset, len));
        id
    }

    fn get_block_mut(&mut self, id: u32) -> &mut [u8] {
        let (offset, len) = self.block_table[id as usize];
        let start = offset as usize;
        let end = start + len as usize;
        &mut self.entries[start..end]
    }

    fn add_var(&mut self, name: &str, data: &[u8]) {
        let block_id = self.add_block(data);
        self.vars.extend_from_slice(&block_id.to_be_bytes());
        self.vars.push(name.len() as u8);
        self.vars.extend_from_slice(name.as_bytes());
        self.num_vars += 1;
    }

    fn write_to<W: Write>(&self, out: &mut W) -> io::Result<()> {
        let header_size = 512u32;
        let vars_size = 4u32 + self.vars.len() as u32;
        let entries_size = self.entries.len() as u32;
        let block_table_size = 4u32 + (self.block_table.len() as u32 * 8);
        let free_list_size = 4u32 + (2 * 8); // 0 entries + 2 dummy pointers (matches mkbom.cpp)

        let number_of_blocks = (self.block_table.len() - 1) as u32;
        let vars_offset = header_size;
        let index_offset = header_size + vars_size + entries_size;
        let index_length = block_table_size + free_list_size;

        // 1. 512-byte BOMHeader
        let mut header = [0u8; 512];
        header[0..8].copy_from_slice(b"BOMStore");
        header[8..12].copy_from_slice(&1u32.to_be_bytes());
        header[12..16].copy_from_slice(&number_of_blocks.to_be_bytes());
        header[16..20].copy_from_slice(&index_offset.to_be_bytes());
        header[20..24].copy_from_slice(&index_length.to_be_bytes());
        header[24..28].copy_from_slice(&vars_offset.to_be_bytes());
        header[28..32].copy_from_slice(&vars_size.to_be_bytes());
        out.write_all(&header)?;

        // 2. BOMVars
        out.write_all(&self.num_vars.to_be_bytes())?;
        out.write_all(&self.vars)?;

        // 3. Block payload bytes
        out.write_all(&self.entries)?;

        // 4. BOMBlockTable
        let base_data_offset = header_size + vars_size;
        out.write_all(&(self.block_table.len() as u32).to_be_bytes())?;
        for &(rel_addr, len) in &self.block_table {
            let abs_addr = if len == 0 {
                0
            } else {
                base_data_offset + rel_addr
            };
            out.write_all(&abs_addr.to_be_bytes())?;
            out.write_all(&len.to_be_bytes())?;
        }

        // 5. BOMFreeList (count = 0, 2 null BOMPointers)
        out.write_all(&[0u8; 20])?;
        Ok(())
    }
}

fn make_bom_tree(child: u32, block_size: u32, path_count: u32) -> [u8; 21] {
    let mut buf = [0u8; 21];
    buf[0..4].copy_from_slice(b"tree");
    buf[4..8].copy_from_slice(&1u32.to_be_bytes());
    buf[8..12].copy_from_slice(&child.to_be_bytes());
    buf[12..16].copy_from_slice(&block_size.to_be_bytes());
    buf[16..20].copy_from_slice(&path_count.to_be_bytes());
    buf[20] = 0; // unknown3
    buf
}

fn collect_entries(
    fs_path: &Path,
    bom_path: &str,
    uid_override: Option<u32>,
    gid_override: Option<u32>,
    out: &mut Vec<BomEntry>,
) -> Result<()> {
    let meta = fs::symlink_metadata(fs_path)?;
    let file_type = meta.file_type();

    let (uid, gid, mode, modtime, rdev) = extract_platform_meta(&meta);
    let uid = uid_override.unwrap_or(uid);
    let gid = gid_override.unwrap_or(gid);

    if file_type.is_dir() {
        out.push(BomEntry {
            path: bom_path.to_string(),
            entry_type: EntryType::Dir,
            architecture: 3,
            mode: (mode & 0o7777) | 0o040000,
            uid,
            gid,
            modtime,
            size: 0,
            checksum_or_dev: 0,
            link_name: None,
        });

        let mut children: Vec<_> =
            fs::read_dir(fs_path)?.collect::<std::result::Result<Vec<_>, _>>()?;
        children.sort_by_key(|e| e.file_name());

        for child in children {
            let name = child.file_name();
            let name_str = name.to_string_lossy();
            let child_bom_path = format!("{}/{}", bom_path, name_str);
            collect_entries(
                &child.path(),
                &child_bom_path,
                uid_override,
                gid_override,
                out,
            )?;
        }
    } else if file_type.is_file() {
        let (size, crc) = file_size_and_crc32(fs_path)?;
        out.push(BomEntry {
            path: bom_path.to_string(),
            entry_type: EntryType::File,
            architecture: 3,
            mode: (mode & 0o7777) | 0o100000,
            uid,
            gid,
            modtime,
            size,
            checksum_or_dev: crc,
            link_name: None,
        });
    } else if file_type.is_symlink() {
        let target = fs::read_link(fs_path)?;
        let target_str = target.to_string_lossy().into_owned();
        let target_bytes = target_str.as_bytes();
        let crc = crc32_bytes(target_bytes);
        out.push(BomEntry {
            path: bom_path.to_string(),
            entry_type: EntryType::Link,
            architecture: 3,
            mode: (mode & 0o7777) | 0o120000,
            uid,
            gid,
            modtime,
            size: target_bytes.len() as u32,
            checksum_or_dev: crc,
            link_name: Some(target_str),
        });
    } else {
        #[cfg(unix)]
        if file_type.is_block_device() || file_type.is_char_device() {
            let type_bits = if file_type.is_block_device() {
                0o060000
            } else {
                0o020000
            };
            out.push(BomEntry {
                path: bom_path.to_string(),
                entry_type: EntryType::Dev,
                architecture: 3,
                mode: (mode & 0o7777) | type_bits,
                uid,
                gid,
                modtime,
                size: 0,
                checksum_or_dev: rdev,
                link_name: None,
            });
            return Ok(());
        }
        let _ = rdev;
        return Err(Error::UnsupportedNodeType(fs_path.to_path_buf()));
    }

    Ok(())
}

#[cfg(unix)]
fn extract_platform_meta(meta: &fs::Metadata) -> (u32, u32, u16, u32, u32) {
    (
        meta.uid(),
        meta.gid(),
        meta.mode() as u16,
        meta.mtime() as u32,
        meta.rdev() as u32,
    )
}

#[cfg(not(unix))]
fn extract_platform_meta(meta: &fs::Metadata) -> (u32, u32, u16, u32, u32) {
    let mode = if meta.is_dir() {
        0o755
    } else if meta.permissions().readonly() {
        0o444
    } else {
        0o644
    };
    let modtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);
    (0, 0, mode, modtime, 0)
}

/// Computes Apple BOM CRC-32 checksum (standard IEEE 802.3 / zlib CRC-32) and byte size.
fn file_size_and_crc32(path: &Path) -> io::Result<(u32, u32)> {
    let mut file = File::open(path)?;
    let mut crc = 0xFFFF_FFFFu32;
    let mut total = 0u32;
    let mut buf = [0u8; 65536];

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total = total.wrapping_add(n as u32);
        crc = update_crc32(crc, &buf[..n]);
    }
    Ok((total, !crc))
}

fn crc32_bytes(data: &[u8]) -> u32 {
    !update_crc32(0xFFFF_FFFF, data)
}

const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            if c & 1 != 0 {
                c = 0xEDB8_8320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

fn update_crc32(mut crc: u32, buf: &[u8]) -> u32 {
    for &b in buf {
        let idx = ((crc ^ (b as u32)) & 0xFF) as usize;
        crc = CRC32_TABLE[idx] ^ (crc >> 8);
    }
    crc
}

fn read_u16_be(buf: &[u8], offset: usize) -> Result<u16> {
    let bytes = buf
        .get(offset..offset + 2)
        .ok_or_else(|| Error::InvalidFormat("Unexpected EOF reading u16".into()))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32_be(buf: &[u8], offset: usize) -> Result<u32> {
    let bytes = buf
        .get(offset..offset + 4)
        .ok_or_else(|| Error::InvalidFormat("Unexpected EOF reading u32".into()))?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}
