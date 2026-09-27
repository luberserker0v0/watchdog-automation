param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug"
)

$ErrorActionPreference = "Stop"
$runtimeVersion = "1.24.4"
$directMlVersion = "1.15.4"
$projectRoot = Split-Path -Parent $PSScriptRoot
$targetRoot = Join-Path $projectRoot "src-tauri\target"
$downloadCacheRoot = Join-Path $projectRoot "src-tauri\.cache\native-runtime"
$cacheRoot = Join-Path $downloadCacheRoot "onnxruntime-directml-$runtimeVersion"
$archivePath = Join-Path $downloadCacheRoot "Microsoft.ML.OnnxRuntime.DirectML-$runtimeVersion.zip"
$nativeRoot = Join-Path $cacheRoot "runtimes\win-x64\native"
$runtimePath = Join-Path $nativeRoot "onnxruntime.dll"
$directMlCacheRoot = Join-Path $downloadCacheRoot "directml-$directMlVersion"
$directMlArchivePath = Join-Path $downloadCacheRoot "Microsoft.AI.DirectML-$directMlVersion.zip"
$directMlPath = Join-Path $directMlCacheRoot "bin\x64-win\DirectML.dll"
$destinationDirectory = Join-Path $targetRoot $Profile
$bundleDirectory = Join-Path $projectRoot "src-tauri\native\windows-x64"

if (-not (Test-Path -LiteralPath $runtimePath)) {
    New-Item -ItemType Directory -Path $downloadCacheRoot -Force | Out-Null
    $packageVersion = $runtimeVersion.ToLowerInvariant()
    $downloadUrl = "https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime.directml/$packageVersion/microsoft.ml.onnxruntime.directml.$packageVersion.nupkg"
    Write-Host "Downloading ONNX Runtime DirectML $runtimeVersion..."
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
    New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null
    Expand-Archive -LiteralPath $archivePath -DestinationPath $cacheRoot -Force
}

if (-not (Test-Path -LiteralPath $directMlPath)) {
    New-Item -ItemType Directory -Path $downloadCacheRoot -Force | Out-Null
    $packageVersion = $directMlVersion.ToLowerInvariant()
    $downloadUrl = "https://api.nuget.org/v3-flatcontainer/microsoft.ai.directml/$packageVersion/microsoft.ai.directml.$packageVersion.nupkg"
    Write-Host "Downloading DirectML $directMlVersion..."
    Invoke-WebRequest -Uri $downloadUrl -OutFile $directMlArchivePath
    New-Item -ItemType Directory -Path $directMlCacheRoot -Force | Out-Null
    Expand-Archive -LiteralPath $directMlArchivePath -DestinationPath $directMlCacheRoot -Force
}

New-Item -ItemType Directory -Path $destinationDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $bundleDirectory -Force | Out-Null
$runtimeDlls = @(Get-ChildItem -LiteralPath $nativeRoot -Filter "*.dll" -File)
if ($runtimeDlls.Count -eq 0) {
    throw "No runtime DLLs found in $nativeRoot"
}
if (-not (Test-Path -LiteralPath $directMlPath -PathType Leaf)) {
    throw "DirectML runtime DLL not found at $directMlPath"
}
$nativeDlls = @($runtimeDlls) + @(Get-Item -LiteralPath $directMlPath)
foreach ($runtimeDll in $nativeDlls) {
    Copy-Item -LiteralPath $runtimeDll.FullName -Destination (Join-Path $destinationDirectory $runtimeDll.Name) -Force
    Copy-Item -LiteralPath $runtimeDll.FullName -Destination (Join-Path $bundleDirectory $runtimeDll.Name) -Force
}
Write-Host "Installed ONNX Runtime DirectML $runtimeVersion at $destinationDirectory"
Write-Host "Installed DirectML $directMlVersion at $destinationDirectory"
Write-Host "Staged ONNX Runtime bundle resources at $bundleDirectory"
