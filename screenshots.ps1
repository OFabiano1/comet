# Telas do Comet para o README: renderiza a UI de verdade (ui::draw) e tira PNG no Chromium.
# Uso: .\screenshots.ps1   ->   screenshots\*.png
# Continue: no PowerShell 5.1 o stderr do cargo vira erro fatal com Stop.
$ErrorActionPreference = "Continue"
Set-Location $PSScriptRoot

$html = Join-Path $PSScriptRoot "target\screenshots"
$out = Join-Path $PSScriptRoot "screenshots"
New-Item -ItemType Directory -Force $html, $out | Out-Null

$env:COMET_SHOT_DIR = $html
cargo test render_screenshots -- --ignored --quiet
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# Fonte do Prism, se o repo dele estiver ao lado; sem ela cai pra Cascadia Mono.
$font = Join-Path $PSScriptRoot "..\prism\fonts\JetBrainsMonoVariable.woff2"
if (Test-Path $font) { Copy-Item $font (Join-Path $html "JetBrainsMono.woff2") -Force }

$chrome = $env:COMET_CHROME
if (-not $chrome) {
    $chrome = Get-ChildItem "$env:LOCALAPPDATA\ms-playwright\chromium_headless_shell-*\*\chrome-headless-shell.exe" -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $chrome) {
    $chrome = (Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe", "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe" -ErrorAction SilentlyContinue |
        Select-Object -First 1).'(default)'
}
if (-not $chrome) { throw "Nenhum Chromium achado. Defina COMET_CHROME com o caminho do chrome.exe." }

# 80x32 celulas de 10x20 px + 24 px de margem; escala 2 pra ficar nitido.
foreach ($page in Get-ChildItem $html -Filter *.html) {
    $png = Join-Path $out ($page.BaseName + ".png")
    $url = ([System.Uri]$page.FullName).AbsoluteUri
    & $chrome --headless --disable-gpu --hide-scrollbars --force-device-scale-factor=2 `
        --window-size=848,688 --screenshot="$png" $url 2>$null | Out-Null
    if (-not (Test-Path $png)) { throw "falhou: $png" }
    Write-Host "OK: screenshots\$($page.BaseName).png"
}
