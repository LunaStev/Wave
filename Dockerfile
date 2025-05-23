FROM ubuntu:22.04

# 1. 기본 패키지 설치
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

RUN rustc --version && cargo --version

WORKDIR /wave

COPY . .

CMD ["/bin/bash"]
