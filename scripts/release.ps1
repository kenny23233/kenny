# Release script for local-toolbox
# Usage:
#   1. Create a GitHub Personal Access Token with 'repo' scope
#   2. Set GITHUB_TOKEN env var
#   3. Run: powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -NewVersion "0.1.6" -ReleaseNotes "xxx"
#
# Auto: update version, build MSI, create GitHub Release, upload MSI, update manifest

param(
    [Parameter(Mandatory=$true)][string]$NewVersion,
    [string]$ReleaseNotes = "New release",
    [string]$Repo = "kenny23233/kenny"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $ProjectRoot

# Ensure cargo is in PATH (PowerShell launched from bash may not have it)
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

Write-Host "=== Local Toolbox Release ===" -ForegroundColor Cyan
Write-Host "Version: $NewVersion" -ForegroundColor Green
Write-Host "Repo: $Repo" -ForegroundColor Green
Write-Host ""

# 1. Update versions (write as UTF-8 without BOM to preserve Chinese)
Write-Host "[1/7] Updating versions..." -ForegroundColor Yellow
$tConf = "src-tauri\tauri.conf.json"
$cToml = "src-tauri\Cargo.toml"
$pJson = "package.json"

$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
$utf8 = [System.Text.Encoding]::UTF8

[System.IO.File]::WriteAllText($tConf,
    ([System.IO.File]::ReadAllText($tConf, $utf8) -replace '"version": "[\d.]+"', "`"version`": `"$NewVersion`""),
    $utf8NoBom)
[System.IO.File]::WriteAllText($cToml,
    ([System.IO.File]::ReadAllText($cToml, $utf8) -replace '^version = "[\d.]+"', "version = `"$NewVersion`""),
    $utf8NoBom)
[System.IO.File]::WriteAllText($pJson,
    ([System.IO.File]::ReadAllText($pJson, $utf8) -replace '"version": "[\d.]+"', "`"version`": `"$NewVersion`""),
    $utf8NoBom)
Write-Host "  Done" -ForegroundColor Green

# 2. Build MSI
Write-Host "[2/7] Building MSI (1-2 minutes)..." -ForegroundColor Yellow
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw "Build failed" }
Write-Host "  Build complete" -ForegroundColor Green

# 3. Locate MSI
$msiFile = Get-ChildItem "src-tauri\target\release\bundle\msi\*.msi" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like "*zh-CN*" } | Select-Object -First 1
if (-not $msiFile) { throw "MSI file not found" }
$msiSize = $msiFile.Length
Write-Host "  Found MSI: $($msiFile.Name) ($msiSize bytes)" -ForegroundColor Green

# 4. Check token
if (-not $env:GITHUB_TOKEN) {
    throw "GITHUB_TOKEN env var required (Personal Access Token with 'repo' scope)"
}

$headers = @{
    "Authorization" = "token $env:GITHUB_TOKEN"
    "Accept" = "application/vnd.github+json"
    "User-Agent" = "local-toolbox-release-script"
}

# 5. Create Release
Write-Host "[3/7] Creating GitHub Release v$NewVersion..." -ForegroundColor Yellow
$releaseApi = "https://api.github.com/repos/$Repo/releases"
$body = @{
    tag_name = "v$NewVersion"
    name = "Local Toolbox v$NewVersion"
    body = $ReleaseNotes
    draft = $false
    prerelease = $false
} | ConvertTo-Json

$releaseResp = Invoke-RestMethod -Uri $releaseApi -Method Post -Headers $headers -Body $body -ContentType "application/json"
$uploadUrl = $releaseResp.upload_url -replace "\{\?name,label\}", ""
Write-Host "  Release created (id: $($releaseResp.id))" -ForegroundColor Green

# 6. Upload MSI
Write-Host "[4/7] Uploading MSI to Release..." -ForegroundColor Yellow
$uploadHeaders = $headers.Clone()
$uploadHeaders["Content-Type"] = "application/octet-stream"

$msiBytes = [System.IO.File]::ReadAllBytes($msiFile.FullName)
Invoke-RestMethod -Uri "$uploadUrl?name=$($msiFile.Name)" -Method Post -Headers $uploadHeaders -Body $msiBytes | Out-Null
Write-Host "  Uploaded: $($msiFile.Name)" -ForegroundColor Green

# 7. Update manifest
Write-Host "[5/7] Updating update-manifest.json..." -ForegroundColor Yellow
$manifestPath = "update-manifest.json"
$today = Get-Date -Format "yyyy-MM-dd"
$msiUrl = "https://github.com/$Repo/releases/download/v$NewVersion/$($msiFile.Name)"

$manifest = @{
    version = $NewVersion
    date = $today
    releaseNotes = $ReleaseNotes
    msiPath = $msiUrl
    msiSizeBytes = $msiSize
} | ConvertTo-Json -Depth 10

[System.IO.File]::WriteAllText($manifestPath, $manifest, $utf8NoBom)
Write-Host "  Manifest updated" -ForegroundColor Green

# 8. Copy manifest to app data
Write-Host "[6/7] Copying manifest to app data..." -ForegroundColor Yellow
$appDataDir = Join-Path $env:APPDATA "com.local-toolbox.app"
if (-not (Test-Path $appDataDir)) {
    New-Item -ItemType Directory -Path $appDataDir -Force | Out-Null
}
Copy-Item $manifestPath -Destination (Join-Path $appDataDir "update-manifest.json") -Force
Write-Host "  Copied to $appDataDir\update-manifest.json" -ForegroundColor Green

# 9. Commit and push
Write-Host "[7/7] Committing and pushing..." -ForegroundColor Yellow
git add .
git commit -m "v${NewVersion}: ${ReleaseNotes}" --allow-empty
git push
if ($LASTEXITCODE -ne 0) { Write-Host "  Push failed, may need manual handling" -ForegroundColor Yellow }
else { Write-Host "  Pushed" -ForegroundColor Green }

Write-Host ""
Write-Host "=== Release Complete ===" -ForegroundColor Cyan
Write-Host "Release URL: https://github.com/$Repo/releases/tag/v$NewVersion" -ForegroundColor Green
Write-Host "MSI URL:     $msiUrl" -ForegroundColor Green
Write-Host ""
Write-Host "Other machines will detect the update on next launch." -ForegroundColor Cyan
