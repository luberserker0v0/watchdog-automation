param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-Checked {
    param([Parameter(Mandatory = $true)][scriptblock]$Command, [Parameter(Mandatory = $true)][string]$Description)
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE"
    }
}

function Find-SignTool {
    $command = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $kits = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
    $candidate = Get-ChildItem -LiteralPath $kits -Recurse -Filter signtool.exe -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $candidate) { throw "signtool.exe was not found in PATH or the Windows SDK." }
    return $candidate.FullName
}

function Assert-Signed {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Signed artifact is missing: $Path" }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Invalid Authenticode signature for ${Path}: $($signature.Status) $($signature.StatusMessage)"
    }
}

foreach ($name in @("WINDOWS_CERTIFICATE_BASE64", "WINDOWS_CERTIFICATE_PASSWORD", "WINDOWS_TIMESTAMP_URL", "TAURI_SIGNING_PRIVATE_KEY", "TAURI_SIGNING_PRIVATE_KEY_PASSWORD")) {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) {
        throw "Required signing variable $name is missing. Ordinary builds do not require signing variables."
    }
}

$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$targetRoot = Join-Path $projectRoot "src-tauri\target\release"
$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("watchdog-signing-" + [Guid]::NewGuid().ToString("N"))
$pfxPath = Join-Path $temporaryRoot "certificate.pfx"
$overlayPath = Join-Path $temporaryRoot "tauri.signing.json"
$updaterKeyPath = Join-Path $temporaryRoot "updater.key"
$signingThumbprint = $null
$newImportedThumbprints = @()

try {
    New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
    [System.IO.File]::WriteAllText($updaterKeyPath, $env:TAURI_SIGNING_PRIVATE_KEY, [System.Text.UTF8Encoding]::new($false))
    $env:TAURI_SIGNING_PRIVATE_KEY = $updaterKeyPath
    $encoded = ($env:WINDOWS_CERTIFICATE_BASE64 -replace '\s', '')
    [System.IO.File]::WriteAllBytes($pfxPath, [Convert]::FromBase64String($encoded))
    $password = ConvertTo-SecureString -String $env:WINDOWS_CERTIFICATE_PASSWORD -AsPlainText -Force
    $knownThumbprints = @(Get-ChildItem Cert:\CurrentUser\My | Select-Object -ExpandProperty Thumbprint)
    $importedCertificates = @(Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation Cert:\CurrentUser\My -Password $password -Exportable:$false)
    $newImportedThumbprints = @($importedCertificates.Thumbprint | Where-Object { $knownThumbprints -notcontains $_ })
    $certificate = $importedCertificates |
        Where-Object {
            $_.HasPrivateKey -and @($_.EnhancedKeyUsageList | Where-Object { $_.ObjectId.Value -eq "1.3.6.1.5.5.7.3.3" }).Count -gt 0
        } |
        Select-Object -First 1
    if (-not $certificate -or [string]::IsNullOrWhiteSpace($certificate.Thumbprint)) {
        throw "The signing certificate could not be imported."
    }
    $signingThumbprint = $certificate.Thumbprint
    $signTool = Find-SignTool

    Invoke-Checked { npm run build:all-targets } "Building Windows executables"

    foreach ($name in @("watch-dog.exe", "watch-dog-tester.exe", "watch-dog-target.exe")) {
        $path = Join-Path $targetRoot $name
        Invoke-Checked {
            & $signTool sign /sha1 $signingThumbprint /fd SHA256 /tr $env:WINDOWS_TIMESTAMP_URL /td SHA256 $path
        } "Signing $name"
        Assert-Signed $path
    }

    Invoke-Checked {
        & (Join-Path $PSScriptRoot "stage-sample-target.ps1") -Profile release
    } "Staging the signed Sample Target"

    $overlay = [ordered]@{
        bundle = [ordered]@{
            createUpdaterArtifacts = $true
            windows = [ordered]@{
                certificateThumbprint = $signingThumbprint
                digestAlgorithm = "sha256"
                timestampUrl = $env:WINDOWS_TIMESTAMP_URL
            }
        }
    }
    $overlay | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $overlayPath -Encoding utf8
    Invoke-Checked {
        npx tauri build --bundles nsis --config $overlayPath
    } "Building the signed NSIS installer"

    $packageJson = Get-Content -LiteralPath (Join-Path $projectRoot "package.json") -Raw | ConvertFrom-Json
    $installer = Join-Path $targetRoot "bundle\nsis\watch-dog_$($packageJson.version)_x64-setup.exe"
    $updaterSignature = "$installer.sig"
    foreach ($path in @(
        (Join-Path $targetRoot "watch-dog.exe"),
        (Join-Path $targetRoot "watch-dog-tester.exe"),
        (Join-Path $targetRoot "watch-dog-target.exe"),
        (Join-Path $projectRoot "src-tauri\companions\windows-x64\watch-dog-target.exe"),
        $installer
    )) {
        Assert-Signed $path
    }
    if (-not (Test-Path -LiteralPath $updaterSignature -PathType Leaf) -or (Get-Item -LiteralPath $updaterSignature).Length -eq 0) {
        throw "Tauri updater signature is missing or empty: $updaterSignature"
    }

    $minisign = Get-Command minisign.exe -ErrorAction SilentlyContinue
    if (-not $minisign) { throw "minisign.exe is required to verify the generated updater signature." }
    $productionConfig = Get-Content -LiteralPath (Join-Path $projectRoot "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json
    $publicKey = [string]$productionConfig.plugins.updater.pubkey
    if ([string]::IsNullOrWhiteSpace($publicKey) -or $publicKey.Contains("__WATCHDOG")) {
        throw "Production updater public key is not configured."
    }
    try {
        $minisignPublicDocument = [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($publicKey))
    }
    catch {
        throw "Production updater public key is not a valid Tauri base64 wrapper: $($_.Exception.Message)"
    }
    $publicKeyLine = @($minisignPublicDocument -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })[-1].Trim()
    Invoke-Checked {
        & $minisign.Source -Vm $installer -P $publicKeyLine -x $updaterSignature
    } "Verifying the Tauri updater signature"

    Invoke-Checked {
        & (Join-Path $PSScriptRoot "package-windows.ps1") -Profile release -SkipBuild
    } "Packaging signed portable artifacts"
    Write-Host "Signed Windows artifacts passed Authenticode verification."
}
finally {
    $env:TAURI_SIGNING_PRIVATE_KEY = $null
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $null
    foreach ($thumbprint in $newImportedThumbprints) {
        Remove-Item -LiteralPath "Cert:\CurrentUser\My\$thumbprint" -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}
