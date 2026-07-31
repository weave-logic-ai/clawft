#Requires -Version 5.1
<#
.SYNOPSIS
  Install the Clawft / Weft wake-word daemon as a Windows Task Scheduler task.

.DESCRIPTION
  WEFT-220 final Windows route: schtasks ONLOGON for the current user
  (LIMITED rights). Not a Windows Service (SCM) — the wake daemon needs
  the interactive session microphone.

  Preferred one-liner from an installed binary:
    weft voice install-service
    weft voice install-service --manager schtasks

  This script is the optional companion for ops / imaging without invoking
  the CLI, or when reinstalling from a checkout.

.PARAMETER WeftPath
  Absolute path to weft.exe. Default: resolve `weft` from PATH.

.PARAMETER TaskName
  Task Scheduler task name. Default: ClawftWake (matches the CLI).

.PARAMETER Uninstall
  Delete the task instead of creating it.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/install-clawft-wake-schtasks.ps1

.EXAMPLE
  .\scripts\install-clawft-wake-schtasks.ps1 -WeftPath 'C:\Program Files\weft\weft.exe'

.EXAMPLE
  .\scripts\install-clawft-wake-schtasks.ps1 -Uninstall
#>
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string]$WeftPath,
    [string]$TaskName = 'ClawftWake',
    [switch]$Uninstall
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Resolve-WeftExe {
    param([string]$Explicit)
    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit)) {
            throw "WeftPath not found: $Explicit"
        }
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    $cmd = Get-Command weft -ErrorAction SilentlyContinue
    if ($cmd -and $cmd.Source) {
        return $cmd.Source
    }
    throw @"
Could not find 'weft' on PATH. Install WeftOS first, or pass -WeftPath.

  curl -fsSL https://weftos.weavelogic.ai/install.sh | sh
  # or build: scripts/build.sh native && use target/release/weft.exe
"@
}

if ($Uninstall) {
    if ($PSCmdlet.ShouldProcess($TaskName, 'schtasks /Delete')) {
        & schtasks.exe /Delete /TN $TaskName /F
        if ($LASTEXITCODE -ne 0) {
            throw "schtasks /Delete failed with exit $LASTEXITCODE"
        }
        Write-Host "Deleted task '$TaskName'."
    }
    return
}

$exe = Resolve-WeftExe -Explicit $WeftPath
# Quote path when it contains spaces so /TR parses program + args correctly.
if ($exe -match '\s') {
    $tr = "`"$exe`" voice wake"
} else {
    $tr = "$exe voice wake"
}

Write-Host "Installing Task Scheduler task: $TaskName"
Write-Host "  Program:   $exe"
Write-Host "  Arguments: voice wake"
Write-Host "  Trigger:   ONLOGON (current user, LIMITED)"
Write-Host ""

if ($PSCmdlet.ShouldProcess($TaskName, 'schtasks /Create ONLOGON')) {
    & schtasks.exe /Create /TN $TaskName /TR $tr /SC ONLOGON /RL LIMITED /F
    if ($LASTEXITCODE -ne 0) {
        throw "schtasks /Create failed with exit $LASTEXITCODE"
    }
    Write-Host "Task '$TaskName' created."
    Write-Host ""
    Write-Host "Commands:"
    Write-Host "  schtasks /Run    /TN `"$TaskName`"   # Start now"
    Write-Host "  schtasks /End    /TN `"$TaskName`"   # Stop"
    Write-Host "  schtasks /Query  /TN `"$TaskName`" /V /FO LIST"
    Write-Host "  schtasks /Delete /TN `"$TaskName`" /F"
    Write-Host "  # or: .\scripts\install-clawft-wake-schtasks.ps1 -Uninstall"
}
