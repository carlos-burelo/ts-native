param(
    [switch]$NoBuild,
    [switch]$NoPersist
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$tsnHome = Join-Path $HOME ".tsn"
$binDir = Join-Path $tsnHome "bin"
$stdlibDir = Join-Path $tsnHome "stdlib"
$cacheDir = Join-Path $tsnHome "cache"

Write-Host "[tsn] repo root: $repoRoot"
Write-Host "[tsn] install dir: $tsnHome"

New-Item -ItemType Directory -Force -Path $binDir | Out-Null
New-Item -ItemType Directory -Force -Path $stdlibDir | Out-Null
New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null

if (-not $NoBuild) {
    Write-Host "[tsn] building release binaries..."
    Push-Location $repoRoot
    cargo build --release --bin tsn
    cargo build --release --bin tsn-lsp
    Pop-Location
}

$exeSource = Join-Path $repoRoot "target\release\tsn.exe"
if (-not (Test-Path $exeSource)) {
    throw "release binary not found at $exeSource"
}

$exeDest = Join-Path $binDir "tsn.exe"
Copy-Item -Force $exeSource $exeDest

$lspSource = Join-Path $repoRoot "target\release\tsn-lsp.exe"
if (-not (Test-Path $lspSource)) {
    throw "release lsp binary not found at $lspSource"
}

$lspDest = Join-Path $binDir "tsn-lsp.exe"
Copy-Item -Force $lspSource $lspDest

$stdlibSource = Join-Path $repoRoot "tsn-stdlib"
if (-not (Test-Path $stdlibSource)) {
    throw "tsn-stdlib folder not found at $stdlibSource"
}

Get-ChildItem -Force $stdlibDir | Remove-Item -Recurse -Force
Copy-Item -Recurse -Force (Join-Path $stdlibSource "*") $stdlibDir

if (-not $NoPersist) {
    [Environment]::SetEnvironmentVariable("TSN_HOME", $tsnHome, "User")
    [Environment]::SetEnvironmentVariable("TSN_STDLIB", $stdlibDir, "User")
    [Environment]::SetEnvironmentVariable("TSN_CACHE_DIR", $cacheDir, "User")

    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($currentPath)) {
        $newPath = $binDir
    } elseif ($currentPath -notlike "*$binDir*") {
        $newPath = "$currentPath;$binDir"
    } else {
        $newPath = $currentPath
    }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
}

Write-Host ""
Write-Host "[tsn] installed:"
Write-Host "  binary : $exeDest"
Write-Host "  lsp    : $lspDest"
Write-Host "  stdlib : $stdlibDir"
Write-Host "  cache  : $cacheDir"
Write-Host ""
Write-Host "[tsn] verify with:"
Write-Host "  tsn doctor"
Write-Host "  tsn .\examples\production-test.tsn"
