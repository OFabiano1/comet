# Build do Comet Terminal: cargo release + zip do exe.
# Uso: .\build.ps1
$ErrorActionPreference = "Continue"

cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Force dist | Out-Null
Copy-Item target\release\comet.exe dist\comet.exe -Force

$ver = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$zip = "dist\comet-v$ver.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path dist\comet.exe, README.md -DestinationPath $zip
Write-Host "OK: $zip"
