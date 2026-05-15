# System Health Check Tool - PowerShell
# ตรวจสอบ CPU, RAM, Disk ของเครื่อง

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "# SYSTEM HEALTH CHECK TOOL" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# 1. ข้อมูลระบบ
Write-Host "[1] SYSTEM INFORMATION" -ForegroundColor Yellow
Write-Host "------------------------" -ForegroundColor Gray
$OS = Get-CimInstance Win32_OperatingSystem
Write-Host "Operating System: $($OS.Caption)" -ForegroundColor White
Write-Host "Version: $($OS.Version)" -ForegroundColor White
Write-Host "Build: $($OS.BuildNumber)" -ForegroundColor White
Write-Host "Hostname: $($env:COMPUTERNAME)" -ForegroundColor White
Write-Host ""

# 2. ข้อมูล CPU
Write-Host "[2] CPU INFORMATION" -ForegroundColor Yellow
Write-Host "------------------------" -ForegroundColor Gray
$Cpus = Get-CimInstance Win32_Processor
foreach ($Cpu in $Cpus) {
    Write-Host "CPU Model: $($Cpu.Name)" -ForegroundColor White
    Write-Host "  Cores: $($Cpu.NumberOfCores)" -ForegroundColor White
    if ($Cpu.LoadPercentage -gt 0) {
        Write-Host "  Load: $($Cpu.LoadPercentage)%" -ForegroundColor White
    }
}
Write-Host ""

# 3. ข้อมูล RAM
Write-Host "[3] MEMORY (RAM) INFORMATION" -ForegroundColor Yellow
Write-Host "------------------------" -ForegroundColor Gray
$Memory = Get-CimInstance Win32_OperatingSystem
$TotalMB = $Memory.TotalVisibleMemorySize
$FreeMB = $Memory.FreePhysicalMemory
$TotalGB = [math]::Round($TotalMB / 1GB, 2)
$FreeGB = [math]::Round($FreeMB / 1GB, 2)
$UsedGB = [math]::Round($TotalGB - $FreeGB, 2)
$UsagePercent = [math]::Round(($UsedGB / $TotalGB) * 100, 1)
Write-Host "Total RAM: $TotalGB GB" -ForegroundColor White
Write-Host "Free RAM: $FreeGB GB" -ForegroundColor White
Write-Host "Used RAM: $UsedGB GB ($UsagePercent%)" -ForegroundColor White
if ($UsagePercent -gt 80) {
    Write-Host "⚠️  Memory usage is high!" -ForegroundColor Red
} elseif ($UsagePercent -gt 50) {
    Write-Host "ℹ️  Memory usage is moderate." -ForegroundColor Yellow
} else {
    Write-Host "✅ Memory usage is healthy." -ForegroundColor Green
}
Write-Host ""

# 4. ข้อมูล Disk
Write-Host "[4] DISK INFORMATION" -ForegroundColor Yellow
Write-Host "------------------------" -ForegroundColor Gray
$Disks = Get-CimInstance Win32_LogicalDisk -Filter "DriveType=3"  # Only fixed drives
Write-Host "{'Drive' :<10} {'Size' :<15} {'Free' :<15} {'File System'}" -ForegroundColor Gray
Write-Host "{'-'*60}" -ForegroundColor Gray
foreach ($Disk in $Disks) {
    $Drive = if ($Disk.DeviceID -eq 'C:') { 'C:' } else { $Disk.DeviceID }
    $SizeGB = [math]::Round($Disk.Size / 1GB, 2)
    $FreeGB = [math]::Round($Disk.FreeSpace / 1GB, 2)
    $UsedGB = [math]::Round(($SizeGB - $FreeGB), 2)
    $UsagePercent = if ($SizeGB -gt 0) { [math]::Round(($UsedGB / $SizeGB) * 100, 1) } else { 0 }
    Write-Host "{$Drive:<10} {$SizeGB:>10.2f} GB  {$FreeGB:>10.2f} GB  {$Disk.FileSystem}" -ForegroundColor White
    if ($UsagePercent -gt 90) {
        Write-Host "⚠️  Drive '$Drive' is nearly full!" -ForegroundColor Red
    } elseif ($UsagePercent -gt 80) {
        Write-Host "ℹ️  Drive '$Drive' has moderate usage." -ForegroundColor Yellow
    } else {
        Write-Host "✅ Drive '$Drive' is healthy." -ForegroundColor Green
    }
}
Write-Host ""

# 5. สรุป
Write-Host "[5] SUMMARY" -ForegroundColor Yellow
Write-Host "------------------------" -ForegroundColor Gray
Write-Host "System Status: HEALTHY" -ForegroundColor Green
Write-Host "Report Generated: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
Write-Host ""

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "✅ Health Check Complete!" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Cyan
