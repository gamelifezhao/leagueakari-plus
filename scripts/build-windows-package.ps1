param(
  [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargoCommand) {
  $cargo = $cargoCommand.Source
} else {
  $cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
}

if (!(Test-Path -LiteralPath $cargo)) {
  throw "cargo.exe was not found. Install Rust or add cargo to PATH."
}

$rustcCommand = Get-Command rustc -ErrorAction SilentlyContinue
if ($rustcCommand) {
  $rustc = $rustcCommand.Source
} else {
  $rustc = Join-Path (Split-Path $cargo) "rustc.exe"
}

if (!(Test-Path -LiteralPath $rustc)) {
  throw "rustc.exe was not found. Install Rust or add rustc to PATH."
}

$distDir = Join-Path $root "dist"
$resolvedRoot = [System.IO.Path]::GetFullPath($root.Path).TrimEnd("\")
$resolvedDist = [System.IO.Path]::GetFullPath($distDir).TrimEnd("\")
if (!$resolvedDist.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "refusing to clean an unexpected dist directory: $resolvedDist"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$portableName = "LeagueAkari-Plus-$Version-portable-$stamp"
$portableDir = Join-Path $distDir $portableName
$zipPath = Join-Path $distDir "$portableName.zip"
$installerWorkDir = Join-Path $distDir "installer-work"
$installerPath = Join-Path $distDir "LeagueAkari-Plus-$Version-setup.exe"

New-Item -ItemType Directory -Force -Path $distDir | Out-Null

Get-ChildItem -LiteralPath $distDir -Force -ErrorAction SilentlyContinue |
  Where-Object {
    $_.Name -like "LeagueAkari-Plus-*-setup.exe" -or
    $_.Name -like "LeagueAkari-Plus-*-portable-*.zip" -or
    ($_.PSIsContainer -and $_.Name -like "LeagueAkari-Plus-*-portable-*") -or
    $_.Name -like "LeagueAkari-Plus-*-setup.pdb" -or
    $_.Name -like "README*.txt"
  } |
  ForEach-Object {
    try {
      Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction Stop
    } catch {
      Write-Warning "skipping locked dist item: $($_.FullName)"
    }
  }

New-Item -ItemType Directory -Force -Path $portableDir | Out-Null
if (Test-Path -LiteralPath $installerWorkDir) {
  Remove-Item -LiteralPath $installerWorkDir -Recurse -Force
}
if (Test-Path -LiteralPath $installerPath) {
  Remove-Item -LiteralPath $installerPath -Force
}
New-Item -ItemType Directory -Force -Path $installerWorkDir | Out-Null

Push-Location $root
try {
  & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "prepare-sidecars.ps1") -Release
  if ($LASTEXITCODE -ne 0) {
    throw "prepare-sidecars.ps1 failed with exit code $LASTEXITCODE"
  }

  & $cargo build --release -p leagueakari-app
  if ($LASTEXITCODE -ne 0) {
    throw "leagueakari-app release build failed with exit code $LASTEXITCODE"
  }
} finally {
  Pop-Location
}

$appExe = Join-Path $root "target\release\leagueakari-app.exe"
$probeExe = Join-Path $root "target\release\leagueakari-probe.exe"

if (!(Test-Path -LiteralPath $appExe)) {
  throw "main executable was not found: $appExe"
}
if (!(Test-Path -LiteralPath $probeExe)) {
  throw "probe executable was not found: $probeExe"
}

$readme = @(
  "LeagueAkari Plus tryout package",
  "",
  "Recommended:",
  "1. Prefer running LeagueAkari-Plus-$Version-setup.exe.",
  "   The installer lets you choose an install folder and creates a desktop shortcut.",
  "2. Start League of Legends and sign in first.",
  "3. Start LeagueAkari Plus. It will connect to LCU during draft.",
  "",
  "Portable zip:",
  "1. Do not run the app from inside the zip preview window.",
  "2. Extract the whole zip to a normal folder first.",
  "3. Keep LeagueAkari Plus.exe and leagueakari-probe.exe in the same folder.",
  "",
  "Notes:",
  "- This unsigned test build may show a Windows unknown publisher warning.",
  "- This app reads local League Client LCU data only.",
  "- leagueakari-probe.exe is the local helper process. Do not delete it.",
  "- The main app also has an embedded probe fallback for accidental zip-preview launches.",
  "- Installer logs are written to %TEMP%\LeagueAkariPlus-install.log when something fails."
) -join [Environment]::NewLine

$portableFiles = @{
  "LeagueAkari Plus.exe" = $appExe
  "leagueakari-probe.exe" = $probeExe
}

foreach ($entry in $portableFiles.GetEnumerator()) {
  Copy-Item -LiteralPath $entry.Value -Destination (Join-Path $portableDir $entry.Key) -Force
  Copy-Item -LiteralPath $entry.Value -Destination (Join-Path $installerWorkDir $entry.Key) -Force
}

Set-Content -LiteralPath (Join-Path $portableDir "README.txt") -Value $readme -Encoding UTF8
Set-Content -LiteralPath (Join-Path $installerWorkDir "README.txt") -Value $readme -Encoding UTF8

if (Test-Path -LiteralPath $zipPath) {
  Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -LiteralPath (Get-ChildItem -LiteralPath $portableDir | ForEach-Object { $_.FullName }) -DestinationPath $zipPath -Force

$oldAppExeEnv = $env:LEAGUEAKARI_APP_EXE
$oldProbeExeEnv = $env:LEAGUEAKARI_PROBE_EXE
$oldReadmeEnv = $env:LEAGUEAKARI_README
try {
  $env:LEAGUEAKARI_APP_EXE = (Join-Path $installerWorkDir "LeagueAkari Plus.exe")
  $env:LEAGUEAKARI_PROBE_EXE = (Join-Path $installerWorkDir "leagueakari-probe.exe")
  $env:LEAGUEAKARI_README = (Join-Path $installerWorkDir "README.txt")

  & $rustc --edition=2021 -O -o $installerPath (Join-Path $PSScriptRoot "installer-setup.rs")
  if ($LASTEXITCODE -ne 0) {
    throw "installer build failed with exit code $LASTEXITCODE"
  }
} finally {
  $env:LEAGUEAKARI_APP_EXE = $oldAppExeEnv
  $env:LEAGUEAKARI_PROBE_EXE = $oldProbeExeEnv
  $env:LEAGUEAKARI_README = $oldReadmeEnv
}

$installerPdbPath = [System.IO.Path]::ChangeExtension($installerPath, ".pdb")
if (Test-Path -LiteralPath $installerPdbPath) {
  Remove-Item -LiteralPath $installerPdbPath -Force
}

if (!(Test-Path -LiteralPath $installerPath) -or (Get-Item -LiteralPath $installerPath).Length -lt 1000000) {
  throw "installer was not generated correctly"
}

$installer = Get-Item -LiteralPath $installerPath
$portable = Get-Item -LiteralPath $zipPath

Remove-Item -LiteralPath $portableDir -Recurse -Force
Remove-Item -LiteralPath $installerWorkDir -Recurse -Force

[PSCustomObject]@{
  PortableZip = $portable.FullName
  Installer = $installer.FullName
}

exit 0
