use crate::*;
use anyhow::{Context, Result, bail, ensure};
use object::{Object, ObjectSection, ObjectSymbol};
use std::{fs, path::Path};

pub fn demangle(name: &str) -> String {
    if let Ok(s) = rustc_demangle::try_demangle(name) {
        return format!("{s:#}");
    }
    if let Ok(s) = cpp_demangle::Symbol::new(if name.starts_with("__Z") {
        &name[1..]
    } else {
        name
    }) && let Ok(s) = s.demangle(&cpp_demangle::DemangleOptions::default())
    {
        return s;
    }
    name.to_owned()
}

pub fn inspect_path(path: impl AsRef<Path>) -> Result<Snapshot> {
    let path = path.as_ref();
    inspect(&fs::read(path).with_context(|| format!("reading {}", path.display()))?)
}
/// Loads either a supported binary or a versioned Vecta JSON snapshot.
pub fn load(path: impl AsRef<Path>) -> Result<Snapshot> {
    let bytes =
        fs::read(path.as_ref()).with_context(|| format!("reading {}", path.as_ref().display()))?;
    if bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{') {
        let snapshot: Snapshot = serde_json::from_slice(&bytes).context("invalid snapshot")?;
        ensure!(
            snapshot.schema_version == SCHEMA_VERSION,
            "unsupported snapshot schema {}",
            snapshot.schema_version
        );
        Ok(snapshot)
    } else {
        inspect(&bytes)
    }
}

pub fn inspect(bytes: &[u8]) -> Result<Snapshot> {
    let file = object::File::parse(bytes)
        .context("unsupported or malformed binary (fat Mach-O: extract one architecture first)")?;
    let mut snapshot = Snapshot {
        schema_version: SCHEMA_VERSION,
        format: format!("{:?}", file.format()),
        architecture: format!("{:?}", file.architecture()),
        bits: if file.is_64() { 64 } else { 32 },
        little_endian: file.is_little_endian(),
        file_size: bytes.len() as u64,
        sections: Vec::new(),
        symbols: Vec::new(),
        libraries: Vec::new(),
        abi: BTreeMap::new(),
        abi_requirements: Vec::new(),
        abi_complete: true,
        warnings: Vec::new(),
    };
    for section in file.sections() {
        snapshot.sections.push(Section {
            name: section.name()?.to_owned(),
            size: section.size(),
            file_size: section.file_range().map_or(0, |(_, size)| size),
        });
    }
    // Prefer the complete table; dynsym is a fallback, not an additional copy.
    let full: Vec<_> = file.symbols().collect();
    let symbols: Vec<_> = if full.is_empty() {
        snapshot.warnings.push(
            "No static symbol table; using dynamic symbols. Symbol attribution is incomplete."
                .into(),
        );
        file.dynamic_symbols().collect()
    } else {
        full
    };
    for symbol in symbols {
        if !symbol.is_definition()
            || !matches!(
                symbol.kind(),
                object::SymbolKind::Text
                    | object::SymbolKind::Data
                    | object::SymbolKind::Tls
                    | object::SymbolKind::Unknown
            )
        {
            continue;
        }
        let raw_name = symbol.name()?.to_owned();
        if raw_name.is_empty() {
            continue;
        }
        let section = symbol
            .section_index()
            .and_then(|i| file.section_by_index(i).ok())
            .and_then(|s| s.name().ok())
            .unwrap_or("")
            .to_owned();
        let content_hash = symbol
            .section_index()
            .and_then(|i| file.section_by_index(i).ok())
            .and_then(|s| {
                if symbol.size() == 0 {
                    return None;
                }
                let start = usize::try_from(symbol.address().checked_sub(s.address())?).ok()?;
                let end = start.checked_add(usize::try_from(symbol.size()).ok()?)?;
                let data = s.data().ok()?;
                Some(blake3::hash(data.get(start..end)?).to_hex().to_string())
            });
        snapshot.symbols.push(Symbol {
            name: demangle(&raw_name),
            raw_name,
            section,
            size: symbol.size(),
            content_hash,
        });
    }
    match goblin::Object::parse(bytes)? {
        goblin::Object::Elf(elf) => {
            snapshot.libraries = elf.libraries.iter().map(|s| (*s).to_owned()).collect();
            if elf.section_headers.is_empty() {
                snapshot.abi_complete = false;
                snapshot.warnings.push(
                    "ELF section headers missing; ABI requirements cannot be established.".into(),
                );
            }
            let mut found_versions = false;
            for section in &elf.section_headers {
                if section.sh_type != goblin::elf::section_header::SHT_GNU_VERNEED {
                    continue;
                }
                found_versions = true;
                let strings = elf
                    .section_headers
                    .get(section.sh_link as usize)
                    .context("invalid GNU version string table link")?;
                let records = section_bytes(bytes, section.sh_offset, section.sh_size)?;
                let strings = section_bytes(bytes, strings.sh_offset, strings.sh_size)?;
                for name in crate::elf_versions::required_versions(
                    records,
                    strings,
                    elf.little_endian,
                    section.sh_info,
                )? {
                    snapshot.abi_requirements.push(name.clone());
                    for family in ["GLIBC", "GLIBCXX"] {
                        if let Some(suffix) = name.strip_prefix(&format!("{family}_")) {
                            match suffix.parse::<Version>() {
                                Ok(version) => {
                                    snapshot
                                        .abi
                                        .entry(family.into())
                                        .and_modify(|v| {
                                            if version > *v {
                                                *v = version.clone();
                                            }
                                        })
                                        .or_insert(version);
                                }
                                Err(_) => {
                                    snapshot.abi_complete = false;
                                    snapshot
                                        .warnings
                                        .push(format!("Non-numeric ABI requirement: {name}"));
                                }
                            }
                        }
                    }
                }
            }
            if !found_versions
                && elf.dynamic.as_ref().is_some_and(|d| {
                    d.dyns
                        .iter()
                        .any(|d| d.d_tag == goblin::elf::dynamic::DT_VERNEED)
                })
            {
                snapshot.abi_complete = false;
                snapshot.warnings.push("Dynamic version needs exist without a GNU version-need section; ABI extraction is incomplete.".into());
            }
        }
        goblin::Object::PE(pe) => {
            snapshot.libraries = pe
                .libraries
                .iter()
                .map(|s| s.to_ascii_lowercase())
                .collect();
            snapshot.warnings.push("PE symbol sizes are unavailable in many images; zero means unknown. PDB files are not read.".into());
        }
        goblin::Object::Mach(goblin::mach::Mach::Binary(mach)) => {
            snapshot.libraries = mach.libs.iter().skip(1).map(|s| (*s).to_owned()).collect();
            snapshot
                .warnings
                .push("Mach-O symbols do not encode sizes; zero means unknown.".into());
        }
        _ => bail!("unsupported binary format"),
    }
    snapshot
        .sections
        .sort_by(|a, b| a.name.cmp(&b.name).then(a.size.cmp(&b.size)));
    snapshot
        .symbols
        .sort_by(|a, b| (&a.raw_name, &a.section, a.size).cmp(&(&b.raw_name, &b.section, b.size)));
    snapshot.abi_requirements.sort();
    snapshot.abi_requirements.dedup();
    snapshot.libraries.sort();
    snapshot.libraries.dedup();
    snapshot.warnings.sort();
    snapshot.warnings.dedup();
    Ok(snapshot)
}

fn section_bytes(bytes: &[u8], offset: u64, size: u64) -> Result<&[u8]> {
    let start = usize::try_from(offset)?;
    let end = usize::try_from(
        offset
            .checked_add(size)
            .context("ELF section offset overflow")?,
    )?;
    bytes
        .get(start..end)
        .context("ELF section extends beyond the input")
}
