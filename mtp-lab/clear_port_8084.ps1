$ErrorActionPreference = "Stop"

$Port = if ($env:SUNDAY_MTP_PORT) { [int]$env:SUNDAY_MTP_PORT } else { 8084 }
$pids = Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue |
    Where-Object { $_.OwningProcess -ne 0 } |
    Select-Object -ExpandProperty OwningProcess -Unique

if (-not $pids) {
    Write-Host "[PORT] No process found on $Port" -ForegroundColor DarkGray
    return
}

foreach ($processId in $pids) {
    $proc = Get-Process -Id $processId -ErrorAction SilentlyContinue
    if ($proc) {
        Write-Host "[PORT] Stopping $($proc.ProcessName) ($processId) on $Port" -ForegroundColor Yellow
        Stop-Process -Id $processId -Force
    }
}
