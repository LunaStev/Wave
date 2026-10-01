# SPDX-License-Identifier: MPL-2.0
param([ValidateSet("arm64", "x64")][string]$Architecture)
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$pins = Get-Content (Join-Path $PSScriptRoot "ci/toolchains.json") -Raw | ConvertFrom-Json
$version = $pins.LLVM_SOURCE_VERSION
$hostMachine = if ($Architecture -eq "arm64") { "ARM64" } else { "AMD64" }
if ($env:PROCESSOR_ARCHITECTURE -ne $hostMachine) { throw "Expected native $Architecture host" }
$triple = if ($Architecture -eq "arm64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
$sha = if ($Architecture -eq "arm64") {
    $pins.WINDOWS_LLVM_ARM64_SHA256
} else {
    $pins.WINDOWS_LLVM_X64_SHA256
}
$archive = Join-Path $env:RUNNER_TEMP "llvm-$Architecture.tar.xz"
$directory = Join-Path $env:RUNNER_TEMP "llvm-$Architecture"
$url = "https://github.com/llvm/llvm-project/releases/download/llvmorg-$version/clang%2Bllvm-$version-$triple.tar.xz"
& curl.exe --fail --location --retry 3 $url --output $archive
if ($LASTEXITCODE -ne 0) { throw "LLVM download failed" }
if ((Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $sha) { throw "LLVM checksum mismatch" }
New-Item -ItemType Directory -Force -Path $directory | Out-Null
& tar.exe -xJf $archive --strip-components=1 -C $directory
if ($LASTEXITCODE -ne 0) { throw "LLVM extraction failed" }
# Keep the exact notices with the SDK even when the upstream binary archive
# omits its source-tree licenses. Staging never substitutes an unpinned file.
$notices = @(
    @("llvm", $pins.LLVM_NOTICE_SHA256),
    @("compiler-rt", $pins.COMPILER_RT_NOTICE_SHA256)
)
$noticeDirectory = Join-Path $directory "wave-notices"
New-Item -ItemType Directory -Force -Path $noticeDirectory | Out-Null
foreach ($notice in $notices) {
    $path = Join-Path $noticeDirectory "$($notice[0]).txt"
    $url = "https://raw.githubusercontent.com/llvm/llvm-project/llvmorg-$version/$($notice[0])/LICENSE.TXT"
    & curl.exe --fail --location --retry 3 $url --output $path
    if ($LASTEXITCODE -ne 0) { throw "LLVM notice download failed: $url" }
    if ((Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $notice[1]) {
        throw "LLVM notice checksum mismatch: $path"
    }
}
@(
    "WAVE_LLVM_HOME=$directory"
    "WAVE_WINDOWS_LLVM_BIN=$directory\bin"
    "LLVM_SYS_211_PREFIX=$directory"
    "LLVM_CONFIG_PATH=$directory\bin\llvm-config.exe"
) | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
"$directory\bin" | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
