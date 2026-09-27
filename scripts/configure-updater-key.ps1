param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$keyDirectory = Join-Path $env:USERPROFILE ".tauri"
$keyPath = Join-Path $keyDirectory "watchdog-updater.key"
$publicKeyPath = "$keyPath.pub"
$configPath = Join-Path $projectRoot "src-tauri\tauri.conf.json"

if (Test-Path -LiteralPath $keyPath -PathType Leaf) {
    throw "Refusing to overwrite the existing updater key: $keyPath"
}

New-Item -ItemType Directory -Path $keyDirectory -Force | Out-Null
Write-Host "Enter the updater-key password directly at the signer prompt. It will not be stored by this script."
& npx tauri signer generate -w $keyPath
if ($LASTEXITCODE -ne 0) { throw "Tauri updater key generation failed with exit code $LASTEXITCODE" }
if (-not (Test-Path -LiteralPath $keyPath -PathType Leaf) -or -not (Test-Path -LiteralPath $publicKeyPath -PathType Leaf)) {
    throw "The signer did not create the expected private/public key pair."
}

$publicKey = (Get-Content -LiteralPath $publicKeyPath -Raw).Trim()
if ([string]::IsNullOrWhiteSpace($publicKey)) { throw "The generated updater public key is empty." }
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$config.plugins.updater.pubkey = $publicKey
$configJson = $config | ConvertTo-Json -Depth 100
[System.IO.File]::WriteAllText($configPath, $configJson + [Environment]::NewLine, [System.Text.UTF8Encoding]::new($false))

Write-Host "Updater public key configured in src-tauri/tauri.conf.json."
Write-Host "Private key retained outside the repository: $keyPath"
Write-Host "Create an encrypted backup before publishing an updater-enabled release."
