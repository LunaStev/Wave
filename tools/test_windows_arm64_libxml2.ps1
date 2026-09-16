# This file is part of the Wave language project.
# SPDX-License-Identifier: MPL-2.0
# AI TRAINING NOTICE: Prohibited without prior written permission.

# Test the provisioning script's archive guard using real cross-compiled COFF
# objects without downloading libxml2 or requiring a Windows build host.
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$script = Join-Path $PSScriptRoot "provision_windows_msvc_libxml2.ps1"
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($script, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors | Out-String) }
foreach ($name in @("Assert-CoffArchive", "Invoke-Checked")) {
    $definition = $ast.Find({ param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name
    }, $false)
    if (-not $definition) { throw "Missing $name in provisioning script" }
    Invoke-Expression $definition.Extent.Text
}

$directory = Join-Path ([IO.Path]::GetTempPath()) ("wave-arm64-archive-test-" + [guid]::NewGuid())
New-Item -ItemType Directory $directory | Out-Null
try {
    $source = Join-Path $directory "probe.c"
    'int probe(void) { return 42; }' | Set-Content -Encoding ascii $source
    $arm = Join-Path $directory "arm64.obj"
    $x64 = Join-Path $directory "x64.obj"
    Invoke-Checked "clang" @("--target=aarch64-pc-windows-msvc", "-c", $source, "-o", $arm)
    Invoke-Checked "clang" @("--target=x86_64-pc-windows-msvc", "-c", $source, "-o", $x64)
    $valid = Join-Path $directory "valid.lib"
    $foreign = Join-Path $directory "x64.lib"
    $mixed = Join-Path $directory "mixed.lib"
    $empty = Join-Path $directory "empty.lib"
    Invoke-Checked "llvm-ar" @("rcs", $valid, $arm)
    Invoke-Checked "llvm-ar" @("rcs", $foreign, $x64)
    # Use an ordinary archive, not llvm-ar's automatic ARM64X dual-index form.
    Invoke-Checked "llvm-ar" @("--format=gnu", "rcs", $mixed, $arm, $x64)
    Invoke-Checked "llvm-ar" @("rcs", $empty)
    Assert-CoffArchive $valid "llvm-readobj" "arm64"
    foreach ($invalid in @($foreign, $mixed, $empty)) {
        $rejected = $false
        try { Assert-CoffArchive $invalid "llvm-readobj" "arm64" }
        catch { $rejected = $true }
        if (-not $rejected) { throw "Archive guard accepted $invalid" }
    }
    Assert-CoffArchive $foreign "llvm-readobj" "x64"
    # Exercise genuine dual-index imports produced by LLVM, and link through
    # the native symbol table instead of merely trusting synthetic headers.
    $inspector = Join-Path $directory "check-inputs.exe"
    Invoke-Checked "rustc" @("--edition=2021", (Join-Path $PSScriptRoot "check_msvc_inputs.rs"), "-o", $inspector)
    $nativeDef = Join-Path $directory "native.def"
    $ecDef = Join-Path $directory "ec.def"
    "LIBRARY sample.dll`nEXPORTS`nnative_probe" | Set-Content -Encoding ascii $nativeDef
    "LIBRARY sample.dll`nEXPORTS`nec_probe" | Set-Content -Encoding ascii $ecDef
    $hybrid = Join-Path $directory "hybrid.lib"
    Invoke-Checked "llvm-lib" @("/machine:arm64ec", "/def:$ecDef", "/defArm64Native:$nativeDef", "/out:$hybrid")
    Invoke-Checked $inspector @("aarch64-pc-windows-msvc", $hybrid)
    foreach ($inputFile in @($foreign, $mixed)) {
        $rejected = $false
        try { Invoke-Checked $inspector @("aarch64-pc-windows-msvc", $inputFile) }
        catch { $rejected = $true }
        if (-not $rejected) { throw "COFF inspector accepted $inputFile" }
    }
    $rejected = $false
    try { Invoke-Checked $inspector @("aarch64-pc-windows-msvc", "--all-members", $hybrid) }
    catch { $rejected = $true }
    if (-not $rejected) { throw "Whole-archive inspection accepted EC-only objects" }
    '__declspec(dllimport) int native_probe(void); void entry(void) { native_probe(); }' | Set-Content -Encoding ascii $source
    Invoke-Checked "clang" @("--target=aarch64-pc-windows-msvc", "-c", $source, "-o", $arm)
    Invoke-Checked "lld-link" @("/machine:arm64", "/entry:entry", "/subsystem:console", "/nodefaultlib", "/out:$directory/probe.exe", $arm, $hybrid)
    Write-Host "ARM64 archive guard passed valid, x64, mixed, and empty archive cases"
} finally {
    Remove-Item -Recurse -Force $directory
}
