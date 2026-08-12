# 绿色便携版打包脚本（解压即用，对应 doc/07 / doc/10）
# 用法：在仓库根目录执行  powershell -ExecutionPolicy Bypass -File scripts/package-green.ps1
# 前提：已在本机执行过 `npm run tauri build`（package 配置 active:false，仅产出裸 exe）
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$Exe  = Join-Path $Root "src-tauri\target\release\rest-reminder.exe"
$TauriConf = Join-Path $Root "src-tauri\tauri.conf.json"

if (-not (Test-Path $Exe)) {
    Write-Error "找不到 $Exe 。请先在本机执行 `npm run tauri build`。"
    exit 1
}

# 读取版本号（从 tauri.conf.json 的 version 字段），解析失败时回退默认
$ver = "0.1.0"
$m = Select-String -Path $TauriConf -Pattern '"version"\s*:\s*"([^"]+)"' | Select-Object -First 1
if ($m -and $m.Matches.Groups.Count -gt 1) { $ver = $m.Matches.Groups[1].Value }

$product = "休息提醒助手"
$OutDir  = Join-Path $Root "src-tauri\target\release\bundle\portable\$product"
$Zip     = Join-Path $Root "src-tauri\target\release\bundle\portable\${product}_v${ver}_portable.zip"

# 准备干净的发布目录
if (Test-Path $OutDir) { Remove-Item $OutDir -Recurse -Force }
New-Item -ItemType Directory -Path $OutDir | Out-Null

# 复制主程序（Tauri 2 已内嵌前端与图标，单 exe 即可运行，仅需系统 WebView2 Runtime）
# 构建期 exe 可能被 tauri build 占用，做简短重试以避免伪失败
$copied = $false
for ($i = 0; $i -lt 5; $i++) {
    try { Copy-Item $Exe (Join-Path $OutDir "rest-reminder.exe") -Force; $copied = $true; break }
    catch { Start-Sleep -Seconds 1 }
}
if (-not $copied) {
    Write-Error "复制 exe 失败：可能被其它进程（如正在运行的 tauri build）占用，请关闭后重试。"
    exit 1
}

# 使用说明
$readme = @"
$product 绿色便携版 v$ver
================================
· 解压后双击 rest-reminder.exe 即可使用，无需安装、无注册表写入。
· 运行依赖：Windows 10/11 已内置 Microsoft Edge WebView2 Runtime；
  若双击无反应，请到微软官网安装“WebView2 Runtime (Evergreen)”。
· 数据存放：%LOCALAPPDATA%\rest-reminder（按日落盘，可手动删除清空）。
· 关闭窗口 = 最小化到系统托盘（不退出）；右键托盘图标可退出/设置。
· 暂停/恢复按钮在窗口右上角。
· 本软件零网络外联，所有数据仅存于本机。
"@
Set-Content -Path (Join-Path $OutDir "使用说明.txt") -Value $readme -Encoding UTF8

# 打包成 zip
if (Test-Path $Zip) { Remove-Item $Zip -Force }
Compress-Archive -Path (Join-Path $OutDir "*") -DestinationPath $Zip -CompressionLevel Optimal

Write-Host "绿色便携包已生成：`n  $Zip"
Write-Host "内含：rest-reminder.exe + 使用说明.txt"
