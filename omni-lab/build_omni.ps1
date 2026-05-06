param(
    [switch] $Cuda
)

# Build llama.cpp-omni in the isolated omni-lab folder.

$ErrorActionPreference = "Stop"

$LabRoot = $PSScriptRoot
$RepoDir = Join-Path $LabRoot "llama.cpp-omni"
$BuildDirName = if ($Cuda) { "build-cuda" } else { "build" }
$BuildDir = Join-Path $RepoDir $BuildDirName

if (-not (Test-Path $RepoDir)) {
    throw "Missing runtime repo. Run .\omni-lab\setup_omni_lab.ps1 first."
}

function Import-VsDevEnvironment {
    if (Get-Command cl.exe -ErrorAction SilentlyContinue) {
        return
    }

    $VsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $VsWhere)) {
        return
    }

    $InstallPath = & $VsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $InstallPath) {
        return
    }

    $VsDevCmd = Join-Path $InstallPath "Common7\Tools\VsDevCmd.bat"
    if (-not (Test-Path $VsDevCmd)) {
        return
    }

    Write-Host "[ENV] Loading Visual Studio build environment" -ForegroundColor Cyan
    $envLines = cmd.exe /s /c "`"$VsDevCmd`" -arch=x64 -host_arch=x64 >nul && set"
    foreach ($line in $envLines) {
        if ($line -match "^(.*?)=(.*)$") {
            [Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
        }
    }
}

function Patch-WindowsCompatibility {
    $OmniCli = Join-Path $RepoDir "tools\omni\omni-cli.cpp"
    if (-not (Test-Path $OmniCli)) {
        return
    }

    $Text = Get-Content -LiteralPath $OmniCli -Raw
    if ($Text -match "static void usleep\(uint64_t usec\)") {
        return
    }

    $Needle = @"
#include <windows.h>
#include <signal.h>
#endif
"@
    $Replacement = @"
#include <windows.h>
#include <signal.h>
#include <stdint.h>

static void usleep(uint64_t usec) {
    Sleep((DWORD)((usec + 999) / 1000));
}
#endif
"@

    if (-not $Text.Contains($Needle)) {
        throw "Could not patch Windows usleep compatibility in $OmniCli."
    }

    Write-Host "[PATCH] Adding Windows usleep compatibility" -ForegroundColor Cyan
    $Text.Replace($Needle, $Replacement) | Set-Content -LiteralPath $OmniCli -NoNewline
}

Import-VsDevEnvironment

if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) {
    throw "MSVC compiler cl.exe not found. Install Visual Studio Build Tools with C++ build tools, or run this from Developer PowerShell for Visual Studio."
}

if ($Cuda -and -not (Get-Command nvcc.exe -ErrorAction SilentlyContinue)) {
    throw "CUDA build requested, but nvcc.exe was not found. Install NVIDIA CUDA Toolkit or run without -Cuda for CPU build."
}

Patch-WindowsCompatibility

if (Test-Path (Join-Path $BuildDir "CMakeCache.txt")) {
    Write-Host "[CLEAN] Removing failed CMake cache" -ForegroundColor Yellow
    Remove-Item -LiteralPath $BuildDir -Recurse -Force
}

$CmakeArgs = @(
    "-S", $RepoDir,
    "-B", $BuildDir,
    "-G", "Ninja",
    "-DCMAKE_BUILD_TYPE=Release",
    "-DLLAMA_CURL=OFF"
)

if ($Cuda) {
    Write-Host "[CONFIGURE] llama.cpp-omni CUDA build" -ForegroundColor Cyan
    $CmakeArgs += "-DGGML_CUDA=ON"
} else {
    Write-Host "[CONFIGURE] llama.cpp-omni CPU build" -ForegroundColor Cyan
}

cmake @CmakeArgs
if ($LASTEXITCODE -ne 0) {
    throw "CMake configure failed. Try running from Developer PowerShell for Visual Studio."
}

Write-Host "[BUILD] llama-omni-cli" -ForegroundColor Cyan
cmake --build $BuildDir --target llama-omni-cli --config Release -j
if ($LASTEXITCODE -ne 0) {
    throw "Build failed."
}

Write-Host "[OK] Build complete." -ForegroundColor Green
