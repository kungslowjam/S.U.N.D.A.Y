$ErrorActionPreference = "Stop"

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$LabRoot = $PSScriptRoot
$RepoUrl = "https://github.com/AtomicBot-ai/atomic-llama-cpp-turboquant.git"
$RepoPath = Join-Path $LabRoot "atomic-llama-cpp-turboquant"
$Cuda = if ($env:SUNDAY_MTP_CUDA) { $env:SUNDAY_MTP_CUDA } else { "ON" }
$Generator = if ($env:SUNDAY_MTP_CMAKE_GENERATOR) {
    $env:SUNDAY_MTP_CMAKE_GENERATOR
} elseif (Get-Command ninja -ErrorAction SilentlyContinue) {
    "Ninja"
} else {
    "Visual Studio 17 2022"
}
$BuildPath = if ($env:SUNDAY_MTP_BUILD_DIR) {
    $env:SUNDAY_MTP_BUILD_DIR
} elseif ($Generator -eq "Ninja") {
    Join-Path $LabRoot "build-ninja"
} else {
    Join-Path $LabRoot "build"
}
$CudaCompiler = if ($env:CUDACXX) {
    $env:CUDACXX
} elseif (Test-Path "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.4\bin\nvcc.exe") {
    "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.4\bin\nvcc.exe"
} else {
    $null
}

function Assert-Command {
    param([string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name was not found in PATH."
    }
}

Assert-Command git
Assert-Command cmake

if (-not (Test-Path $RepoPath)) {
    Write-Host "[MTP] Cloning Atomic fork..." -ForegroundColor Cyan
    git clone $RepoUrl $RepoPath
} else {
    Write-Host "[MTP] Updating Atomic fork..." -ForegroundColor Cyan
    git -C $RepoPath pull --ff-only
}

New-Item -ItemType Directory -Force -Path $BuildPath | Out-Null

Write-Host "[MTP] Configuring build (generator=$Generator, GGML_CUDA=$Cuda)..." -ForegroundColor Cyan
$ConfigureArgs = @(
    "-S", $RepoPath,
    "-B", $BuildPath,
    "-G", $Generator,
    "-DGGML_CUDA=$Cuda",
    "-DCMAKE_BUILD_TYPE=Release"
)
if ($Cuda -eq "ON" -and $CudaCompiler) {
    $ConfigureArgs += "-DCMAKE_CUDA_COMPILER=$CudaCompiler"
    $ConfigureArgs += "-DCMAKE_CUDA_ARCHITECTURES=89"
}
cmake @ConfigureArgs
if ($LASTEXITCODE -ne 0) {
    throw "CMake configure failed. If Ninja fails, try running from Developer PowerShell or set SUNDAY_MTP_CMAKE_GENERATOR='Visual Studio 17 2022'."
}

Write-Host "[MTP] Building llama-server..." -ForegroundColor Cyan
cmake --build $BuildPath --config Release --target llama-server -j
if ($LASTEXITCODE -ne 0) {
    throw "Build failed."
}

$ServerCandidates = @(
    (Join-Path $BuildPath "bin\Release\llama-server.exe"),
    (Join-Path $BuildPath "bin\llama-server.exe")
)
$ServerPath = $ServerCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $ServerPath) {
    throw "llama-server.exe was not found under $BuildPath."
}

Write-Host "[MTP] Build ready: $ServerPath" -ForegroundColor Green
