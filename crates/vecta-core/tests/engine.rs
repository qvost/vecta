use std::collections::BTreeMap;
use vecta_core::*;
fn snapshot() -> Snapshot {
    Snapshot {
        schema_version: SCHEMA_VERSION,
        format: "Elf".into(),
        architecture: "X86_64".into(),
        bits: 64,
        little_endian: true,
        file_size: 100,
        sections: vec![Section {
            name: ".text".into(),
            size: 20,
            file_size: 20,
        }],
        symbols: vec![Symbol {
            raw_name: "test".into(),
            name: "test".into(),
            section: ".text".into(),
            size: 10,
            content_hash: None,
        }],
        libraries: vec!["libc.so.6".into()],
        abi: BTreeMap::from([("GLIBC".into(), "2.9".parse().unwrap())]),
        abi_requirements: vec!["GLIBC_2.9".into()],
        abi_complete: true,
        warnings: vec![],
    }
}
#[test]
fn numeric_versions_and_invalid_versions() {
    assert!("2.34".parse::<Version>().unwrap() > "2.9".parse().unwrap());
    assert_eq!(
        "2.34.0".parse::<Version>().unwrap(),
        "2.34".parse().unwrap()
    );
    for s in ["", "2..3", "2.-1", "GLIBC_2.34", "2.x", " 2.3"] {
        assert!(s.parse::<Version>().is_err());
    }
}
#[test]
fn deterministic_roundtrip_and_self_diff() {
    let base = snapshot();
    let json = serde_json::to_vec(&base).unwrap();
    let copy: Snapshot = serde_json::from_slice(&json).unwrap();
    assert_eq!(base, copy);
    assert_eq!(json, serde_json::to_vec(&copy).unwrap());
    let diff = compare(&base, &copy).unwrap();
    assert_eq!(diff.file.delta, 0);
    assert!(diff.symbols.is_empty());
}
#[test]
fn symbol_categories_and_gate_boundaries() {
    let base = snapshot();
    let mut head = base.clone();
    head.file_size = 110;
    head.sections[0].size = 24;
    head.symbols[0].size = 17;
    let mut added = head.symbols[0].clone();
    added.raw_name = "added".into();
    added.name = "added".into();
    added.size = 2;
    head.symbols.push(added);
    head.libraries.push("libssl.so.3".into());
    head.abi.insert("GLIBC".into(), "2.34".parse().unwrap());
    let diff = compare(&base, &head).unwrap();
    assert_eq!(diff.symbols[0].delta, 7);
    assert_eq!(diff.symbols[0].kind, ChangeKind::Modified);
    assert_eq!(diff.symbols[1].kind, ChangeKind::Added);
    assert!(diff.abi[0].increased);
    let rules = Budgets {
        max_growth: Some(10),
        max_growth_percent: Some(10.0),
        section_growth: BTreeMap::from([(".text".into(), 4)]),
        ..Default::default()
    };
    assert!(evaluate(&diff, &rules).unwrap().passed);
    let rules = Budgets {
        max_growth: Some(9),
        forbid_new_libraries: true,
        max_glibc: Some("2.28".parse().unwrap()),
        ..Default::default()
    };
    assert_eq!(evaluate(&diff, &rules).unwrap().violations.len(), 3);
    let reverse = compare(&head, &base).unwrap();
    assert_eq!(reverse.symbols[1].kind, ChangeKind::Removed);
    assert!(!reverse.abi[0].increased);
}
#[test]
fn zero_sized_additions_are_preserved_and_architectures_rejected() {
    let base = snapshot();
    let mut head = base.clone();
    let mut symbol = head.symbols[0].clone();
    symbol.raw_name = "zero".into();
    symbol.size = 0;
    head.symbols.push(symbol);
    assert_eq!(
        compare(&base, &head).unwrap().symbols[0].kind,
        ChangeKind::Added
    );
    head.architecture = "Aarch64".into();
    assert!(compare(&base, &head).is_err());
}

#[test]
fn missing_abi_never_passes_abi_gate() {
    let mut head = snapshot();
    head.abi_complete = false;
    let diff = compare(&snapshot(), &head).unwrap();
    assert!(
        !evaluate(
            &diff,
            &Budgets {
                max_glibc: Some("99".parse().unwrap()),
                ..Default::default()
            }
        )
        .unwrap()
        .passed
    );
}

#[test]
fn detects_same_size_content_changes() {
    let mut base = snapshot();
    let mut head = snapshot();
    base.symbols[0].content_hash = Some("aaa".into());
    head.symbols[0].content_hash = Some("bbb".into());
    let diff = compare(&base, &head).unwrap();
    assert_eq!(diff.symbols[0].kind, ChangeKind::Modified);
    assert_eq!(diff.symbols[0].delta, 0);
}
