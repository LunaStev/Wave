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
    .cfi_undefined ra
    call .Lwave_load_gp

    # Linux RISC-V enters with a0 holding the dynamic loader finalizer and
    # sp pointing at argc followed by argv, envp, and the auxiliary vector.
    mv a5, a0
    lla a0, __wave_main_trampoline
    ld a1, 0(sp)
    addi a2, sp, 8
    andi sp, sp, -16
    li a3, 0
    li a4, 0
    mv a6, sp
    call __libc_start_main@plt
    ebreak
    .cfi_endproc
    .size _start, .-_start

    .type __wave_main_trampoline,@function
__wave_main_trampoline:
    .cfi_startproc
    addi sp, sp, -16
    .cfi_def_cfa_offset 16
    sd ra, 8(sp)
    .cfi_offset ra, -8
    call main
    ld ra, 8(sp)
    addi sp, sp, 16
    .cfi_def_cfa_offset 0
    ret
    .cfi_endproc
    .size __wave_main_trampoline, .-__wave_main_trampoline

.Lwave_load_gp:
    .option push
    .option norelax
    lla gp, __global_pointer$
    .option pop
    ret

    .section .preinit_array,"aw",@preinit_array
    .p2align 3
    .dword .Lwave_load_gp

    .data
    .globl __data_start
__data_start:
    .dword 0
    .weak data_start
    .set data_start, __data_start

    .section .note.GNU-stack,"",@progbits
