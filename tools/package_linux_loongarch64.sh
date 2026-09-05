#!/usr/bin/env bash

# This file is part of the Wave language project.
# SPDX-License-Identifier: MPL-2.0
# AI TRAINING NOTICE: Prohibited without prior written permission.

set -euo pipefail

release_version="${1:?usage: package_linux_loongarch64.sh <version>}"

llvm_version="${LLVM_SOURCE_VERSION:-21.1.8}"
llvm_sha256="${LLVM_SOURCE_SHA256:-4633a23617fa31a3ea51242586ea7fb1da7140e426bd62fc164261fe036aa142}"
toolchain_version="${LOONGARCH_TOOLCHAIN_VERSION:-2025.08.08}"
toolchain_archive="${LOONGARCH_TOOLCHAIN_ARCHIVE:-x86_64-cross-tools-loongarch64-binutils_2.45-gcc_15.1.0-glibc_2.42.tar.xz}"
toolchain_sha256="${LOONGARCH_TOOLCHAIN_SHA256:-b8572e2083143ff1807658f02e11eba53e5ed81d6194854d369b43fceea72de7}"

build_root="${WAVE_LOONGARCH64_BUILD_ROOT:-/tmp/wave-loongarch64-release}"
download_root="$build_root/downloads"
source_root="$build_root/llvm-project-$llvm_version.src"
llvm_build="$build_root/llvm-build"
toolchain_root="$build_root/toolchain"
toolchain="${WAVE_LOONGARCH64_TOOLCHAIN:-$toolchain_root/cross-tools}"
sysroot="$toolchain/target"
target_triple="loongarch64-unknown-linux-gnu"
cross_prefix="$toolchain/bin/$target_triple"
host_llvm_config="${WAVE_HOST_LLVM_CONFIG:-$(command -v llvm-config)}"
host_llvm_prefix="$("$host_llvm_config" --prefix)"
host_llvm_bin="$host_llvm_prefix/bin"

export CARGO_BUILD_JOBS=2
mkdir -p "$download_root" "$toolchain_root"

llvm_archive="$download_root/llvm-project-$llvm_version.src.tar.xz"
if [[ ! -f "$llvm_archive" ]]; then
  curl --proto '=https' --tlsv1.2 --fail --location --retry 3 \
    "https://github.com/llvm/llvm-project/releases/download/llvmorg-$llvm_version/llvm-project-$llvm_version.src.tar.xz" \
    --output "$llvm_archive"
fi
echo "$llvm_sha256  $llvm_archive" | sha256sum --check

if [[ ! -d "$source_root/llvm" ]]; then
  tar -xJf "$llvm_archive" -C "$build_root"
fi
if [[ ! -x "$cross_prefix-gcc" ]]; then
  cross_archive="$download_root/$toolchain_archive"
  if [[ ! -f "$cross_archive" ]]; then
    curl --proto '=https' --tlsv1.2 --fail --location --retry 3 \
      "https://github.com/loongson/build-tools/releases/download/$toolchain_version/$toolchain_archive" \
      --output "$cross_archive"
  fi
  echo "$toolchain_sha256  $cross_archive" | sha256sum --check
  tar -xJf "$cross_archive" -C "$toolchain_root"
fi

test -x "$cross_prefix-gcc"
test -x "$cross_prefix-g++"
test -f "$sysroot/usr/lib64/ld-linux-loongarch-lp64d.so.1"
test "$("$host_llvm_config" --version | cut -d. -f1)" = 21
test -x "$host_llvm_bin/llvm-mc"
test -x "$host_llvm_bin/llvm-tblgen"

cmake -S "$source_root/llvm" -B "$llvm_build" -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_SYSTEM_NAME=Linux \
  -DCMAKE_SYSTEM_PROCESSOR=loongarch64 \
  -DCMAKE_C_COMPILER="$cross_prefix-gcc" \
  -DCMAKE_CXX_COMPILER="$cross_prefix-g++" \
  -DCMAKE_AR="$cross_prefix-ar" \
  -DCMAKE_RANLIB="$cross_prefix-ranlib" \
  -DCMAKE_STRIP="$cross_prefix-strip" \
  -DCMAKE_EXE_LINKER_FLAGS='-static-libgcc -static-libstdc++ -Wl,-rpath,$ORIGIN/../lib' \
  -DCMAKE_SHARED_LINKER_FLAGS='-static-libgcc -static-libstdc++' \
  -DLLVM_NATIVE_TOOL_DIR="$host_llvm_bin" \
  -DLLVM_HOST_TRIPLE="$target_triple" \
  -DLLVM_DEFAULT_TARGET_TRIPLE="$target_triple" \
  -DLLVM_TARGETS_TO_BUILD='AArch64;LoongArch;RISCV;WebAssembly;X86' \
  -DLLVM_ENABLE_PROJECTS=lld \
  -DLLVM_BUILD_LLVM_DYLIB=ON \
  -DLLVM_LINK_LLVM_DYLIB=ON \
  -DLLVM_BUILD_TOOLS=ON \
  -DLLVM_INCLUDE_TESTS=OFF \
  -DLLVM_INCLUDE_EXAMPLES=OFF \
  -DLLVM_INCLUDE_BENCHMARKS=OFF \
  -DLLVM_INCLUDE_DOCS=OFF \
  -DLLVM_ENABLE_BINDINGS=OFF \
  -DLLVM_ENABLE_FFI=OFF \
  -DLLVM_ENABLE_LIBEDIT=OFF \
  -DLLVM_ENABLE_LIBXML2=OFF \
  -DLLVM_ENABLE_TERMINFO=OFF \
  -DLLVM_ENABLE_ZLIB=OFF \
  -DLLVM_ENABLE_ZSTD=OFF

cmake --build "$llvm_build" --parallel 2 \
  --target llvm-config llc llvm-as llvm-mc ld.lld

for tool in llvm-config llc llvm-as llvm-mc ld.lld; do
  test -x "$llvm_build/bin/$tool"
  file "$llvm_build/bin/$tool" | grep -Fq 'LoongArch'
done
test -n "$(find "$llvm_build/lib" -maxdepth 1 -type f -name 'libLLVM*.so*' -print -quit)"

llvm_config_wrapper="$build_root/llvm-config-loongarch64"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  "exec qemu-loongarch64 -L '$sysroot' '$llvm_build/bin/llvm-config' \"\$@\"" \
  > "$llvm_config_wrapper"
chmod +x "$llvm_config_wrapper"

rustup target add "$target_triple"

export WAVE_CROSS_LLVM_TARGET="$target_triple"
export WAVE_LOONGARCH64_SYSROOT="$sysroot"
export WAVE_LLVM_HOME="$llvm_build"
export WAVE_LLVM_BIN="$llvm_build/bin"
export WAVE_LLVM_LIB="$llvm_build/lib"
export WAVE_HOST_LLVM_MC="$host_llvm_bin/llvm-mc"
export WAVE_LLVM_MC="$host_llvm_bin/llvm-mc"
export LLVM_SYS_211_PREFIX="$llvm_build"
export LLVM_CONFIG_PATH="$llvm_config_wrapper"
export CARGO_TARGET_LOONGARCH64_UNKNOWN_LINUX_GNU_LINKER="$cross_prefix-gcc"
export CC_loongarch64_unknown_linux_gnu="$cross_prefix-gcc"
export CXX_loongarch64_unknown_linux_gnu="$cross_prefix-g++"

python3 x.py release "$target_triple"

archive="wave-v${release_version}-loongarch64-linux-gnu.tar.gz"
package="wave-v${release_version}-loongarch64-linux-gnu"
test -f "$archive"

temporary_dir="$(mktemp -d)"
trap 'rm -rf "$temporary_dir"' EXIT
tar -xzf "$archive" -C "$temporary_dir"

file "$temporary_dir/$package/wavec" | grep -Fq 'LoongArch'
"$cross_prefix-readelf" -d "$temporary_dir/$package/wavec" \
  | grep -Fq 'Library rpath: [$ORIGIN/llvm/lib]'

if [[ -r /proc/sys/fs/binfmt_misc/qemu-loongarch64 ]]; then
  env -i \
    PATH=/usr/bin:/bin \
    HOME=/tmp \
    QEMU_LD_PREFIX="$sysroot" \
    WAVE_LOONGARCH64_SYSROOT="$sysroot" \
    "$temporary_dir/$package/wavec" -V

  printf 'fun main() { println("release smoke"); }\n' > "$temporary_dir/smoke.wave"
  env -i \
    PATH=/usr/bin:/bin \
    HOME=/tmp \
    QEMU_LD_PREFIX="$sysroot" \
    WAVE_LOONGARCH64_SYSROOT="$sysroot" \
    "$temporary_dir/$package/wavec" run "$temporary_dir/smoke.wave" \
    | grep -Fx 'release smoke'
elif [[ "${WAVE_REQUIRE_LOONGARCH64_BINFMT:-0}" == 1 ]]; then
  echo 'LoongArch64 binfmt registration is required for the native compiler smoke test' >&2
  exit 1
else
  qemu-loongarch64 -L "$sysroot" "$temporary_dir/$package/wavec" -V
fi

for abi_flag in 'lp64s:0x41' 'lp64f:0x42' 'lp64d:0x43'; do
  abi="${abi_flag%%:*}"
  flag="${abi_flag##*:}"
  crt="$temporary_dir/$package/crt/$target_triple/$abi/crt1.o"
  test -f "$crt"
  "$cross_prefix-readelf" -h "$crt" | grep -Eq "Flags:[[:space:]]+$flag"
done

sha256sum "$archive" > "$archive.sha256"
