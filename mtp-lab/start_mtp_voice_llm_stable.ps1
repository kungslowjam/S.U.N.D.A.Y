# Starts Atomic Gemma 4 MTP in a conservative profile for crash isolation.

$env:SUNDAY_MTP_PROFILE = "stable"
$env:SUNDAY_MTP_CACHE_TYPE = if ($env:SUNDAY_MTP_CACHE_TYPE) { $env:SUNDAY_MTP_CACHE_TYPE } else { "f16" }
$env:SUNDAY_MTP_DRAFT_BLOCK_SIZE = if ($env:SUNDAY_MTP_DRAFT_BLOCK_SIZE) { $env:SUNDAY_MTP_DRAFT_BLOCK_SIZE } else { "2" }
$env:SUNDAY_MTP_DRAFT_MAX = if ($env:SUNDAY_MTP_DRAFT_MAX) { $env:SUNDAY_MTP_DRAFT_MAX } else { "4" }
$env:SUNDAY_MTP_DRAFT_MIN = if ($env:SUNDAY_MTP_DRAFT_MIN) { $env:SUNDAY_MTP_DRAFT_MIN } else { "0" }
$env:SUNDAY_MTP_CONTEXT_SIZE = if ($env:SUNDAY_MTP_CONTEXT_SIZE) { $env:SUNDAY_MTP_CONTEXT_SIZE } else { "1024" }

& (Join-Path $PSScriptRoot "start_mtp_voice_llm.ps1")
