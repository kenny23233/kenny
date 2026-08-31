<#
.SYNOPSIS
    video-toolbox E2E smoke test (release binary launch validation)
.DESCRIPTION
    Last gate before Phase 5 ship. Does NOT actually install the MSI / NSIS,
    but verifies artifacts exist, launches the release binary, waits 5 seconds,
    confirms the process is still alive, then kills it.
.NOTES
    Run:  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/smoke-test-e2e.ps1

    ExitCode: 0 = PASS, non-zero = FAIL
    Output:  "OK" / "FAIL: <reason>" on stdout
#>

$ErrorActionPreference = "Stop"
$ProgressPreference   = "SilentlyContinue"

# Resolve project root from script location
$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
Set-Location $ProjectRoot

$ReleaseExe  = Join-Path $ProjectRoot "src-tauri\target\release\video-toolbox.exe"
$MsiBundle   = Join-Path $ProjectRoot "src-tauri\target\release\bundle\msi"
$NsisBundle  = Join-Path $ProjectRoot "src-tauri\target\release\bundle\nsis"
$ProcessName = "video-toolbox"
$WaitSeconds = 5

function Write-Result {
    param([string]$Status, [string]$Message = "")
    if ($Message) {
        Write-Host ("{0}: {1}" -f $Status, $Message)
    } else {
        Write-Host $Status
    }
}

function Test-BundleExists {
    param([string]$BundleDir, [string]$Pattern)
    if (-not (Test-Path $BundleDir)) {
        return $null
    }
    $files = Get-ChildItem -Path $BundleDir -Filter $Pattern -ErrorAction SilentlyContinue
    if ($files -and $files.Count -gt 0) {
        return $files[0].FullName
    }
    return $null
}

# ---------- main flow ----------

Write-Host "=== video-toolbox E2E smoke test ==="
Write-Host ("ProjectRoot : {0}" -f $ProjectRoot)
Write-Host ("ReleaseExe  : {0}" -f $ReleaseExe)
Write-Host ""

# 1) release binary
if (-not (Test-Path $ReleaseExe)) {
    Write-Result "FAIL" ("release binary not found: {0} (run `cargo tauri build` first)" -f $ReleaseExe)
    exit 1
}
Write-Host ("[OK] release binary exists: {0}" -f $ReleaseExe)

# 2) bundle (either MSI or NSIS is fine)
$msi  = Test-BundleExists $MsiBundle  "*.msi"
$nsis = Test-BundleExists $NsisBundle "*.exe"

if ($msi) {
    Write-Host ("[OK] MSI  artifact: {0}" -f $msi)
} else {
    Write-Host ("[WARN] MSI  artifact missing ({0}\*.msi)" -f $MsiBundle)
}
if ($nsis) {
    Write-Host ("[OK] NSIS artifact: {0}" -f $nsis)
} else {
    Write-Host ("[WARN] NSIS artifact missing ({0}\*.exe)" -f $NsisBundle)
}
if (-not $msi -and -not $nsis) {
    Write-Result "FAIL" "Neither MSI nor NSIS artifact found; build incomplete"
    exit 2
}

# 3) launch
Write-Host ""
Write-Host ("Launching release binary, waiting {0} seconds..." -f $WaitSeconds)
$proc = $null
try {
    $proc = Start-Process -FilePath $ReleaseExe `
                          -PassThru `
                          -WindowStyle Normal `
                          -RedirectStandardError "smoke_stderr.log" `
                          -RedirectStandardOutput "smoke_stdout.log"
} catch {
    Write-Result "FAIL" ("Failed to start binary: {0}" -f $_.Exception.Message)
    exit 3
}

if ($null -eq $proc -or $proc.HasExited) {
    Write-Result "FAIL" ("Binary exited immediately (ExitCode={0})" -f $proc.ExitCode)
    if (Test-Path "smoke_stderr.log") {
        Write-Host "--- stderr ---"
        Get-Content "smoke_stderr.log" -Tail 20
    }
    exit 4
}

Write-Host ("PID = {0}" -f $proc.Id)

# 4) wait and poll
$waited = 0
$tickMs = 500
while ($waited -lt ($WaitSeconds * 1000)) {
    Start-Sleep -Milliseconds $tickMs
    $waited += $tickMs
    if ($proc.HasExited) {
        Write-Result "FAIL" ("Process exited in {0}ms (ExitCode={1})" -f $waited, $proc.ExitCode)
        if (Test-Path "smoke_stderr.log") {
            Write-Host "--- stderr ---"
            Get-Content "smoke_stderr.log" -Tail 30
        }
        exit 5
    }
}

Write-Host ("[OK] Process survived {0}s" -f $WaitSeconds)

# 5) kill
try {
    Stop-Process -Id $proc.Id -Force -ErrorAction Stop
    Write-Host ("[OK] Stop-Process PID={0}" -f $proc.Id)
} catch {
    Write-Host ("[WARN] Stop-Process failed (may already be closed): {0}" -f $_.Exception.Message)
}

# cleanup logs
Remove-Item "smoke_stdout.log" -ErrorAction SilentlyContinue
Remove-Item "smoke_stderr.log" -ErrorAction SilentlyContinue

Write-Host ""
Write-Result "OK" "Smoke test passed"
exit 0
