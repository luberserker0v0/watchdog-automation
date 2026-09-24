param(
    [Parameter(Mandatory = $true)]
    [string]$PackagePath
)

$ErrorActionPreference = "Stop"
$packageRoot = [System.IO.Path]::GetFullPath($PackagePath)
if (-not (Test-Path -LiteralPath $packageRoot -PathType Container)) {
    throw "Package directory does not exist: $packageRoot"
}
$packagePrefix = $packageRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar

function Resolve-PackageFile {
    param([Parameter(Mandatory = $true)][string]$RelativePath)

    $resolved = [System.IO.Path]::GetFullPath((Join-Path $packageRoot $RelativePath))
    if (-not $resolved.StartsWith($packagePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Package entry escapes the package directory: $RelativePath"
    }
    return $resolved
}

function Get-PackageRelativePath {
    param([Parameter(Mandatory = $true)][string]$FilePath)

    $baseUri = [Uri]($packagePrefix)
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

$requiredFiles = @(
    "watch-dog.exe",
    "watch-dog-tester.exe",
    "watch-dog-target.exe",
    "onnxruntime.dll",
    "onnxruntime_providers_shared.dll",
    "DirectML.dll",
    "resources\pipelines\sample\sample.md",
    "resources\pipelines\sample\sample.meta.json",
    "resources\pipelines\patrol-redirection\patrol-redirection.md",
    "resources\pipelines\patrol-redirection\patrol-redirection.meta.json",
    "resources\models\ocr\det.onnx",
    "resources\models\ocr\rec.onnx",
    "resources\models\ocr\keys.txt",
    "schemas\watchdog-metadata.schema.json",
    "schemas\watchdog-layout.schema.json",
    "LICENSE",
    "NOTICE",
    "README.md",
    "CHANGELOG.md",
    "manifest.json",
    "checksums.sha256"
)

$missing = @($requiredFiles | Where-Object {
    -not (Test-Path -LiteralPath (Join-Path $packageRoot $_) -PathType Leaf)
})
if ($missing.Count -gt 0) {
    throw "Package is missing required files: $($missing -join ', ')"
}

$unexpectedDebugFiles = @(Get-ChildItem -LiteralPath $packageRoot -Recurse -File | Where-Object {
    $_.Extension -in @(".pdb", ".ilk")
})
if ($unexpectedDebugFiles.Count -gt 0) {
    throw "Package contains debug files: $($unexpectedDebugFiles.FullName -join ', ')"
}

$manifestPath = Join-Path $packageRoot "manifest.json"
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.platform -ne "windows" -or $manifest.architecture -ne "x64") {
    throw "Package manifest must identify windows x64"
}

$manifestEntries = @{}
foreach ($entry in $manifest.files) {
    $normalizedPath = $entry.path.Replace('/', '\')
    if ($manifestEntries.ContainsKey($normalizedPath)) {
        throw "Duplicate manifest entry: $($entry.path)"
    }
    $manifestEntries[$normalizedPath] = $entry.sha256
    $filePath = Resolve-PackageFile -RelativePath $normalizedPath
    if (-not (Test-Path -LiteralPath $filePath -PathType Leaf)) {
        throw "Manifest entry is missing: $($entry.path)"
    }
    $actual = Get-Sha256Hex -FilePath $filePath
    if ($actual -ne $entry.sha256) {
        throw "Checksum mismatch for $($entry.path)"
    }
}

$payloadFiles = @(Get-ChildItem -LiteralPath $packageRoot -Recurse -File | Where-Object {
    $_.Name -notin @("manifest.json", "checksums.sha256")
})
foreach ($payloadFile in $payloadFiles) {
    $relative = Get-PackageRelativePath -FilePath $payloadFile.FullName
    if (-not $manifestEntries.ContainsKey($relative)) {
        throw "Package payload is absent from manifest.json: $relative"
    }
}
if ($manifestEntries.Count -ne $payloadFiles.Count) {
    throw "manifest.json file count does not match the package payload"
}

$checksumEntries = @{}
foreach ($line in Get-Content -LiteralPath (Join-Path $packageRoot "checksums.sha256")) {
    if ($line -notmatch '^([0-9a-fA-F]{64})  (.+)$') {
        throw "Invalid checksums.sha256 line: $line"
    }
    $relative = $Matches[2].Replace('/', '\')
    if ($checksumEntries.ContainsKey($relative)) {
        throw "Duplicate checksum entry: $relative"
    }
    $checksumEntries[$relative] = $Matches[1].ToLowerInvariant()
    $filePath = Resolve-PackageFile -RelativePath $relative
    if (-not (Test-Path -LiteralPath $filePath -PathType Leaf)) {
        throw "Checksum entry is missing: $relative"
    }
    $actual = Get-Sha256Hex -FilePath $filePath
    if ($actual -ne $checksumEntries[$relative]) {
        throw "checksums.sha256 mismatch for $relative"
    }
}
if ($checksumEntries.Count -ne $payloadFiles.Count) {
    throw "checksums.sha256 file count does not match the package payload"
}
foreach ($payloadFile in $payloadFiles) {
    $relative = Get-PackageRelativePath -FilePath $payloadFile.FullName
    if (-not $checksumEntries.ContainsKey($relative)) {
        throw "Package payload is absent from checksums.sha256: $relative"
    }
}

Write-Host "Verified WatchDog $($manifest.version) Windows x64 package at $packageRoot"
Write-Host "Verified $($manifest.files.Count) manifest and checksum entries and all required runtime files."
