use crate::*;
use anyhow::{Result, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use flate2::{Compression, write::DeflateEncoder};
use std::io::Write;
/// Conservative full-URL limit for Markdown and browsers.
pub const MAX_SHARE_URL: usize = 8000;
/// Raw DEFLATE + URL-safe unpadded Base64. The payload is versioned JSON.
/// Removes lowest-ranked symbols, then section rows until the URL fits;
/// ABI and library changes are never silently discarded.
pub fn share_url(diff: &Diff, viewer: &str) -> Result<String> {
    ensure!(
        (viewer.starts_with("https://") || viewer.starts_with("http://"))
            && viewer.is_ascii()
            && !viewer.contains('#')
            && !viewer.chars().any(char::is_control)
            && !viewer.chars().any(char::is_whitespace),
        "viewer must be an HTTP(S) URL without a fragment or whitespace"
    );
    ensure!(
        viewer.len() + 3 < MAX_SHARE_URL,
        "viewer URL leaves no room for a share payload"
    );
    let mut summary = diff.clone();
    summary.symbols.truncate(30);
    summary.warnings.clear();
    summary.sections.retain(|s| s.delta != 0);
    let mut omitted_symbols = diff.symbols.len() - summary.symbols.len();
    let mut omitted_sections = diff.sections.len() - summary.sections.len();
    loop {
        let payload = serde_json::json!({"schema_version":SCHEMA_VERSION,"diff":summary,"omitted_symbols":omitted_symbols,"omitted_sections":omitted_sections});
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&serde_json::to_vec(&payload)?)?;
        let url = format!("{viewer}#z={}", URL_SAFE_NO_PAD.encode(encoder.finish()?));
        if url.len() <= MAX_SHARE_URL {
            return Ok(url);
        }
        if summary.symbols.pop().is_some() {
            omitted_symbols += 1;
        } else if summary.sections.pop().is_some() {
            omitted_sections += 1;
        } else {
            anyhow::bail!(
                "ABI/library summary exceeds {MAX_SHARE_URL}-byte URL limit; use JSON output"
            );
        }
    }
}
