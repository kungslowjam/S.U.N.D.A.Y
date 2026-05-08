$ErrorActionPreference = "Stop"

$Port = if ($env:SUNDAY_MTP_PORT) { [int]$env:SUNDAY_MTP_PORT } else { 8084 }
$Prompt = if ($env:SUNDAY_MTP_TEST_PROMPT) { $env:SUNDAY_MTP_TEST_PROMPT } else { "Morning, Sunday. Reply in one short sentence." }
$TimeoutSec = if ($env:SUNDAY_MTP_TEST_TIMEOUT) { [int]$env:SUNDAY_MTP_TEST_TIMEOUT } else { 180 }

function Test-ModelsReady {
    param([int]$Port)
    try {
        $code = & curl.exe -s -o NUL -w "%{http_code}" --max-time 3 "http://127.0.0.1:$Port/v1/models"
        return $code -eq "200"
    } catch {
        return $false
    }
}

$Body = @{
    model = "local-model"
    messages = @(
        @{ role = "system"; content = "You are SUNDAY in fast live voice mode. Reply naturally and briefly." },
        @{ role = "user"; content = $Prompt }
    )
    stream = $false
    max_tokens = 48
    temperature = 0.4
} | ConvertTo-Json -Depth 8

$Started = Get-Date
$Deadline = $Started.AddSeconds($TimeoutSec)
while (-not (Test-ModelsReady -Port $Port)) {
    if ((Get-Date) -gt $Deadline) {
        throw "Timeout waiting for MTP server on port $Port"
    }
    Write-Host "." -NoNewline
    Start-Sleep -Seconds 2
}
Write-Host ""

while ($true) {
    $TempFile = Join-Path $env:TEMP "sunday_mtp_test_$(Get-Random).json"
    try {
        $Body | Set-Content -LiteralPath $TempFile -Encoding UTF8
        $Raw = & curl.exe `
            -sS `
            --max-time 90 `
            -H "Content-Type: application/json" `
            -H "Connection: close" `
            --data-binary "@$TempFile" `
            "http://127.0.0.1:$Port/v1/chat/completions"
        if ($LASTEXITCODE -ne 0) {
            throw "curl failed with exit code $LASTEXITCODE"
        }
        if ($Raw -match '"error"') {
            throw $Raw
        }
        $Response = $Raw | ConvertFrom-Json
        break
    } catch {
        $Message = $_.Exception.Message
        if ((Get-Date) -gt $Deadline -or $Message -notmatch "503|Loading model|unavailable") {
            throw
        }
        Write-Host "[MTP] Model still loading, retrying..." -ForegroundColor Yellow
        Start-Sleep -Seconds 3
    } finally {
        Remove-Item -LiteralPath $TempFile -Force -ErrorAction SilentlyContinue
    }
}
$ElapsedMs = [int]((Get-Date) - $Started).TotalMilliseconds

Write-Host "[MTP] $ElapsedMs ms" -ForegroundColor Green
$Response.choices[0].message.content
