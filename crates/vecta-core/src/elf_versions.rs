//! Strict GNU version-need traversal. Offsets are relative to their enclosing
//! records, and the same 16-byte record layout is used by ELF32 and ELF64.
use anyhow::{Context, Result, ensure};
fn word(data: &[u8], offset: usize, little: bool) -> Result<u32> {
    let bytes: [u8; 4] = data
        .get(offset..offset.checked_add(4).context("ELF offset overflow")?)
        .context("truncated GNU version record")?
        .try_into()?;
    Ok(if little {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    })
}
fn half(data: &[u8], offset: usize, little: bool) -> Result<u16> {
    let bytes: [u8; 2] = data
        .get(offset..offset.checked_add(2).context("ELF offset overflow")?)
        .context("truncated GNU version record")?
        .try_into()?;
    Ok(if little {
        u16::from_le_bytes(bytes)
    } else {
        u16::from_be_bytes(bytes)
    })
}
fn advance(offset: usize, delta: u32) -> Result<usize> {
    offset
        .checked_add(delta as usize)
        .context("ELF version offset overflow")
}
pub(crate) fn required_versions(
    data: &[u8],
    strings: &[u8],
    little: bool,
    count: u32,
) -> Result<Vec<String>> {
    ensure!(
        count as usize <= data.len() / 16,
        "invalid GNU version-need record count"
    );
    ensure!(
        count > 0 || data.is_empty(),
        "GNU version section has records but zero declared count"
    );
    let mut out = Vec::new();
    let mut offset: usize = 0;
    for index in 0..count {
        ensure!(
            offset.checked_add(16).is_some_and(|end| end <= data.len()),
            "truncated GNU version record"
        );
        ensure!(
            half(data, offset, little)? == 1,
            "unsupported GNU version-need revision"
        );
        let aux_count = half(data, offset + 2, little)?;
        let aux_delta = word(data, offset + 8, little)?;
        ensure!(
            aux_delta >= 16 && aux_count > 0,
            "invalid GNU version auxiliary offset/count"
        );
        let mut aux = advance(offset, aux_delta)?;
        ensure!(
            aux_count as usize <= data.len().saturating_sub(aux) / 16,
            "invalid GNU version auxiliary count"
        );
        for aux_index in 0..aux_count {
            ensure!(
                aux.checked_add(16).is_some_and(|end| end <= data.len()),
                "truncated GNU auxiliary record"
            );
            let name_offset = word(data, aux + 8, little)? as usize;
            let string = strings
                .get(name_offset..)
                .context("invalid ELF version string offset")?;
            let end = string
                .iter()
                .position(|b| *b == 0)
                .context("unterminated ELF version name")?;
            out.push(std::str::from_utf8(&string[..end])?.to_owned());
            let next = word(data, aux + 12, little)?;
            if aux_index + 1 < aux_count {
                ensure!(next >= 16, "invalid GNU version auxiliary chain");
                aux = advance(aux, next)?;
            } else {
                ensure!(next == 0, "GNU version auxiliary count/chain mismatch");
            }
        }
        let next = word(data, offset + 12, little)?;
        if index + 1 < count {
            ensure!(next >= 16, "invalid GNU version-need chain");
            offset = advance(offset, next)?;
        } else {
            ensure!(next == 0, "GNU version-need count/chain mismatch");
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(little: bool) -> Vec<u8> {
        let mut data = Vec::new();
        for n in [1u16, 1] {
            data.extend(if little {
                n.to_le_bytes()
            } else {
                n.to_be_bytes()
            });
        }
        for n in [0u32, 16, 0, 0, 0, 1, 0] {
            data.extend(if little {
                n.to_le_bytes()
            } else {
                n.to_be_bytes()
            });
        }
        data
    }
    #[test]
    fn endian_and_required_only() {
        for le in [true, false] {
            assert_eq!(
                required_versions(&fixture(le), b"\0GLIBC_2.34\0GLIBC_99.0\0", le, 1).unwrap(),
                ["GLIBC_2.34"]
            );
        }
    }
    #[test]
    fn malformed_chains_and_truncation() {
        let data = fixture(true);
        for size in 0..data.len() {
            assert!(required_versions(&data[..size], b"\0GLIBC_2.34\0", true, 1).is_err());
        }
        let mut bad = data.clone();
        bad[12] = 16;
        assert!(required_versions(&bad, b"\0GLIBC_2.34\0", true, 1).is_err());
        assert!(required_versions(&data, b"\0GLIBC_2.34", true, 1).is_err());
    }
}
