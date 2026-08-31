# 发布新版本到 GitHub
# 用法：
#   1. 编辑本脚本顶部的 $NewVersion 和 $ReleaseNotes
#   2. 确保 $env:GITHUB_TOKEN 已设置（Personal Access Token, scope: repo）
#   3. 运行：powershell -ExecutionPolicy Bypass -File scripts/release.ps1
#
# 会自动完成：
#   - 更新 tauri.conf.json + Cargo.toml + package.json 的版本号
#   - build MSI
#   - 创建 GitHub Release + 上传 MSI
#   - 更新 update-manifest.json
#   - 复制 manifest 到 %APPDATA%\com.local-toolbox.app\
#   - commit + push

param(
    [Parameter(Mandatory=$true)][string]$NewVersion,
    [string]$ReleaseNotes = "新版本发布",
    [string]$Repo = "kenny23233/kenny"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot | Split-Path -Parent
Set-Location $ProjectRoot

Write-Host "=== 本地工具箱发布流程 ===" -ForegroundColor Cyan
Write-Host "目标版本: $NewVersion" -ForegroundColor Green
Write-Host "目标仓库: $Repo" -ForegroundColor Green
Write-Host ""

# 1. 更新版本号
Write-Host "[1/7] 更新版本号..." -ForegroundColor Yellow
$tConf = "src-tauri\tauri.conf.json"
$cToml = "src-tauri\Cargo.toml"
$pJson = "package.json"

(Get-Content $tConf) -replace '"version": "[\d.]+"', "`"version`": `"$NewVersion`"" | Set-Content $tConf
(Get-Content $cToml) -replace '^version = "[\d.]+"', "version = `"$NewVersion`"" | Set-Content $cToml
(Get-Content $pJson) -replace '"version": "[\d.]+"', "`"version`": `"$NewVersion`"" | Set-Content $pJson
Write-Host "  ✓ tauri.conf.json, Cargo.toml, package.json" -ForegroundColor Green

# 2. 构建 MSI
Write-Host "[2/7] 构建 MSI (这可能要 1-2 分钟)..." -ForegroundColor Yellow
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw "构建失败" }
Write-Host "  ✓ 构建完成" -ForegroundColor Green

# 3. 定位 MSI
$msiFile = Get-ChildItem "src-tauri\target\release\bundle\msi\*.msi" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -like "*zh-CN*" } | Select-Object -First 1
if (-not $msiFile) { throw "找不到 MSI 文件" }
$msiSize = $msiFile.Length
Write-Host "  ✓ 找到 MSI: $($msiFile.Name) ($msiSize bytes)" -ForegroundColor Green

# 4. 检查 GitHub Token
if (-not $env:GITHUB_TOKEN) {
    throw "需要设置 GITHUB_TOKEN 环境变量（Personal Access Token, scope: repo）"
}

$headers = @{
    "Authorization" = "token $env:GITHUB_TOKEN"
    "Accept" = "application/vnd.github+json"
    "User-Agent" = "local-toolbox-release-script"
}

# 5. 创建 GitHub Release
Write-Host "[3/7] 创建 GitHub Release v$NewVersion..." -ForegroundColor Yellow
$releaseApi = "https://api.github.com/repos/$Repo/releases"
$body = @{
    tag_name = "v$NewVersion"
    name = "本地工具箱 v$NewVersion"
    body = $ReleaseNotes
    draft = $false
    prerelease = $false
} | ConvertTo-Json

$releaseResp = Invoke-RestMethod -Uri $releaseApi -Method Post -Headers $headers -Body $body -ContentType "application/json"
$uploadUrl = $releaseResp.upload_url -replace "\{\?name,label\}", ""
Write-Host "  ✓ Release 创建成功 (id: $($releaseResp.id))" -ForegroundColor Green

# 6. 上传 MSI 到 Release
Write-Host "[4/7] 上传 MSI 到 Release..." -ForegroundColor Yellow
$uploadHeaders = $headers.Clone()
$uploadHeaders["Content-Type"] = "application/octet-stream"

$msiBytes = [System.IO.File]::ReadAllBytes($msiFile.FullName)
Invoke-RestMethod -Uri "$uploadUrl?name=$($msiFile.Name)" -Method Post -Headers $uploadHeaders -Body $msiBytes | Out-Null
Write-Host "  ✓ 上传完成: $($msiFile.Name)" -ForegroundColor Green

# 7. 更新 update-manifest.json
Write-Host "[5/7] 更新 update-manifest.json..." -ForegroundColor Yellow
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

$manifest | Set-Content $manifestPath -Encoding UTF8
Write-Host "  ✓ manifest 已更新" -ForegroundColor Green

# 8. 复制 manifest 到 app data 目录
Write-Host "[6/7] 复制 manifest 到 app data 目录..." -ForegroundColor Yellow
$appDataDir = Join-Path $env:APPDATA "com.local-toolbox.app"
if (-not (Test-Path $appDataDir)) {
    New-Item -ItemType Directory -Path $appDataDir -Force | Out-Null
}
Copy-Item $manifestPath -Destination (Join-Path $appDataDir "update-manifest.json") -Force
Write-Host "  ✓ manifest 已复制到 $appDataDir\update-manifest.json" -ForegroundColor Green

# 9. commit + push
Write-Host "[7/7] 提交并推送代码..." -ForegroundColor Yellow
git add .
git commit -m "v$NewVersion: $ReleaseNotes" --allow-empty
git push
if ($LASTEXITCODE -ne 0) { Write-Host "  ⚠ push 失败，可能需要手动处理" -ForegroundColor Yellow }
else { Write-Host "  ✓ 已 push" -ForegroundColor Green }

Write-Host ""
Write-Host "=== 发布完成 ===" -ForegroundColor Cyan
Write-Host "Release URL: https://github.com/$Repo/releases/tag/v$NewVersion" -ForegroundColor Green
Write-Host "MSI URL:     $msiUrl" -ForegroundColor Green
Write-Host ""
Write-Host "其他电脑下次打开应用时会自动检测到更新，点 '🚀 立即更新并自动重启' 即可完成升级。" -ForegroundColor Cyan
