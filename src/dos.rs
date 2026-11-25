// Gemini
// https://share.gemini.google/MV4X5HAidp5h

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn generate_8_3_map<P: AsRef<Path>>(
    root_dir: P,
) -> Result<BTreeMap<String, String>, walkdir::Error> {
    let root_dir = root_dir.as_ref();
    let mut path_to_short: BTreeMap<String, String> = BTreeMap::new();
    let mut dir_used_names: BTreeMap<PathBuf, HashSet<String>> = BTreeMap::new();

    let walker = WalkDir::new(root_dir).sort_by_file_name();

    for entry in walker {
        let entry = entry?;
        let full_path = entry.path();

        // For root_dir (depth 0), resolve its actual folder name if passed as "." or "./"
        let raw_name = if entry.depth() == 0 {
            full_path
                .canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| {
                    let name = entry.file_name().to_string_lossy();
                    if name.is_empty() || name == "." || name == ".." {
                        "ROOT".to_string()
                    } else {
                        name.into_owned()
                    }
                })
        } else {
            entry.file_name().to_string_lossy().into_owned()
        };

        // Group collisions by parent directory (use an empty PathBuf for root_dir's scope)
        let parent_dir = if entry.depth() == 0 {
            PathBuf::new()
        } else {
            full_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_path_buf()
        };

        let used_in_dir = dir_used_names.entry(parent_dir).or_default();
        let short_name = generate_unique_8_3(&raw_name, used_in_dir);
        used_in_dir.insert(short_name.clone());

        path_to_short.insert(full_path.to_string_lossy().into_owned(), short_name);
    }

    Ok(path_to_short)
}

/// Generates a VFAT/DOS-compatible 8.3 short name and resolves collisions (`~1`, `~2`, etc.).
fn generate_unique_8_3(long_name: &str, used_names: &HashSet<String>) -> String {
    // 1. Strip leading/trailing spaces and leading dots (per VFAT spec)
    let trimmed = long_name.trim().trim_start_matches('.');

    // 2. Split on the *last* dot for base name and extension
    let (raw_base, raw_ext) = match trimmed.rfind('.') {
        Some(idx) if idx > 0 => (&trimmed[..idx], &trimmed[idx + 1..]),
        _ => (trimmed, ""),
    };

    // 3. Sanitize characters according to DOS 8.3 rules
    let base = sanitize_8_3_part(raw_base);
    let ext = sanitize_8_3_part(raw_ext);

    let base_chars: Vec<char> = base.chars().collect();
    let ext_trunc: String = ext.chars().take(3).collect();

    // Check if the name already fits 8.3 without truncation or invalid characters
    let needs_tail = base_chars.len() > 8
        || ext.chars().count() > 3
        || raw_base.chars().any(|c| c == ' ' || c == '.')
        || base.is_empty();

    if !needs_tail {
        let exact = format_8_3(&base, &ext_trunc);
        if !used_names.contains(&exact) {
            return exact;
        }
    }

    // 4. Generate numeric tail (~1 through ~999999)
    let fallback_base: Vec<char> = if base_chars.is_empty() {
        vec!['_']
    } else {
        base_chars
    };

    for n in 1..=999_999 {
        let suffix = format!("~{}", n);
        let max_prefix_len = 8usize.saturating_sub(suffix.len());
        let prefix: String = fallback_base.iter().take(max_prefix_len).collect();
        let candidate_base = format!("{}{}", prefix, suffix);
        let candidate = format_8_3(&candidate_base, &ext_trunc);

        if !used_names.contains(&candidate) {
            return candidate;
        }
    }

    unreachable!("Exhausted all 8.3 numeric suffixes in a single directory");
}

/// Converts characters to uppercase ASCII and replaces invalid 8.3 characters with `_`.
fn sanitize_8_3_part(s: &str) -> String {
    s.chars()
        .filter(|&c| c != ' ' && c != '.')
        .map(|c| {
            let uc = c.to_ascii_uppercase();
            if uc.is_ascii_alphanumeric()
                || matches!(
                    uc,
                    '$' | '%'
                        | '\''
                        | '-'
                        | '_'
                        | '@'
                        | '~'
                        | '`'
                        | '!'
                        | '('
                        | ')'
                        | '{'
                        | '}'
                        | '^'
                        | '#'
                        | '&'
                )
            {
                uc
            } else {
                '_'
            }
        })
        .collect()
}

/// Formats base and extension into `NAME.EXT` (or `NAME` if extension is empty).
fn format_8_3(base: &str, ext: &str) -> String {
    if ext.is_empty() {
        base.to_string()
    } else {
        format!("{}.{}", base, ext)
    }
}
