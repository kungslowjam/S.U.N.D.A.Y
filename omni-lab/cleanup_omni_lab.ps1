# Remove generated omni-lab assets. Leaves scripts and README intact.

$ErrorActionPreference = "Stop"

$Targets = @(
    (Join-Path $PSScriptRoot "llama.cpp-omni"),
    (Join-Path $PSScriptRoot "models"),
    (Join-Path $PSScriptRoot "output"),
    (Join-Path $PSScriptRoot "build-logs")
)

foreach ($Target in $Targets) {
    if (Test-Path $Target) {
        Write-Host "[REMOVE] $Target" -ForegroundColor Yellow
        Remove-Item -LiteralPath $Target -Recurse -Force
    }
}

Write-Host "[OK] Omni lab generated files removed." -ForegroundColor Green
