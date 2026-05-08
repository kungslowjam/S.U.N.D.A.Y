$ErrorActionPreference = "Stop"

$Port = if ($env:SUNDAY_MTP_PORT) { [int]$env:SUNDAY_MTP_PORT } else { 8084 }
$Prompt = if ($env:SUNDAY_MTP_TEST_PROMPT) { $env:SUNDAY_MTP_TEST_PROMPT } else { "User: Morning, Sunday.\nAssistant:" }
$TempFile = Join-Path $env:TEMP "sunday_mtp_completion_$(Get-Random).json"

$Body = @{
    model = "local-model"
    prompt = $Prompt
    stream = $false
    max_tokens = 32
    temperature = 0.4
} | ConvertTo-Json -Depth 5

try {
    $Body | Set-Content -LiteralPath $TempFile -Encoding UTF8
    $Started = Get-Date
    $Raw = & curl.exe `
        -sS `
        --max-time 90 `
        -H "Content-Type: application/json" `
        -H "Connection: close" `
        --data-binary "@$TempFile" `
        "http://127.0.0.1:$Port/v1/completions"
    if ($LASTEXITCODE -ne 0) {
        throw "curl failed with exit code $LASTEXITCODE"
    }
    $ElapsedMs = [int]((Get-Date) - $Started).TotalMilliseconds
    Write-Host "[MTP completion] $ElapsedMs ms" -ForegroundColor Green
    $Raw
} finally {
    Remove-Item -LiteralPath $TempFile -Force -ErrorAction SilentlyContinue
}
