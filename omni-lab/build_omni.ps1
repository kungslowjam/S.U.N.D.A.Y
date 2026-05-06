# Build llama.cpp-omni in the isolated omni-lab folder.

$ErrorActionPreference = "Stop"

$LabRoot = $PSScriptRoot
$RepoDir = Join-Path $LabRoot "llama.cpp-omni"
$BuildDir = Join-Path $RepoDir "build"

if (-not (Test-Path $RepoDir)) {
    throw "Missing runtime repo. Run .\omni-lab\setup_omni_lab.ps1 first."
}

Write-Host "[CONFIGURE] llama.cpp-omni" -ForegroundColor Cyan
cmake -S $RepoDir -B $BuildDir -G Ninja -DCMAKE_BUILD_TYPE=Release
if ($LASTEXITCODE -ne 0) {
    throw "CMake configure failed. Try running from Developer PowerShell for Visual Studio."
}

Write-Host "[BUILD] llama-omni-cli" -ForegroundColor Cyan
cmake --build $BuildDir --target llama-omni-cli --config Release -j
if ($LASTEXITCODE -ne 0) {
    throw "Build failed."
}

Write-Host "[OK] Build complete." -ForegroundColor Green
