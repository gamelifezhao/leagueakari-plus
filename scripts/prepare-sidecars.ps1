param(
  [switch]$Release
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$profile = if ($Release) { "release" } else { "debug" }
$buildArg = if ($Release) { "--release" } else { "" }
$targetTriple = "x86_64-pc-windows-msvc"

$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargoCommand) {
  $cargo = $cargoCommand.Source
} else {
  $cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
}

if (!(Test-Path -LiteralPath $cargo)) {
  throw "cargo.exe was not found. Install Rust or add cargo to PATH."
}

Push-Location $root
try {
  if ($buildArg) {
    & $cargo build $buildArg -p leagueakari-probe
  } else {
    & $cargo build -p leagueakari-probe
  }
  if ($LASTEXITCODE -ne 0) {
    throw "leagueakari-probe build failed with exit code $LASTEXITCODE"
  }

  $source = Join-Path $root "target\$profile\leagueakari-probe.exe"
  $target = Join-Path $root "target\$profile\leagueakari-probe-$targetTriple.exe"

  if (!(Test-Path -LiteralPath $source)) {
    throw "probe executable was not found: $source"
  }

  Copy-Item -LiteralPath $source -Destination $target -Force
  Write-Host "Prepared Tauri sidecar: $target"
} finally {
  Pop-Location
}
