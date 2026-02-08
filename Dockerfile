# This file is part of the Wave language project.
# Copyright (c) 2024–2026 Wave Foundation
# Copyright (c) 2024–2026 LunaStev and contributors
#
# This Source Code Form is subject to the terms of the
# Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file,
# You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    curl \
    gnupg \
    lsb-release \
    sudo \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    wget \
    clang \
    llvm-14 \
    llvm-14-dev \
    clang-14 \
    libclang-14-dev \
    lld-14 \
    && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain 1.86.0
ENV PATH="/root/.cargo/bin:${PATH}"

RUN ln -s /usr/lib/llvm-14/lib/libLLVM-14.so /usr/lib/libllvm-14.so
ENV LLVM_SYS_140_PREFIX=/usr/lib/llvm-14

WORKDIR /wave

COPY . .

CMD ["/bin/bash"]
