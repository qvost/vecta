use crate::*;
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub name: String,
    pub base: u64,
    pub head: u64,
    pub delta: i128,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolChange {
    pub raw_name: String,
    pub name: String,
    pub section: String,
    pub kind: ChangeKind,
    pub base: u64,
    pub head: u64,
    pub delta: i128,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiChange {
    pub family: String,
    pub base: Option<Version>,
    pub head: Option<Version>,
    pub increased: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    pub schema_version: u32,
    pub format: String,
    pub architecture: String,
    pub file: Delta,
    pub sections: Vec<Delta>,
    pub symbols: Vec<SymbolChange>,
    pub libraries_added: Vec<String>,
    pub libraries_removed: Vec<String>,
    pub abi: Vec<AbiChange>,
    pub abi_complete: bool,
    pub warnings: Vec<String>,
}
fn delta(name: String, base: u64, head: u64) -> Delta {
    Delta {
        name,
        base,
        head,
        delta: i128::from(head) - i128::from(base),
    }
}
fn section_totals(s: &Snapshot) -> Result<BTreeMap<String, u64>> {
    let mut out = BTreeMap::<String, u64>::new();
    for section in &s.sections {
        let value = out.entry(section.name.clone()).or_default();
        *value = value
            .checked_add(section.size)
            .ok_or_else(|| anyhow::anyhow!("section size overflow"))?;
    }
    Ok(out)
}
#[derive(PartialEq, Eq)]
struct SymbolTotals {
    name: String,
    size: u64,
    hashes: Vec<Option<String>>,
}
fn symbol_totals(s: &Snapshot) -> Result<BTreeMap<(String, String), SymbolTotals>> {
    let mut out = BTreeMap::<(String, String), SymbolTotals>::new();
    for symbol in &s.symbols {
        let value = out
            .entry((symbol.raw_name.clone(), symbol.section.clone()))
            .or_insert_with(|| SymbolTotals {
                name: symbol.name.clone(),
                size: 0,
                hashes: Vec::new(),
            });
        value.size = value
            .size
            .checked_add(symbol.size)
            .ok_or_else(|| anyhow::anyhow!("symbol size overflow"))?;
        value.hashes.push(symbol.content_hash.clone());
    }
    for value in out.values_mut() {
        value.hashes.sort();
    }
    Ok(out)
}
pub fn compare(base: &Snapshot, head: &Snapshot) -> Result<Diff> {
    ensure!(
        base.schema_version == SCHEMA_VERSION && head.schema_version == SCHEMA_VERSION,
        "unsupported snapshot schema"
    );
    ensure!(
        (
            base.format.as_str(),
            base.architecture.as_str(),
            base.bits,
            base.little_endian
        ) == (
            head.format.as_str(),
            head.architecture.as_str(),
            head.bits,
            head.little_endian
        ),
        "cannot compare different binary formats or architectures"
    );
    let a = section_totals(base)?;
    let b = section_totals(head)?;
    let sections = a
        .keys()
        .chain(b.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|key| {
            delta(
                key.clone(),
                *a.get(key).unwrap_or(&0),
                *b.get(key).unwrap_or(&0),
            )
        })
        .collect();
    let a = symbol_totals(base)?;
    let b = symbol_totals(head)?;
    let mut symbols = Vec::new();
    for key in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
        let old = a.get(key);
        let new = b.get(key);
        if old == new {
            continue;
        }
        let base = old.map_or(0, |s| s.size);
        let head = new.map_or(0, |s| s.size);
        symbols.push(SymbolChange {
            raw_name: key.0.clone(),
            section: key.1.clone(),
            name: new.or(old).unwrap().name.clone(),
            kind: match (old, new) {
                (None, _) => ChangeKind::Added,
                (_, None) => ChangeKind::Removed,
                _ => ChangeKind::Modified,
            },
            base,
            head,
            delta: i128::from(head) - i128::from(base),
        });
    }
    symbols.sort_by(|a, b| {
        b.delta
            .abs()
            .cmp(&a.delta.abs())
            .then(a.raw_name.cmp(&b.raw_name))
            .then(a.section.cmp(&b.section))
    });
    let a = base.libraries.iter().cloned().collect::<BTreeSet<_>>();
    let b = head.libraries.iter().cloned().collect::<BTreeSet<_>>();
    let abi = base
        .abi
        .keys()
        .chain(head.abi.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|family| {
            let old = base.abi.get(family).cloned();
            let new = head.abi.get(family).cloned();
            AbiChange {
                family: family.clone(),
                increased: new.is_some() && new > old,
                base: old,
                head: new,
            }
        })
        .collect();
    let warnings = base
        .warnings
        .iter()
        .chain(&head.warnings)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(Diff {
        schema_version: SCHEMA_VERSION,
        abi_complete: base.abi_complete && head.abi_complete,
        format: head.format.clone(),
        architecture: head.architecture.clone(),
        file: delta("file".into(), base.file_size, head.file_size),
        sections,
        symbols,
        libraries_added: b.difference(&a).cloned().collect(),
        libraries_removed: a.difference(&b).cloned().collect(),
        abi,
        warnings,
    })
}
