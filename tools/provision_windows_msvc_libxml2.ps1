# This file is part of the Wave language project.
# SPDX-License-Identifier: MPL-2.0
# AI TRAINING NOTICE: Prohibited without prior written permission.

# llvm-sys 211 cannot dynamically link LLVM on MSVC, even with prefer-dynamic.
# The official LLVM SDKs request xml2s.lib (ARM64) or libxml2s.lib (x64) through --system-libs, but
# does not ship it. Build an ABI-compatible libxml2 2.13 static library from
# checksum-pinned GNOME sources; never substitute a library for another architecture.
param([ValidateSet("arm64", "x64")][string]$Architecture = "arm64")

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE" }
}

function Assert-CoffArchive([string]$Library, [string]$Readobj, [string]$Architecture) {
    # llvm-readobj visits every COFF member, so mixed-architecture archives fail
    # here before Cargo/MSVC reaches a confusing unresolved-symbol diagnostic.
    $headers = & $Readobj --file-headers $Library
    if ($LASTEXITCODE -ne 0) { throw "Could not inspect COFF archive: $Library" }
    $machines = @($headers | Select-String '^\s*Machine:')
    if ($machines.Count -eq 0) { throw "No COFF objects found in archive: $Library" }
    foreach ($machine in $machines) {
        $expected = if ($Architecture -eq 'arm64') { 'IMAGE_FILE_MACHINE_ARM64 \(0xAA64\)' } else { 'IMAGE_FILE_MACHINE_AMD64 \(0x8664\)' }
        if ($machine.Line -notmatch $expected) {
            throw "Wrong-machine archive member: $machine"
        }
    }
    Write-Host "Verified $($machines.Count) $Architecture COFF objects"
}

$hostMachine = if ($Architecture -eq "arm64") { "ARM64" } else { "AMD64" }
$triple = if ($Architecture -eq "arm64") { "aarch64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
if ($env:PROCESSOR_ARCHITECTURE -ne $hostMachine) { throw "Expected a native $Architecture runner" }
$version = "2.13.9"
$sha256 = "a2c9ae7b770da34860050c309f903221c67830c86e4a7e760692b803df95143a"
$root = Join-Path $env:RUNNER_TEMP "wave-libxml2-$Architecture-$version"
$archive = Join-Path $root "libxml2-$version.tar.xz"
$source = Join-Path $root "libxml2-$version"
$build = Join-Path $root "build"
$install = Join-Path $root "install"
New-Item -ItemType Directory -Force -Path $root | Out-Null
Invoke-Checked "curl.exe" @("--fail", "--location", "--retry", "3",
    "https://download.gnome.org/sources/libxml2/2.13/libxml2-$version.tar.xz", "--output", $archive)
if ((Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $sha256) {
    throw "libxml2 source checksum mismatch"
}
Invoke-Checked "tar.exe" @("-xJf", $archive, "-C", $root)

# Import the matching native MSVC/Windows SDK environment for clang-cl and Ninja.
$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
$visualStudio = & $vswhere -latest -products '*' -utf8 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $visualStudio) { throw "Visual Studio was not found" }
$devCmd = Join-Path $visualStudio "Common7\Tools\VsDevCmd.bat"
$environment = & cmd.exe /d /s /c "`"$devCmd`" -no_logo -arch=$Architecture -host_arch=$Architecture >nul && set"
if ($LASTEXITCODE -ne 0) { throw "Could not initialize native $Architecture MSVC" }
foreach ($line in $environment) {
    if ($line -match '^([^=]+)=(.*)$') {
        [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], "Process")
    }
}
if ($env:VSCMD_ARG_TGT_ARCH -ne $Architecture) { throw "MSVC is not targeting $Architecture" }

$llvmBin = Split-Path $env:LLVM_CONFIG_PATH
$clang = Join-Path $llvmBin "clang-cl.exe"
$systemLibraryOutput = & $env:LLVM_CONFIG_PATH --system-libs --link-static
if ($LASTEXITCODE -ne 0) { throw "llvm-config --system-libs failed" }
$systemLibraries = $systemLibraryOutput -join " "
Write-Host "LLVM static system libraries: $systemLibraries"
$libraryName = if ($Architecture -eq "arm64") { "xml2s.lib" } else { "libxml2s.lib" }
if ($systemLibraries -notmatch [regex]::Escape($libraryName)) {
    throw "The pinned LLVM SDK no longer requests $libraryName; review this provisioning contract"
}

$directives = & (Join-Path $llvmBin "llvm-readobj.exe") --coff-directives (Join-Path (Split-Path $llvmBin) "lib\LLVMSupport.lib")
if ($LASTEXITCODE -ne 0) { throw "Could not inspect LLVM CRT directives" }
if (($directives -join " ") -notmatch '(?i)/DEFAULTLIB:libcmt' -or ($directives -join " ") -match '(?i)/DEFAULTLIB:msvcrt') {
    throw "LLVM SDK CRT contract changed; review allocator ownership before building"
}
Write-Host "Verified LLVM static release CRT; Rust debug and release builds use +crt-static"

# /MT matches the inspected pinned LLVM SDK.
# LLVM embeds rpmalloc: mixing dynamic-CRT allocation helpers with its free
# corrupts ownership of LLVM messages. No zlib, lzma, iconv, Python or DLL
# dependencies are introduced. Keep the pre-2.14 XML ABI used by LLVM 21.
Invoke-Checked "cmake" @("-S", $source, "-B", $build, "-G", "Ninja",
    "-DCMAKE_BUILD_TYPE=Release", "-DCMAKE_INSTALL_PREFIX=$install",
    "-DCMAKE_C_COMPILER=$clang", "-DCMAKE_C_COMPILER_TARGET=$triple",
    "-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded", "-DBUILD_SHARED_LIBS=OFF",
    "-DLIBXML2_WITH_ICONV=OFF", "-DLIBXML2_WITH_ICU=OFF", "-DLIBXML2_WITH_LZMA=OFF",
    "-DLIBXML2_WITH_ZLIB=OFF", "-DLIBXML2_WITH_PYTHON=OFF", "-DLIBXML2_WITH_PROGRAMS=OFF",
    "-DLIBXML2_WITH_TESTS=OFF", "-DLIBXML2_WITH_FTP=OFF", "-DLIBXML2_WITH_HTTP=OFF",
    "-DLIBXML2_WITH_MODULES=OFF", "-DLIBXML2_WITH_TLS=OFF")
Invoke-Checked "cmake" @("--build", $build, "--config", "Release", "--parallel", "2")
Invoke-Checked "cmake" @("--install", $build, "--config", "Release")
$libDir = Join-Path $install "lib"
$library = Join-Path $libDir $libraryName
if ($libraryName -ne "libxml2s.lib") { Copy-Item (Join-Path $libDir "libxml2s.lib") $library -Force }

# Compile the same bounded inspector used by Wave before linking LLVM itself.
# SDK ARM64 libraries may have EC-only members in a separate symbol index.
$inspector = Join-Path $root "check-msvc-inputs.exe"
Invoke-Checked "rustc" @("--edition=2021", (Join-Path $PSScriptRoot "check_msvc_inputs.rs"), "-o", $inspector)
Assert-CoffArchive $library (Join-Path $llvmBin "llvm-readobj.exe") $Architecture

# llvm-sys supplies xml2s; libxml2's Windows entropy/socket helpers also use
# these Windows SDK import libraries. RUSTFLAGS overrides target.rustflags in
# Cargo configuration, so repeat the required static CRT flag here as well.
# Preserve other pre-existing flags.
"LIB=$libDir;$env:LIB" | Out-File $env:GITHUB_ENV -Encoding utf8 -Append
"RUSTFLAGS=$env:RUSTFLAGS -C target-feature=+crt-static -l bcrypt -l ws2_32" | Out-File $env:GITHUB_ENV -Encoding utf8 -Append
"WAVE_LIBXML2_LICENSE=$source\Copyright" | Out-File $env:GITHUB_ENV -Encoding utf8 -Append

# Persist the selected native SDK environment for later Cargo and Wave invocations.
foreach ($key in @("VCToolsInstallDir", "WindowsSdkDir", "WindowsSDKVersion", "VSINSTALLDIR", "INCLUDE")) {
    "$key=$([Environment]::GetEnvironmentVariable($key))" | Out-File $env:GITHUB_ENV -Encoding utf8 -Append
}
$nativePaths = @($env:PATH -split ';' | Where-Object { $_ })
if ($Architecture -eq "x64") {
    $nativePaths = @($nativePaths | Where-Object { $_ -notmatch '(?i)mingw|msys' })
    "PATH=$($nativePaths -join ';')" | Out-File $env:GITHUB_ENV -Encoding utf8 -Append
}
# GITHUB_PATH entries are prepended: preserve the Developer Prompt search order.
[array]::Reverse($nativePaths)
$nativePaths | Out-File $env:GITHUB_PATH -Encoding utf8 -Append
$llvmBin | Out-File $env:GITHUB_PATH -Encoding utf8 -Append
# Fail early for every library llvm-config names, not just the XML dependency.
$searchPaths = @($libDir, (Join-Path (Split-Path $llvmBin) "lib")) + @($env:LIB -split ';')
foreach ($name in ($systemLibraries -split '\s+' | Where-Object { $_ })) {
    if ($name -notmatch '^[A-Za-z0-9_.-]+\.lib$') { throw "Unexpected LLVM system library token: $name" }
    $found = @($searchPaths | ForEach-Object { Join-Path $_ $name } | Where-Object { Test-Path $_ -PathType Leaf })
    if (-not $found.Count) { throw "LLVM system library is missing for ${Architecture}: $name" }
    # XML was checked above using readobj: its archive also contains a .res
    # member generated by CMake, which is not a CPU-specific COFF object.
    if ($name -ne $libraryName) {
        Invoke-Checked $inspector @($triple, $found[0])
    }
}
