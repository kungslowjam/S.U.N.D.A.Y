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

function Patch-LanguageOption {
    $OmniHeader = Join-Path $RepoDir "tools\omni\omni.h"
    if (Test-Path $OmniHeader) {
        $HeaderText = Get-Content -LiteralPath $OmniHeader -Raw
        if ($HeaderText -notmatch "void omni_set_language\(struct omni_context \* ctx_omni") {
            $HeaderNeedle = "bool stop_speek(struct omni_context * ctx_omni);"
            $HeaderReplacement = "void omni_set_language(struct omni_context * ctx_omni, const std::string & lang);`r`n`r`n$HeaderNeedle"
            if (-not $HeaderText.Contains($HeaderNeedle)) {
                throw "Could not patch omni_set_language declaration in $OmniHeader."
            }
            Write-Host "[PATCH] Adding omni_set_language declaration" -ForegroundColor Cyan
            $HeaderText.Replace($HeaderNeedle, $HeaderReplacement) | Set-Content -LiteralPath $OmniHeader -NoNewline
        }
    }

    $OmniCli = Join-Path $RepoDir "tools\omni\omni-cli.cpp"
    if (-not (Test-Path $OmniCli)) {
        return
    }

    $Text = Get-Content -LiteralPath $OmniCli -Raw
    if ($Text -match "--lang <en\|zh>") {
        return
    }

    Write-Host "[PATCH] Adding CLI language option" -ForegroundColor Cyan
    $Replacements = @(
        @{
            Old = "        `"  --omni              Enable omni mode (audio + vision, media_type=2)\n`""
            New = "        `"  --omni              Enable omni mode (audio + vision, media_type=2)\n`"`r`n        `"  --lang <en|zh>      Response language (default: en)\n`""
        },
        @{
            Old = "    bool run_test = false;`r`n    std::string test_audio_prefix;"
            New = "    bool run_test = false;`r`n    std::string language = `"en`";`r`n    std::string test_audio_prefix;"
        },
        @{
            Old = "        else if (arg == `"--omni`") {`r`n            media_type = 2;`r`n        }"
            New = "        else if (arg == `"--omni`") {`r`n            media_type = 2;`r`n        }`r`n        else if (arg == `"--lang`" && i + 1 < argc) {`r`n            language = argv[++i];`r`n            if (language != `"en`" && language != `"zh`") {`r`n                fprintf(stderr, `"Error: --lang must be 'en' or 'zh', got '%s'\n`", language.c_str());`r`n                return 1;`r`n            }`r`n        }"
        },
        @{
            Old = "    printf(`"  GPU layers: %d\n`", n_gpu_layers);"
            New = "    printf(`"  GPU layers: %d\n`", n_gpu_layers);`r`n    printf(`"  Language: %s\n`", language.c_str());"
        },
        @{
            Old = "    ctx_omni->async = true;`r`n    ctx_omni->ref_audio_path = ref_audio_path;  // 设置参考音频路径"
            New = "    ctx_omni->async = true;`r`n    ctx_omni->ref_audio_path = ref_audio_path;  // 设置参考音频路径`r`n    omni_set_language(ctx_omni, language);"
        }
    )

    foreach ($Replacement in $Replacements) {
        if (-not $Text.Contains($Replacement.Old)) {
            throw "Could not patch language option in $OmniCli."
        }
        $Text = $Text.Replace($Replacement.Old, $Replacement.New)
    }

    $Text | Set-Content -LiteralPath $OmniCli -NoNewline
}

Import-VsDevEnvironment

if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) {
    throw "MSVC compiler cl.exe not found. Install Visual Studio Build Tools with C++ build tools, or run this from Developer PowerShell for Visual Studio."
}

if ($Cuda -and -not (Get-Command nvcc.exe -ErrorAction SilentlyContinue)) {
    throw "CUDA build requested, but nvcc.exe was not found. Install NVIDIA CUDA Toolkit or run without -Cuda for CPU build."
}

Patch-WindowsCompatibility
Patch-LanguageOption

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

Write-Host "[BUILD] llama-omni-cli + duplex test" -ForegroundColor Cyan
cmake --build $BuildDir --target llama-omni-cli llama-omni-test-duplex --config Release -j
if ($LASTEXITCODE -ne 0) {
    throw "Build failed."
}

Write-Host "[OK] Build complete." -ForegroundColor Green
