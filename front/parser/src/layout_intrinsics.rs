//! Target storage queries used by std::mem::layout.

pub fn is_intrinsic(name: &str) -> bool {
    matches!(name, "__wave_size_of" | "__wave_align_of")
}
