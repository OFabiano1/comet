# Comet — fetch external binaries into vendor/.
# Downloads the newest yt-dlp.exe and a Windows ffmpeg essentials build.
# vendor/ is git-ignored; rerun this script to update both.
# Usage (PowerShell):  powershell -ExecutionPolicy Bypass -File scripts\fetch-binaries.ps1

$ErrorActionPreference = 'Stop'
$vendor = Join-Path (Join-Path $PSScriptRoot '..') 'vendor'
New-Item -ItemType Directory -Force -Path $vendor | Out-Null

# --- yt-dlp.exe ---
$ytdlpUrl = 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe'
$ytdlpTarget = Join-Path $vendor 'yt-dlp.exe'
Write-Host "Baixando yt-dlp..."
Invoke-WebRequest -Uri $ytdlpUrl -OutFile $ytdlpTarget -UseBasicParsing
Write-Host "  -> $ytdlpTarget"

# --- ffmpeg (essentials build) ---
$ffmpegUrl = 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip'
$zip = Join-Path $env:TEMP 'comet-ffmpeg.zip'
$extractDir = Join-Path $env:TEMP 'comet-ffmpeg'
Write-Host "Baixando ffmpeg..."
Invoke-WebRequest -Uri $ffmpegUrl -OutFile $zip -UseBasicParsing

if (Test-Path $extractDir) { Remove-Item $extractDir -Recurse -Force }
Expand-Archive -Path $zip -DestinationPath $extractDir

$ffmpegBin = Get-ChildItem -Path $extractDir -Filter 'ffmpeg.exe' -Recurse | Select-Object -First 1
$ffprobeBin = Get-ChildItem -Path $extractDir -Filter 'ffprobe.exe' -Recurse | Select-Object -First 1
if (-not $ffmpegBin -or -not $ffprobeBin) { throw 'ffmpeg/ffprobe não achados dentro do zip.' }

Copy-Item $ffmpegBin.FullName (Join-Path $vendor 'ffmpeg.exe') -Force
Copy-Item $ffprobeBin.FullName (Join-Path $vendor 'ffprobe.exe') -Force
Write-Host "  -> $(Join-Path $vendor 'ffmpeg.exe')"

Remove-Item $zip -Force -ErrorAction SilentlyContinue
Remove-Item $extractDir -Recurse -Force -ErrorAction SilentlyContinue
Write-Host "Comet: binários prontos em vendor/."
