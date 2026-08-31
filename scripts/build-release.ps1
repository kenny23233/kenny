# build-release.ps1 - 一键打包 Video Toolbox release
#
# 用法 (在项目根目录 E:\projects\video-toolbox):
#   powershell -ExecutionPolicy Bypass -File scripts\build-release.ps1
#   或:  npm run build:release
#
# 步骤:
#   1. 预检: yt-dlp.exe / ffmpeg.exe / 图标齐全
#   2. npm install (如果没装)
#   3. npm run tauri build (产出 MSI + NSIS)
#   4. 打印 release 产物路径
#
# 退出码: 0 = 成功, 1 = 预检失败, 2 = 构建失败

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

# --- 路径 ---
$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path (Join-Path $ScriptDir '..')
$SrcTauri    = Join-Path $ProjectRoot 'src-tauri'
$BinDir      = Join-Path $SrcTauri 'bin'
$IconsDir    = Join-Path $SrcTauri 'icons'

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  Video Toolbox - Release Build" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "Project root : $ProjectRoot"
Write-Host "src-tauri    : $SrcTauri"
Write-Host ""

# --- 0. PATH 自愈: cargo/rustc 不在 PATH 时, 从 $env:USERPROFILE\.cargo\bin 找 ---
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if ((Get-Command cargo.exe -ErrorAction SilentlyContinue) -eq $null -and (Test-Path (Join-Path $cargoBin 'cargo.exe'))) {
    Write-Host "INFO: 临时把 $cargoBin 加到 PATH (cargo 不在 PATH 时)" -ForegroundColor DarkYellow
    $env:Path = "$cargoBin;$env:Path"
}

# --- 1. 预检 ---
Write-Host "[1/4] Pre-flight checks ..." -ForegroundColor Yellow

$missing = @()
foreach ($f in @('yt-dlp.exe', 'ffmpeg.exe')) {
    $p = Join-Path $BinDir $f
    if (-not (Test-Path -Path $p -PathType Leaf)) {
        $missing += $p
    } else {
        $size = (Get-Item $p).Length
        $sizeMB = [math]::Round($size / 1MB, 1)
        Write-Host "  OK  $f ($sizeMB MB)"
    }
}
foreach ($f in @('32x32.png', '128x128.png', '128x128@2x.png', 'icon.ico', 'icon.png')) {
    $p = Join-Path $IconsDir $f
    if (-not (Test-Path -Path $p -PathType Leaf)) {
        $missing += $p
    } else {
        Write-Host "  OK  icons/$f"
    }
}
if ($missing.Count -gt 0) {
    Write-Host ""
    Write-Host "ERROR: 以下文件缺失, 无法构建 release 包:" -ForegroundColor Red
    $missing | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    Write-Host ""
    Write-Host "yt-dlp.exe 放到 src-tauri/bin/yt-dlp.exe" -ForegroundColor Yellow
    Write-Host "ffmpeg.exe 放到 src-tauri/bin/ffmpeg.exe" -ForegroundColor Yellow
    Write-Host "图标放到 src-tauri/icons/  (可使用 scripts 里的生成脚本)" -ForegroundColor Yellow
    exit 1
}
Write-Host "  All pre-flight checks passed." -ForegroundColor Green
Write-Host ""

# --- 2. npm install (按需) ---
Write-Host "[2/4] npm install ..." -ForegroundColor Yellow
$nodeModules = Join-Path $ProjectRoot 'node_modules'
if (-not (Test-Path -Path $nodeModules -PathType Container)) {
    Push-Location $ProjectRoot
    try {
        npm install
        if ($LASTEXITCODE -ne 0) { throw "npm install 失败 (exit $LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
} else {
    Write-Host "  node_modules 已存在, 跳过 install"
}
Write-Host ""

# --- 3. tauri build ---
Write-Host "[3/4] npm run tauri build ..." -ForegroundColor Yellow
Write-Host "  (首次构建可能 5-10 分钟, LTO + opt-level=s + strip)" -ForegroundColor Gray
Push-Location $ProjectRoot
try {
    npm run tauri build 2>&1 | Tee-Object -FilePath (Join-Path $SrcTauri '_tauri_build.log')
    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Host "ERROR: tauri build 失败 (exit $LASTEXITCODE)" -ForegroundColor Red
        Write-Host "日志已写入: $SrcTauri\_tauri_build.log" -ForegroundColor Yellow
        exit 2
    }
} finally {
    Pop-Location
}
Write-Host ""

# --- 4. 报告产物 ---
Write-Host "[4/4] Release artifacts:" -ForegroundColor Yellow
$bundleRoot = Join-Path $SrcTauri 'target/release/bundle'
if (-not (Test-Path -Path $bundleRoot -PathType Container)) {
    Write-Host "  WARN: bundle 目录未生成: $bundleRoot" -ForegroundColor Yellow
    exit 2
}

$artifacts = @()
Get-ChildItem -Path $bundleRoot -Recurse -File -Include '*.msi', '*.exe' |
    Where-Object { $_.DirectoryName -like '*\msi\*' -or $_.DirectoryName -like '*\nsis\*' } |
    ForEach-Object {
        $size = [math]::Round($_.Length / 1MB, 1)
        $rel = $_.FullName.Substring($ProjectRoot.Path.Length + 1)
        $artifacts += [PSCustomObject]@{
            Type = ($_.DirectoryName -split '\\')[-1]
            Path = $rel
            Size = "$size MB"
        }
    }

if ($artifacts.Count -eq 0) {
    Write-Host "  WARN: 没有找到 MSI/NSIS 产物" -ForegroundColor Yellow
    Write-Host "  看看 $bundleRoot 下都有什么:" -ForegroundColor Yellow
    Get-ChildItem -Path $bundleRoot -Recurse | Select-Object FullName | Format-Table -AutoSize
} else {
    $artifacts |
        Group-Object Type |
        ForEach-Object {
            Write-Host ""
            Write-Host "  $($_.Name):" -ForegroundColor Cyan
            $_.Group | ForEach-Object {
                Write-Host "    $($_.Path)  [$($_.Size)]" -ForegroundColor White
            }
        }
}

Write-Host ""
Write-Host "============================================================" -ForegroundColor Green
Write-Host "  Build complete." -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Green
Write-Host ""
