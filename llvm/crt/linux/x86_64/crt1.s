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
    .cfi_undefined %rip
    xorl %ebp, %ebp

    # Linux x86_64 enters with:
    #   rdx = dynamic loader finalizer
    #   rsp = argc, argv..., NULL, envp..., NULL, auxv...
    #
    # Use glibc's hosted-program initializer rather than calling main
    # directly. This keeps libc initialization, stdio flushing, TLS setup,
    # argv/envp handling, and exit processing on the normal hosted path.
    movq %rdx, %r9
    popq %rsi
    movq %rsp, %rdx
    andq $-16, %rsp
    pushq %rax
    pushq %rsp
    xorl %r8d, %r8d
    xorl %ecx, %ecx
    leaq __wave_main_trampoline(%rip), %rdi
    call __libc_start_main@PLT
    hlt
    .cfi_endproc

    .size _start, .-_start

    .type __wave_main_trampoline,@function
__wave_main_trampoline:
    .cfi_startproc
    subq $8, %rsp
    .cfi_adjust_cfa_offset 8
    call main
    addq $8, %rsp
    .cfi_adjust_cfa_offset -8
    xorl %eax, %eax
    ret
    .cfi_endproc

    .size __wave_main_trampoline, .-__wave_main_trampoline

    .section .note.GNU-stack,"",@progbits
