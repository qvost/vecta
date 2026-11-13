use crate::*;
#[derive(Debug, Default, Clone)]
pub struct Budgets {
    pub max_growth: Option<u64>,
    pub max_growth_percent: Option<f64>,
    pub section_growth: BTreeMap<String, u64>,
    pub forbid_new_libraries: bool,
    pub max_glibc: Option<Version>,
    pub max_glibcxx: Option<Version>,
    pub forbid_abi_bump: bool,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GateResult {
    pub passed: bool,
    pub violations: Vec<String>,
}
pub fn evaluate(diff: &Diff, budgets: &Budgets) -> anyhow::Result<GateResult> {
    if let Some(p) = budgets.max_growth_percent {
        anyhow::ensure!(
            p.is_finite() && p >= 0.0,
            "percentage must be finite and nonnegative"
        );
    }
    let mut violations = Vec::new();
    if !diff.abi_complete
        && (budgets.max_glibc.is_some() || budgets.max_glibcxx.is_some() || budgets.forbid_abi_bump)
    {
        violations.push(
            "ABI requirements are incomplete or non-numeric; refusing to pass an ABI policy".into(),
        );
    }
    if let Some(max) = budgets.max_growth
        && diff.file.delta > i128::from(max)
    {
        violations.push(format!(
            "File grew by {} bytes; budget is {max}",
            diff.file.delta
        ));
    }
    if let Some(max) = budgets.max_growth_percent {
        let allowed = diff.file.base as f64 * max / 100.0;
        if diff.file.delta > 0 && diff.file.delta as f64 > allowed {
            violations.push(format!(
                "File growth exceeds {max}% of {} bytes",
                diff.file.base
            ));
        }
    }
    for section in &diff.sections {
        if let Some(max) = budgets.section_growth.get(&section.name)
            && section.delta > i128::from(*max)
        {
            violations.push(format!(
                "Section {} grew by {} bytes; budget is {max}",
                section.name, section.delta
            ));
        }
    }
    if budgets.forbid_new_libraries {
        for library in &diff.libraries_added {
            violations.push(format!("New dynamic library: {library}"));
        }
    }
    for abi in &diff.abi {
        if budgets.forbid_abi_bump && abi.increased {
            violations.push(format!(
                "{} runtime requirement increased to {}",
                abi.family,
                abi.head.as_ref().unwrap()
            ));
        }
        let cap = match abi.family.as_str() {
            "GLIBC" => budgets.max_glibc.as_ref(),
            "GLIBCXX" => budgets.max_glibcxx.as_ref(),
            _ => None,
        };
        if let (Some(head), Some(max)) = (&abi.head, cap)
            && head > max
        {
            violations.push(format!("{} {head} exceeds maximum {max}", abi.family));
        }
    }
    anyhow::ensure!(
        diff.format == "Elf" || (budgets.max_glibc.is_none() && budgets.max_glibcxx.is_none()),
        "GLIBC budgets require ELF inputs"
    );
    Ok(GateResult {
        passed: violations.is_empty(),
        violations,
    })
}
