param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug"
)

$ErrorActionPreference = "Stop"
$runtimeVersion = "1.24.4"
$projectRoot = Split-Path -Parent $PSScriptRoot
$targetRoot = Join-Path $projectRoot "src-tauri\target"
$cacheRoot = Join-Path $targetRoot "onnxruntime-directml-$runtimeVersion"
$archivePath = Join-Path $targetRoot "Microsoft.ML.OnnxRuntime.DirectML-$runtimeVersion.zip"
$nativeRoot = Join-Path $cacheRoot "runtimes\win-x64\native"
$runtimePath = Join-Path $nativeRoot "onnxruntime.dll"
$destinationDirectory = Join-Path $targetRoot $Profile

if (-not (Test-Path -LiteralPath $runtimePath)) {
    New-Item -ItemType Directory -Path $targetRoot -Force | Out-Null
    $packageVersion = $runtimeVersion.ToLowerInvariant()
    $downloadUrl = "https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime.directml/$packageVersion/microsoft.ml.onnxruntime.directml.$packageVersion.nupkg"
    Write-Host "Downloading ONNX Runtime DirectML $runtimeVersion..."
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
    New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null
    Expand-Archive -LiteralPath $archivePath -DestinationPath $cacheRoot -Force
}

New-Item -ItemType Directory -Path $destinationDirectory -Force | Out-Null
$runtimeDlls = Get-ChildItem -LiteralPath $nativeRoot -Filter "*.dll" -File
if ($runtimeDlls.Count -eq 0) {
    throw "No runtime DLLs found in $nativeRoot"
}
foreach ($runtimeDll in $runtimeDlls) {
    Copy-Item -LiteralPath $runtimeDll.FullName -Destination (Join-Path $destinationDirectory $runtimeDll.Name) -Force
}
Write-Host "Installed ONNX Runtime DirectML $runtimeVersion at $destinationDirectory"
