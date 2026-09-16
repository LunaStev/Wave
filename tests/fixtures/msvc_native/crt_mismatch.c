// SPDX-License-Identifier: MPL-2.0
// Require the linker to diagnose an explicitly incompatible runtime contract.
#pragma detect_mismatch("RuntimeLibrary", "MD_DynamicRelease")
int native_crt_marker(void) {
    return 1;
}
