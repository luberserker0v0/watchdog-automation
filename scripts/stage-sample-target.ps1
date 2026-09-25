param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"

$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$source = Join-Path $projectRoot "src-tauri\target\$Profile\watch-dog-target.exe"
$stagingRoot = [System.IO.Path]::GetFullPath((Join-Path $projectRoot "src-tauri\companions\windows-x64"))
$expectedRoot = [System.IO.Path]::GetFullPath((Join-Path $projectRoot "src-tauri\companions"))
$expectedPrefix = $expectedRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar

if (-not $stagingRoot.StartsWith($expectedPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to stage outside the companion root: $stagingRoot"
}
if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
    throw "Sample Target executable is missing. Run the Target build first: $source"
}

New-Item -ItemType Directory -Path $stagingRoot -Force | Out-Null
$destination = Join-Path $stagingRoot "watch-dog-target.exe"
Copy-Item -LiteralPath $source -Destination $destination -Force
Write-Host "Staged Sample Target companion at $destination"
