# smoke-test.ps1 - 检查 release 构建产物是否齐全
#
# 用法 (在项目根目录):
#   powershell -ExecutionPolicy Bypass -File scripts\smoke-test.ps1
#
# 只做静态检查, 不实际跑安装 (用户来装)
# 检查项:
#   - target/release/bundle/msi/*.msi  (Windows Installer)
#   - target/release/bundle/nsis/*-setup.exe  (NSIS 安装器)
#   - target/release/video-toolbox.exe  (主可执行文件)

$ErrorActionPreference = 'Stop'

$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path (Join-Path $ScriptDir '..')
$SrcTauri    = Join-Path $ProjectRoot 'src-tauri'
$BundleRoot  = Join-Path $SrcTauri 'target/release/bundle'
$ExePath     = Join-Path $SrcTauri 'target/release/video-toolbox.exe'

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  Video Toolbox - Smoke Test" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""

$ok = $true

# --- 主可执行文件 ---
Write-Host "[1/3] Main executable:" -ForegroundColor Yellow
if (Test-Path -Path $ExePath -PathType Leaf) {
    $size = [math]::Round((Get-Item $ExePath).Length / 1MB, 2)
    Write-Host "  OK   $ExePath ($size MB)" -ForegroundColor Green
} else {
    Write-Host "  FAIL $ExePath 不存在 (请先跑 build-release)" -ForegroundColor Red
    $ok = $false
}
Write-Host ""

# --- MSI ---
Write-Host "[2/3] MSI (Windows Installer):" -ForegroundColor Yellow
$msiDir = Join-Path $BundleRoot 'msi'
$msiFiles = @()
if (Test-Path -Path $msiDir -PathType Container) {
    $msiFiles = Get-ChildItem -Path $msiDir -Filter '*.msi' -ErrorAction SilentlyContinue
}
if ($msiFiles.Count -gt 0) {
    foreach ($f in $msiFiles) {
        $size = [math]::Round($f.Length / 1MB, 1)
        Write-Host "  OK   $($f.FullName) ($size MB)" -ForegroundColor Green
    }
} else {
    Write-Host "  FAIL 没有找到 MSI 产物 (目录: $msiDir)" -ForegroundColor Red
    $ok = $false
}
Write-Host ""

# --- NSIS ---
Write-Host "[3/3] NSIS (Setup EXE):" -ForegroundColor Yellow
$nsisDir = Join-Path $BundleRoot 'nsis'
$nsisFiles = @()
if (Test-Path -Path $nsisDir -PathType Container) {
    $nsisFiles = Get-ChildItem -Path $nsisDir -Filter '*.exe' -ErrorAction SilentlyContinue
}
if ($nsisFiles.Count -gt 0) {
    foreach ($f in $nsisFiles) {
        $size = [math]::Round($f.Length / 1MB, 1)
        Write-Host "  OK   $($f.FullName) ($size MB)" -ForegroundColor Green
    }
} else {
    Write-Host "  FAIL 没有找到 NSIS 产物 (目录: $nsisDir)" -ForegroundColor Red
    $ok = $false
}
Write-Host ""

# --- 总结 ---
Write-Host "============================================================" -ForegroundColor $(if ($ok) { "Green" } else { "Red" })
if ($ok) {
    Write-Host "  Smoke test PASSED. 上面所有产物都存在." -ForegroundColor Green
    Write-Host "  接下来: 拷贝 MSI/NSIS 给用户安装, 或者本地双击体验" -ForegroundColor Green
} else {
    Write-Host "  Smoke test FAILED. 见上方 FAIL 行." -ForegroundColor Red
    Write-Host "  提示: 先跑 scripts/build-release.ps1 一次" -ForegroundColor Yellow
}
Write-Host "============================================================" -ForegroundColor $(if ($ok) { "Green" } else { "Red" })
Write-Host ""

if (-not $ok) { exit 1 }
exit 0
