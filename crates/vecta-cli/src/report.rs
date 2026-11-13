use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use std::{
    fmt::Write,
    io::{self, IsTerminal},
};
use vecta_core::*;
fn safe(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}
fn md(s: &str) -> String {
    safe(s)
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "&#124;")
        .replace('`', "&#96;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
        .replace('*', "&#42;")
        .replace('_', "&#95;")
}
fn version(v: &Option<Version>) -> String {
    v.as_ref().map_or_else(|| "—".into(), ToString::to_string)
}
pub fn markdown(diff: &Diff, gate: Option<&GateResult>, top: usize, share: Option<&str>) -> String {
    let mut out = String::from("## Vecta binary regression report\n\n");
    if let Some(g) = gate {
        let (status, color) = if g.passed {
            ("PASS", "brightgreen")
        } else {
            ("FAIL", "red")
        };
        writeln!(
            out,
            "![Vecta gate: {status}](https://img.shields.io/badge/Vecta-{status}-{color})\n"
        )
        .unwrap();
    }
    writeln!(
        out,
        "{} / {} · file: {} → {} bytes (**{:+}**)\n",
        md(&diff.format),
        md(&diff.architecture),
        diff.file.base,
        diff.file.head,
        diff.file.delta
    )
    .unwrap();
    out.push_str(
        "### Sections\n\n| Section | Base bytes | Head bytes | Delta |\n|---|---:|---:|---:|\n",
    );
    for s in &diff.sections {
        writeln!(
            out,
            "| {} | {} | {} | {:+} |",
            md(&s.name),
            s.base,
            s.head,
            s.delta
        )
        .unwrap();
    }
    out.push_str("\n### Symbol changes\n\n| Symbol | Section | Change | Base bytes | Head bytes | Delta |\n|---|---|---|---:|---:|---:|\n");
    for s in diff.symbols.iter().take(top) {
        writeln!(
            out,
            "| {} | {} | {:?} | {} | {} | {:+} |",
            md(&s.name),
            md(&s.section),
            s.kind,
            s.base,
            s.head,
            s.delta
        )
        .unwrap();
    }
    writeln!(
        out,
        "\nShowing {} of {} symbol changes.\n",
        top.min(diff.symbols.len()),
        diff.symbols.len()
    )
    .unwrap();
    out.push_str(
        "### Runtime requirements\n\n| Runtime | Base | Head | Increased |\n|---|---|---|---|\n",
    );
    for a in &diff.abi {
        writeln!(
            out,
            "| {} | {} | {} | {} |",
            md(&a.family),
            version(&a.base),
            version(&a.head),
            a.increased
        )
        .unwrap();
    }
    for (label, libs) in [
        ("Added dynamic libraries", &diff.libraries_added),
        ("Removed dynamic libraries", &diff.libraries_removed),
    ] {
        writeln!(
            out,
            "\n**{label}:** {}\n",
            if libs.is_empty() {
                "none".into()
            } else {
                libs.iter().map(|s| md(s)).collect::<Vec<_>>().join(", ")
            }
        )
        .unwrap();
    }
    if let Some(g) = gate {
        for v in &g.violations {
            writeln!(out, "- ❌ {}", md(v)).unwrap();
        }
        out.push('\n');
    }
    for warning in &diff.warnings {
        writeln!(out, "> ⚠️ {}\n", md(warning)).unwrap();
    }
    if let Some(url) = share {
        writeln!(
            out,
            "[Open visual diff](<{}>)",
            url.replace('<', "%3C").replace('>', "%3E")
        )
        .unwrap();
    }
    out
}
pub fn terminal(
    diff: &Diff,
    gate: Option<&GateResult>,
    top: usize,
    share: Option<&str>,
    stdout: bool,
) -> String {
    let color = stdout && io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let mut out = format!(
        "Vecta · {} / {}\nFile: {} → {} bytes ({:+})\n\n",
        safe(&diff.format),
        safe(&diff.architecture),
        diff.file.base,
        diff.file.head,
        diff.file.delta
    );
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(["Section", "Base", "Head", "Delta"]);
    for s in &diff.sections {
        table.add_row(vec![
            Cell::new(safe(&s.name)),
            Cell::new(s.base),
            Cell::new(s.head),
            delta_cell(s.delta, color),
        ]);
    }
    writeln!(out, "{table}\n").unwrap();
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(["Symbol", "Section", "Change", "Delta"]);
    for s in diff.symbols.iter().take(top) {
        table.add_row(vec![
            Cell::new(safe(&s.name)),
            Cell::new(safe(&s.section)),
            Cell::new(format!("{:?}", s.kind)),
            delta_cell(s.delta, color),
        ]);
    }
    writeln!(
        out,
        "{table}\nShowing {} of {} symbol changes.\n",
        top.min(diff.symbols.len()),
        diff.symbols.len()
    )
    .unwrap();
    for a in &diff.abi {
        writeln!(
            out,
            "{}: {} → {}{}",
            safe(&a.family),
            version(&a.base),
            version(&a.head),
            if a.increased { " [INCREASED]" } else { "" }
        )
        .unwrap();
    }
    for l in &diff.libraries_added {
        writeln!(out, "+ library {}", safe(l)).unwrap();
    }
    for l in &diff.libraries_removed {
        writeln!(out, "- library {}", safe(l)).unwrap();
    }
    if let Some(g) = gate {
        writeln!(out, "\nGate: {}", if g.passed { "PASS" } else { "FAIL" }).unwrap();
        for v in &g.violations {
            writeln!(out, "  ! {}", safe(v)).unwrap();
        }
    }
    for w in &diff.warnings {
        writeln!(out, "warning: {}", safe(w)).unwrap();
    }
    if let Some(url) = share {
        writeln!(out, "\n{url}").unwrap();
    }
    out
}
fn delta_cell(delta: i128, color: bool) -> Cell {
    let cell = Cell::new(format!("{delta:+}"));
    if color {
        cell.fg(match delta.cmp(&0) {
            std::cmp::Ordering::Greater => Color::Red,
            std::cmp::Ordering::Less => Color::Green,
            std::cmp::Ordering::Equal => Color::White,
        })
    } else {
        cell
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn escapes_untrusted_names() {
        assert_eq!(super::md("<x|`\n\x1b>"), "&lt;x&#124;&#96;&gt;");
    }
}
