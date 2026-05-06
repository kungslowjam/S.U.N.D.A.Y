param(
    [int] $Port = 8099
)

# Lightweight local test UI for omni-lab. Serves only on localhost.

$ErrorActionPreference = "Stop"

$LabRoot = $PSScriptRoot
$RepoDir = Join-Path $LabRoot "llama.cpp-omni"
$OutputRoot = Join-Path $RepoDir "tools\omni\output"
$LogDir = Join-Path $LabRoot "build-logs"
$StateFile = Join-Path $LogDir "omni-test-ui-state.json"

New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

function Send-Text {
    param($Response, [string] $Text, [string] $ContentType = "text/plain; charset=utf-8")
    $Bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $Response.ContentType = $ContentType
    $Response.ContentLength64 = $Bytes.Length
    $Response.OutputStream.Write($Bytes, 0, $Bytes.Length)
}

function Send-Json {
    param($Response, $Value)
    Send-Text $Response ($Value | ConvertTo-Json -Depth 8) "application/json; charset=utf-8"
}

function Get-LatestRound {
    if (-not (Test-Path $OutputRoot)) {
        return $null
    }

    Get-ChildItem -LiteralPath $OutputRoot -Directory -Filter "round_*" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
}

function Read-State {
    if (Test-Path $StateFile) {
        try {
            return Get-Content -LiteralPath $StateFile -Raw | ConvertFrom-Json
        } catch {
            return $null
        }
    }
    return $null
}

function Write-State {
    param([string] $Mode, [int] $ProcessId, [string] $LogPath)
    @{
        mode = $Mode
        processId = $ProcessId
        logPath = $LogPath
        startedAt = (Get-Date).ToString("o")
    } | ConvertTo-Json | Set-Content -LiteralPath $StateFile
}

function Get-RunStatus {
    $State = Read-State
    if (-not $State) {
        return @{ running = $false; mode = $null; processId = $null; logTail = ""; logPath = $null }
    }

    $Running = $false
    try {
        $null = Get-Process -Id ([int] $State.processId) -ErrorAction Stop
        $Running = $true
    } catch {
        $Running = $false
    }

    $Tail = ""
    if ($State.logPath -and (Test-Path $State.logPath)) {
        $Tail = (Get-Content -LiteralPath $State.logPath -Tail 80 -ErrorAction SilentlyContinue) -join "`n"
    }

    @{
        running = $Running
        mode = $State.mode
        processId = $State.processId
        startedAt = $State.startedAt
        logPath = $State.logPath
        logTail = $Tail
    }
}

function Start-TestRun {
    param([string] $Mode)

    $Status = Get-RunStatus
    if ($Status.running) {
        return @{ ok = $false; message = "A test is already running."; status = $Status }
    }

    $Script = if ($Mode -eq "full") { "run_omni_full.ps1" } else { "run_omni_text_only.ps1" }
    $ScriptPath = Join-Path $LabRoot $Script
    if (-not (Test-Path $ScriptPath)) {
        return @{ ok = $false; message = "Missing script: $Script" }
    }

    $LogPath = Join-Path $LogDir ("omni-ui-{0}-{1}.log" -f $Mode, (Get-Date -Format "yyyyMMdd-HHmmss"))
    $Args = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $ScriptPath
    )
    $Process = Start-Process -FilePath "powershell.exe" -ArgumentList $Args -RedirectStandardOutput $LogPath -RedirectStandardError $LogPath -PassThru -WindowStyle Hidden
    Write-State $Mode $Process.Id $LogPath

    @{ ok = $true; message = "Started $Mode test."; processId = $Process.Id; logPath = $LogPath }
}

function Get-AudioFiles {
    $Round = Get-LatestRound
    if (-not $Round) {
        return @{ round = $null; files = @() }
    }

    $WavDir = Join-Path $Round.FullName "tts_wav"
    if (-not (Test-Path $WavDir)) {
        return @{ round = $Round.Name; files = @() }
    }

    $Files = Get-ChildItem -LiteralPath $WavDir -File -Filter "*.wav" |
        Sort-Object Name |
        ForEach-Object {
            @{
                name = $_.Name
                size = $_.Length
                modified = $_.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss")
                url = "/audio/$($Round.Name)/$($_.Name)"
            }
        }

    @{ round = $Round.Name; files = @($Files) }
}

$Html = @'
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>SUNDAY Omni Lab</title>
  <style>
    :root { color-scheme: dark; font-family: Segoe UI, Arial, sans-serif; background: #101214; color: #eef1f4; }
    body { margin: 0; }
    main { max-width: 1120px; margin: 0 auto; padding: 24px; }
    header { display: flex; justify-content: space-between; align-items: center; gap: 16px; margin-bottom: 20px; }
    h1 { margin: 0; font-size: 24px; font-weight: 650; }
    .status { font-size: 14px; color: #a9b1bb; }
    .toolbar { display: flex; flex-wrap: wrap; gap: 10px; margin-bottom: 18px; }
    button { border: 1px solid #39414b; background: #1b2229; color: #f4f7fa; padding: 9px 12px; border-radius: 6px; cursor: pointer; }
    button:hover { background: #26313b; }
    button.primary { background: #245c45; border-color: #31825f; }
    button.danger { background: #5b2a2a; border-color: #8a3b3b; }
    section { border-top: 1px solid #2c333b; padding-top: 16px; margin-top: 16px; }
    .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 18px; }
    .panel { background: #151a1f; border: 1px solid #2a323a; border-radius: 8px; padding: 14px; min-width: 0; }
    .panel h2 { font-size: 15px; margin: 0 0 12px; }
    pre { white-space: pre-wrap; overflow: auto; max-height: 560px; margin: 0; color: #d4dae1; font-size: 12px; line-height: 1.45; }
    .file { display: grid; grid-template-columns: minmax(120px, 1fr) auto; gap: 10px; align-items: center; padding: 10px 0; border-top: 1px solid #252d35; }
    .file:first-child { border-top: 0; }
    audio { width: 320px; max-width: 100%; }
    .muted { color: #9aa4ae; }
    @media (max-width: 820px) { .grid { grid-template-columns: 1fr; } header { align-items: flex-start; flex-direction: column; } }
  </style>
</head>
<body>
  <main>
    <header>
      <h1>SUNDAY Omni Lab</h1>
      <div class="status" id="status">Checking...</div>
    </header>
    <div class="toolbar">
      <button class="primary" onclick="runTest('text')">Run Text-Only Test</button>
      <button onclick="runTest('full')">Run Full TTS Test</button>
      <button onclick="refreshAll()">Refresh</button>
      <button onclick="openOutput()">Open Output Folder</button>
    </div>
    <div class="grid">
      <section class="panel">
        <h2>Generated Audio <span class="muted" id="round"></span></h2>
        <div id="files" class="muted">No files loaded.</div>
      </section>
      <section class="panel">
        <h2>Run Log</h2>
        <pre id="log"></pre>
      </section>
    </div>
  </main>
  <script>
    async function api(path, options) {
      const res = await fetch(path, options);
      if (!res.ok) throw new Error(await res.text());
      return res.json();
    }

    async function runTest(mode) {
      const result = await api('/api/run?mode=' + encodeURIComponent(mode), { method: 'POST' });
      document.getElementById('status').textContent = result.message || 'Started';
      refreshAll();
    }

    async function openOutput() {
      const result = await api('/api/open-output', { method: 'POST' });
      document.getElementById('status').textContent = result.message;
    }

    async function refreshStatus() {
      const status = await api('/api/status');
      document.getElementById('status').textContent = status.running
        ? `Running ${status.mode} test (PID ${status.processId})`
        : 'Idle';
      document.getElementById('log').textContent = status.logTail || '';
    }

    async function refreshFiles() {
      const data = await api('/api/files');
      document.getElementById('round').textContent = data.round ? `(${data.round})` : '';
      const host = document.getElementById('files');
      if (!data.files.length) {
        host.textContent = 'No wav files found yet.';
        host.className = 'muted';
        return;
      }
      host.className = '';
      host.innerHTML = data.files.map(file => `
        <div class="file">
          <div>
            <div>${file.name}</div>
            <div class="muted">${Math.round(file.size / 1024)} KB · ${file.modified}</div>
          </div>
          <audio controls src="${file.url}"></audio>
        </div>
      `).join('');
    }

    async function refreshAll() {
      await Promise.all([refreshStatus(), refreshFiles()]);
    }

    refreshAll();
    setInterval(refreshAll, 2500);
  </script>
</body>
</html>
'@

$Prefix = "http://127.0.0.1:$Port/"
$Listener = [System.Net.HttpListener]::new()
$Listener.Prefixes.Add($Prefix)
$Listener.Start()

Write-Host "[OMNI UI] $Prefix" -ForegroundColor Cyan
Start-Process $Prefix | Out-Null

try {
    while ($Listener.IsListening) {
        $Context = $Listener.GetContext()
        $Request = $Context.Request
        $Response = $Context.Response

        try {
            $Path = $Request.Url.AbsolutePath
            if ($Path -eq "/") {
                Send-Text $Response $Html "text/html; charset=utf-8"
            } elseif ($Path -eq "/api/status") {
                Send-Json $Response (Get-RunStatus)
            } elseif ($Path -eq "/api/files") {
                Send-Json $Response (Get-AudioFiles)
            } elseif ($Path -eq "/api/run" -and $Request.HttpMethod -eq "POST") {
                $Mode = $Request.QueryString["mode"]
                if ($Mode -ne "full") { $Mode = "text" }
                Send-Json $Response (Start-TestRun $Mode)
            } elseif ($Path -eq "/api/open-output" -and $Request.HttpMethod -eq "POST") {
                $Round = Get-LatestRound
                $Target = if ($Round -and (Test-Path (Join-Path $Round.FullName "tts_wav"))) {
                    Join-Path $Round.FullName "tts_wav"
                } elseif ($Round) {
                    $Round.FullName
                } else {
                    $OutputRoot
                }
                if (Test-Path $Target) {
                    explorer $Target
                    Send-Json $Response @{ ok = $true; message = "Opened $Target" }
                } else {
                    Send-Json $Response @{ ok = $false; message = "No output folder found." }
                }
            } elseif ($Path.StartsWith("/audio/")) {
                $Parts = $Path.TrimStart("/").Split("/")
                if ($Parts.Length -ne 3) {
                    $Response.StatusCode = 404
                    Send-Text $Response "Not found"
                } else {
                    $RoundName = [System.Uri]::UnescapeDataString($Parts[1])
                    $FileName = [System.Uri]::UnescapeDataString($Parts[2])
                    $AudioPath = Join-Path $OutputRoot (Join-Path $RoundName (Join-Path "tts_wav" $FileName))
                    $Resolved = Resolve-Path -LiteralPath $AudioPath -ErrorAction SilentlyContinue
                    if (-not $Resolved -or -not ($Resolved.Path.StartsWith((Resolve-Path $OutputRoot).Path))) {
                        $Response.StatusCode = 404
                        Send-Text $Response "Not found"
                    } else {
                        $Bytes = [System.IO.File]::ReadAllBytes($Resolved.Path)
                        $Response.ContentType = "audio/wav"
                        $Response.ContentLength64 = $Bytes.Length
                        $Response.OutputStream.Write($Bytes, 0, $Bytes.Length)
                    }
                }
            } else {
                $Response.StatusCode = 404
                Send-Text $Response "Not found"
            }
        } catch {
            $Response.StatusCode = 500
            Send-Text $Response $_.Exception.Message
        } finally {
            $Response.OutputStream.Close()
        }
    }
} finally {
    $Listener.Stop()
}
