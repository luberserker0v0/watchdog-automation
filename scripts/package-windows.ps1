param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "release",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

function Get-PackageRelativePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$BasePath,
        [Parameter(Mandatory = $true)]
        [string]$FilePath
    )

    $baseUri = [Uri](([System.IO.Path]::GetFullPath($BasePath).TrimEnd('\') + '\'))
    $fileUri = [Uri][System.IO.Path]::GetFullPath($FilePath)
    return [Uri]::UnescapeDataString($baseUri.MakeRelativeUri($fileUri).ToString()).Replace('/', '\')
}

function Get-Sha256Hex {
    param([Parameter(Mandatory = $true)][string]$FilePath)

    $stream = [System.IO.File]::OpenRead($FilePath)
    try {
        $sha256 = [System.Security.Cryptography.SHA256]::Create()
        try {
            return -join ($sha256.ComputeHash($stream) | ForEach-Object { $_.ToString("x2") })
        }
        finally {
            $sha256.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$packageJson = Get-Content -LiteralPath (Join-Path $projectRoot "package.json") -Raw | ConvertFrom-Json
$version = $packageJson.version
$artifactRoot = [System.IO.Path]::GetFullPath((Join-Path $projectRoot "artifacts"))
$packageName = "watch-dog-$version-windows-x64"
if ($Profile -eq "debug") {
    $packageName = "$packageName-debug"
}
$packageRoot = [System.IO.Path]::GetFullPath((Join-Path $artifactRoot $packageName))
$artifactPrefix = $artifactRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
if (-not $packageRoot.StartsWith($artifactPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to package outside the artifact root: $packageRoot"
}

if (-not $SkipBuild) {
    $buildCommand = if ($Profile -eq "release") { "build:all-targets" } else { "build:all-targets:debug" }
    & npm run $buildCommand
    if ($LASTEXITCODE -ne 0) {
        throw "Build command failed: npm run $buildCommand"
    }
}

& (Join-Path $PSScriptRoot "install-onnxruntime.ps1") -Profile $Profile

if (Test-Path -LiteralPath $packageRoot) {
    Remove-Item -LiteralPath $packageRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $packageRoot -Force | Out-Null

$targetDirectory = Join-Path $projectRoot "src-tauri\target\$Profile"
foreach ($name in @("watch-dog.exe", "watch-dog-tester.exe", "watch-dog-target.exe")) {
    $source = Join-Path $targetDirectory $name
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Missing executable. Build all targets first: $source"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $packageRoot $name)
}

$runtimeDlls = @(Get-ChildItem -LiteralPath $targetDirectory -Filter "*.dll" -File)
if (-not ($runtimeDlls.Name -contains "onnxruntime.dll")) {
    throw "ONNX Runtime was not installed into $targetDirectory"
}
foreach ($runtimeDll in $runtimeDlls) {
    Copy-Item -LiteralPath $runtimeDll.FullName -Destination (Join-Path $packageRoot $runtimeDll.Name)
}

Copy-Item -LiteralPath (Join-Path $projectRoot "src-tauri\resources") -Destination (Join-Path $packageRoot "resources") -Recurse
Copy-Item -LiteralPath (Join-Path $projectRoot "schemas") -Destination (Join-Path $packageRoot "schemas") -Recurse
foreach ($name in @("LICENSE", "NOTICE", "README.md", "CHANGELOG.md")) {
    Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination (Join-Path $packageRoot $name)
}

$files = @(Get-ChildItem -LiteralPath $packageRoot -Recurse -File | Sort-Object FullName | ForEach-Object {
    [ordered]@{
        path = (Get-PackageRelativePath -BasePath $packageRoot -FilePath $_.FullName).Replace('\', '/')
        bytes = $_.Length
        sha256 = Get-Sha256Hex -FilePath $_.FullName
    }
})
$manifest = [ordered]@{
    product = "WatchDog"
    version = $version
    platform = "windows"
    architecture = "x64"
    profile = $Profile
    created_at_utc = [DateTime]::UtcNow.ToString("o")
    files = $files
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $packageRoot "manifest.json") -Encoding utf8

$checksumEntries = @(Get-ChildItem -LiteralPath $packageRoot -Recurse -File |
    Where-Object { $_.Name -notin @("checksums.sha256", "manifest.json") } |
    Sort-Object FullName |
    ForEach-Object {
        $relative = (Get-PackageRelativePath -BasePath $packageRoot -FilePath $_.FullName).Replace('\', '/')
        $hash = Get-Sha256Hex -FilePath $_.FullName
        "$hash  $relative"
    })
$checksumEntries | Set-Content -LiteralPath (Join-Path $packageRoot "checksums.sha256") -Encoding utf8

& (Join-Path $PSScriptRoot "verify-windows-package.ps1") -PackagePath $packageRoot

$archivePath = Join-Path $artifactRoot "$packageName.zip"
if (Test-Path -LiteralPath $archivePath) {
    Remove-Item -LiteralPath $archivePath -Force
}
Compress-Archive -LiteralPath $packageRoot -DestinationPath $archivePath -CompressionLevel Optimal
$archiveHash = Get-Sha256Hex -FilePath $archivePath
$archiveChecksumPath = "$archivePath.sha256"
"$archiveHash  $([System.IO.Path]::GetFileName($archivePath))" |
    Set-Content -LiteralPath $archiveChecksumPath -Encoding ascii
Write-Host "Created portable package: $packageRoot"
Write-Host "Created archive: $archivePath"
Write-Host "Created archive checksum: $archiveChecksumPath"
