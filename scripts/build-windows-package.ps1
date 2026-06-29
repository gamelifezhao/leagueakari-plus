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

$distDir = Join-Path $root "dist"
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$portableName = "LeagueAkari-Plus-$Version-portable-$stamp"
$portableDir = Join-Path $distDir $portableName
$zipPath = Join-Path $distDir "$portableName.zip"
$installerWorkDir = Join-Path $distDir "installer-work"
$installerPath = Join-Path $distDir "LeagueAkari-Plus-$Version-setup.exe"
$sedPath = Join-Path $installerWorkDir "leagueakari-plus.sed"

New-Item -ItemType Directory -Force -Path $distDir | Out-Null
New-Item -ItemType Directory -Force -Path $portableDir | Out-Null
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

$readme = @'
LeagueAkari Plus tryout package

How to run:
1. Start League of Legends and sign in first.
2. Run "LeagueAkari Plus.exe".
3. If Windows shows an unknown publisher warning, choose "More info" and continue.

Notes:
- This unsigned test build does not automate gameplay or affect competitive balance.
- The app reads local League Client LCU data for draft, match history, and analysis.
- leagueakari-probe.exe is the local helper process and must stay next to the app.
'@

$installPs1 = @'
$ErrorActionPreference = "Stop"

$appName = "LeagueAkari Plus"
$installDir = Join-Path $env:LOCALAPPDATA $appName
$appExe = Join-Path $installDir "LeagueAkari Plus.exe"

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot "LeagueAkari Plus.exe") -Destination $appExe -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot "leagueakari-probe.exe") -Destination (Join-Path $installDir "leagueakari-probe.exe") -Force
Copy-Item -LiteralPath (Join-Path $PSScriptRoot "README.txt") -Destination (Join-Path $installDir "README.txt") -Force

$shell = New-Object -ComObject WScript.Shell
$shortcuts = @(
  (Join-Path ([Environment]::GetFolderPath("DesktopDirectory")) "$appName.lnk"),
  (Join-Path ([Environment]::GetFolderPath("Programs")) "$appName.lnk")
)

foreach ($shortcutPath in $shortcuts) {
  $shortcut = $shell.CreateShortcut($shortcutPath)
  $shortcut.TargetPath = $appExe
  $shortcut.WorkingDirectory = $installDir
  $shortcut.IconLocation = "$appExe,0"
  $shortcut.Description = "LeagueAkari Plus"
  $shortcut.Save()
}

Write-Host "LeagueAkari Plus installed to $installDir"
'@

$installCmd = @'
@echo off
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install.ps1"
exit /b %ERRORLEVEL%
'@

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
Set-Content -LiteralPath (Join-Path $installerWorkDir "install.ps1") -Value $installPs1 -Encoding UTF8
Set-Content -LiteralPath (Join-Path $installerWorkDir "install.cmd") -Value $installCmd -Encoding ASCII

if (Test-Path -LiteralPath $zipPath) {
  Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -LiteralPath (Get-ChildItem -LiteralPath $portableDir | ForEach-Object { $_.FullName }) -DestinationPath $zipPath -Force

$sourceDir = $installerWorkDir.TrimEnd("\") + "\"
$targetName = $installerPath.Replace("\", "\\")
$sourceName = $sourceDir.Replace("\", "\\")

$sed = @"
[Version]
Class=IEXPRESS
SEDVersion=3
[Options]
PackagePurpose=InstallApp
ShowInstallProgramWindow=1
HideExtractAnimation=1
UseLongFileName=1
InsideCompressed=0
CAB_FixedSize=0
CAB_ResvCodeSigning=0
RebootMode=N
InstallPrompt=
DisplayLicense=
FinishMessage=LeagueAkari Plus has been installed.
TargetName=$targetName
FriendlyName=LeagueAkari Plus
AppLaunched=install.cmd
PostInstallCmd=
AdminQuietInstCmd=install.cmd
UserQuietInstCmd=install.cmd
SourceFiles=SourceFiles
[Strings]
FILE0="LeagueAkari Plus.exe"
FILE1="leagueakari-probe.exe"
FILE2="README.txt"
FILE3="install.ps1"
FILE4="install.cmd"
[SourceFiles]
SourceFiles0=$sourceName
[SourceFiles0]
%FILE0%=
%FILE1%=
%FILE2%=
%FILE3%=
%FILE4%=
"@

Set-Content -LiteralPath $sedPath -Value $sed -Encoding ASCII

& iexpress.exe /N /Q $sedPath
if ($LASTEXITCODE -ne 0) {
  throw "IExpress failed with exit code $LASTEXITCODE"
}

[PSCustomObject]@{
  PortableZip = $zipPath
  Installer = $installerPath
}
