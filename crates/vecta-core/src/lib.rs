//! Binary snapshots, deterministic comparisons, and CI budgets.
mod diff;
mod elf_versions;
mod gate;
mod inspect;
pub use diff::*;
pub use gate::*;
pub use inspect::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub format: String,
    pub architecture: String,
    pub bits: u8,
    pub little_endian: bool,
    pub file_size: u64,
    pub sections: Vec<Section>,
    pub symbols: Vec<Symbol>,
    pub libraries: Vec<String>,
    pub abi: BTreeMap<String, Version>,
    pub abi_requirements: Vec<String>,
    pub abi_complete: bool,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    /// Logical size, including zero-fill sections such as .bss.
    pub size: u64,
    pub file_size: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Symbol {
    pub raw_name: String,
    pub name: String,
    pub section: String,
    pub size: u64,
    /// BLAKE3 of bytes attributed to this symbol; absent when sizes/data are unavailable.
    pub content_hash: Option<String>,
}
/// Numeric runtime version; trailing zero components compare equally.
#[derive(Debug, Clone, Serialize, Deserialize, Eq)]
#[serde(try_from = "String", into = "String")]
pub struct Version(Vec<u32>);
impl std::str::FromStr for Version {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        anyhow::ensure!(!s.is_empty() && s.len() <= 100, "invalid version: {s}");
        let mut parts = s
            .split('.')
            .map(|p| {
                anyhow::ensure!(
                    !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()),
                    "invalid version: {s}"
                );
                Ok(p.parse::<u32>()?)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        while parts.len() > 1 && parts.last() == Some(&0) {
            parts.pop();
        }
        Ok(Self(parts))
    }
}
impl TryFrom<String> for Version {
    type Error = anyhow::Error;
    fn try_from(s: String) -> anyhow::Result<Self> {
        s.parse()
    }
}
impl From<Version> for String {
    fn from(v: Version) -> Self {
        v.to_string()
    }
}
impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, p) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            write!(f, "{p}")?;
        }
        Ok(())
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
