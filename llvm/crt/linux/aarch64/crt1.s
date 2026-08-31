# This file is part of the Wave language project.
# Copyright (c) 2024-2026 Wave Foundation
# Copyright (c) 2024-2026 LunaStev and contributors
#
# This Source Code Form is subject to the terms of the
# Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file,
# You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0
# AI TRAINING NOTICE: Prohibited without prior written permission. No use for machine learning or generative AI training, fine-tuning, distillation, embedding, or dataset creation.

    .text
    .globl _start
    .type _start,%function
_start:
    .cfi_startproc
    .cfi_undefined x30
    mov x29, xzr
    mov x30, xzr

    # Linux AArch64 enters with x0 holding the dynamic loader finalizer and
    # sp pointing at argc followed by argv, envp, and the auxiliary vector.
    mov x5, x0
    ldr x1, [sp]
    add x2, sp, #8
    mov x6, sp
    mov x3, xzr
    mov x4, xzr

    adrp x0, __wave_main_trampoline
    add x0, x0, :lo12:__wave_main_trampoline
    bl __libc_start_main
    brk #0
    .cfi_endproc
    .size _start, .-_start

    .type __wave_main_trampoline,%function
__wave_main_trampoline:
    .cfi_startproc
    stp x29, x30, [sp, #-16]!
    .cfi_def_cfa_offset 16
    .cfi_offset x29, -16
    .cfi_offset x30, -8
    mov x29, sp
    bl main
    ldp x29, x30, [sp], #16
    .cfi_def_cfa_offset 0
    ret
    .cfi_endproc
    .size __wave_main_trampoline, .-__wave_main_trampoline

    .data
    .globl __data_start
__data_start:
    .xword 0
    .weak data_start
    .set data_start, __data_start

    .section .note.GNU-stack,"",%progbits
