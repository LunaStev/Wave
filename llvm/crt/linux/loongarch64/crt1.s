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
    .type _start,@function
_start:
    .cfi_startproc
    .cfi_undefined 1

    # Linux LoongArch64 enters with a0 holding the dynamic loader finalizer
    # and sp pointing at argc followed by argv, envp, and the auxiliary vector.
    or $a5, $a0, $zero
    la.pcrel $a0, $t0, __wave_main_trampoline
    ld.d $a1, $sp, 0
    addi.d $a2, $sp, 8
    bstrins.d $sp, $zero, 3, 0
    move $a3, $zero
    move $a4, $zero
    or $a6, $sp, $zero
    la.pcrel $ra, $t0, __libc_start_main
    jirl $ra, $ra, 0
    break 0
    .cfi_endproc
    .size _start, .-_start

    .type __wave_main_trampoline,@function
__wave_main_trampoline:
    .cfi_startproc
    addi.d $sp, $sp, -16
    .cfi_def_cfa_offset 16
    st.d $ra, $sp, 8
    .cfi_offset 1, -8
    bl main
    ld.d $ra, $sp, 8
    addi.d $sp, $sp, 16
    .cfi_def_cfa_offset 0
    jirl $zero, $ra, 0
    .cfi_endproc
    .size __wave_main_trampoline, .-__wave_main_trampoline

    .data
    .globl __data_start
__data_start:
    .dword 0
    .weak data_start
    .set data_start, __data_start

    .section .note.GNU-stack,"",@progbits
