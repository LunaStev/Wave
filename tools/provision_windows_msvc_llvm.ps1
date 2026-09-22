# SPDX-License-Identifier: MPL-2.0
param([ValidateSet("arm64", "x64")][string]$Architecture)
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$version = "21.1.8"
$hostMachine = if ($Architecture -eq "arm64") { "ARM64" } else { "AMD64" }
if ($env:PROCESSOR_ARCHITECTURE -ne $hostMachine) { throw "Expected native $Architecture host" }
$triple = if ($Architecture -eq "arm64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
$sha = if ($Architecture -eq "arm64") {
    "f214b1226d8de005b5f691dd29d9dfea2b49e22d0de445429916173dbb626f7f"
} else {
    "749d22f565fcd5718dbed06512572d0e5353b502c03fe1f7f17ee8b8aca21a47"
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
    @("llvm", "8d85c1057d742e597985c7d4e6320b015a9139385cff4cbae06ffc0ebe89afee"),
    @("compiler-rt", "1a8f1058753f1ba890de984e48f0242a3a5c29a6a8f2ed9fd813f36985387e8d")
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
