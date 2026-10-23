//! Payload lowering on every target advertised by the enabled LLVM backends.
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn payload_variants_emit_ir_and_objects_for_every_advertised_target() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory =
        std::env::temp_dir().join(format!("wave-payload-targets-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let compiler = env!("CARGO_BIN_EXE_wavec");
    let listed = Command::new(compiler).args(["print", "target-list"]).output().unwrap();
    assert!(listed.status.success());
    let listed = String::from_utf8(listed.stdout).unwrap();
    let advertised: BTreeSet<_> = listed.lines().collect();
    let specifications = llvm::codegen::target::supported_target_specs();
    assert!(!specifications.is_empty(), "no enabled LLVM backend was exercised");
    assert_eq!(advertised, specifications.iter().map(|spec| spec.triple).collect());
    for spec in specifications {
        // test87 covers tags, generics, nested payloads, pointer recursion and
        // struct/array/global interactions. test126 covers compact layout and
        // bit-preserving float/pointer/wide integer constant and static storage.
        for case in ["test87", "test126"] {
            let destination = directory.join(spec.triple).join(case);
            let result = Command::new(compiler)
                .arg("build")
                .arg(root.join(format!("tests/cases/shared/{case}.wave")))
                .args(["--target", spec.triple, "--emit=ir,obj", "-O0", "--out-dir"])
                .arg(&destination)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{case} {}: {}\n{}",
                spec.triple,
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            let ir = fs::read_to_string(destination.join(format!("{case}.ll"))).unwrap();
            let triple = llvm::codegen::target::llvm_triple_for_abi(spec.triple, spec.default_abi);
            assert!(ir.contains(&format!("target triple = \"{triple}\"")), "{}: {ir}", spec.triple);
            assert!(
                ir.contains("target datalayout = \"") && ir.contains("%variant."),
                "{}: {ir}",
                spec.triple
            );
            let object = fs::read(destination.join(format!("{case}.o"))).unwrap();
            match spec.object_format {
                "elf" => assert!(object.starts_with(b"\x7fELF"), "{}", spec.triple),
                "macho" => {
                    assert!(object.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]), "{}", spec.triple)
                },
                "coff" => {
                    let machine = if spec.triple.starts_with("aarch64") {
                        [0x64, 0xaa]
                    } else {
                        [0x64, 0x86]
                    };
                    assert!(object.starts_with(&machine), "{}", spec.triple);
                },
                "wasm" => assert!(object.starts_with(b"\0asm"), "{}", spec.triple),
                other => panic!("unverified object format {other} for {}", spec.triple),
            }
        }
    }
    fs::remove_dir_all(directory).unwrap();
}
