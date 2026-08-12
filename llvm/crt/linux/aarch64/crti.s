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

    .section .init,"ax",%progbits
    .globl _init
    .hidden _init
    .type _init,%function
_init:
    ret
    .size _init, .-_init

    .section .fini,"ax",%progbits
    .globl _fini
    .hidden _fini
    .type _fini,%function
_fini:
    ret
    .size _fini, .-_fini

    .section .note.GNU-stack,"",%progbits
