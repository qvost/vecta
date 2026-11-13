use vecta_core::*;

#[test]
fn rejects_non_binary_and_truncated_input() {
    for bytes in [b"".as_slice(), b"not a binary", b"\x7fELF", b"MZ"] {
        assert!(inspect(bytes).is_err());
    }
}

#[test]
fn demangles_cpp_and_rust() {
    assert_eq!(demangle("_Z3fooi"), "foo(int)");
    assert_eq!(demangle("_ZN4test4main17h1234567890abcdefE"), "test::main");
    assert_eq!(demangle("plain_c"), "plain_c");
}

#[test]
fn local_test_binary_is_inspectable() {
    let binary = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let a = inspect(&binary).unwrap();
    let b = inspect(&binary).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.file_size, binary.len() as u64);
    assert!(!a.sections.is_empty());
}

#[test]
fn macho_sections_and_dylib_load_command() {
    let snapshot = inspect(include_bytes!("fixtures/macho64.bin")).unwrap();
    assert_eq!(snapshot.format, "MachO");
    assert_eq!(snapshot.libraries, ["/usr/lib/libSystem.B.dylib"]);
    assert!(
        snapshot
            .sections
            .iter()
            .any(|s| s.name == "__text" && s.size == 1)
    );
}

#[test]
fn pe_sections_and_import_table() {
    let snapshot = inspect(include_bytes!("fixtures/pe64.bin")).unwrap();
    assert_eq!(snapshot.format, "Pe");
    assert_eq!(snapshot.libraries, ["kernel32.dll"]);
    assert!(snapshot.sections.iter().any(|s| s.name == ".text"));
}
