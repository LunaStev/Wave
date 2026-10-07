// SPDX-License-Identifier: MPL-2.0
//! Whale backend for Wave, alongside the LLVM backend crate.
//!
//! This crate owns typed HIR lowering, Whale IR verification and serialization,
//! and the upstream Whale dependency. CLI dispatch and source preparation belong
//! to the compiler driver.

// Source diagnostics retain their complete span at the backend boundary.
#![allow(clippy::result_large_err)]

mod codegen;

pub use codegen::LowerError;

/// Lower verified Wave HIR to verified textual Whale IR.
///
/// Unsupported constructs return a source diagnostic. This backend never
/// delegates lowering to LLVM.
pub fn emit_ir(program: &hir::TypedProgram) -> Result<String, LowerError> {
    codegen::lower(program).map(|module| whale_ir::print_module(&module))
}
