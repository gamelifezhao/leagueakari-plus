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
  "- The main app also has an embedded probe fallback for accidental zip-preview launches."
) -join [Environment]::NewLine

$installCmd = @(
  "@echo off",
  "cd /d ""%~dp0""",
  "powershell -NoProfile -ExecutionPolicy Bypass -File ""%~dp0install.ps1""",
  "exit /b %ERRORLEVEL%"
) -join "`r`n"

$finishCmd = @(
  "@echo off",
  "exit /b 0"
) -join "`r`n"

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
Copy-Item -LiteralPath (Join-Path $PSScriptRoot "installer-install.ps1") -Destination (Join-Path $installerWorkDir "install.ps1") -Force
Set-Content -LiteralPath (Join-Path $installerWorkDir "install.cmd") -Value $installCmd -Encoding ASCII
Set-Content -LiteralPath (Join-Path $installerWorkDir "finish.cmd") -Value $finishCmd -Encoding ASCII

$installScriptPath = Join-Path $installerWorkDir "install.ps1"
if ((Get-Item -LiteralPath $installScriptPath).Length -lt 100) {
  throw "installer install.ps1 was not generated correctly"
}

if (Test-Path -LiteralPath $zipPath) {
  Remove-Item -LiteralPath $zipPath -Force
}
Compress-Archive -LiteralPath (Get-ChildItem -LiteralPath $portableDir | ForEach-Object { $_.FullName }) -DestinationPath $zipPath -Force

$sourceDir = $installerWorkDir.TrimEnd("\") + "\"
$targetName = $installerPath.Replace("\", "\\")
$sourceName = $sourceDir.Replace("\", "\\")

$sed = @(
  "[Version]",
  "Class=IEXPRESS",
  "SEDVersion=3",
  "[Options]",
  "PackagePurpose=InstallApp",
  "ShowInstallProgramWindow=1",
  "HideExtractAnimation=1",
  "UseLongFileName=1",
  "InsideCompressed=0",
  "CAB_FixedSize=0",
  "CAB_ResvCodeSigning=0",
  "RebootMode=N",
  "InstallPrompt=",
  "DisplayLicense=",
  "FinishMessage=LeagueAkari Plus has been installed.",
  "TargetName=$targetName",
  "FriendlyName=LeagueAkari Plus",
  "AppLaunched=install.cmd",
  "PostInstallCmd=finish.cmd",
  "AdminQuietInstCmd=install.cmd",
  "UserQuietInstCmd=install.cmd",
  "SourceFiles=SourceFiles",
  "[Strings]",
  "FILE0=""LeagueAkari Plus.exe""",
  "FILE1=""leagueakari-probe.exe""",
  "FILE2=""README.txt""",
  "FILE3=""install.ps1""",
  "FILE4=""install.cmd""",
  "FILE5=""finish.cmd""",
  "[SourceFiles]",
  "SourceFiles0=$sourceName",
  "[SourceFiles0]",
  "%FILE0%=",
  "%FILE1%=",
  "%FILE2%=",
  "%FILE3%=",
  "%FILE4%=",
  "%FILE5%="
) -join "`r`n"

Set-Content -LiteralPath $sedPath -Value $sed -Encoding ASCII

& iexpress.exe /N /Q $sedPath
$iexpressExitCode = $LASTEXITCODE

if (!(Test-Path -LiteralPath $installerPath) -or (Get-Item -LiteralPath $installerPath).Length -lt 1000000) {
  throw "installer was not generated correctly; IExpress exit code $iexpressExitCode"
}

[PSCustomObject]@{
  PortableZip = $zipPath
  Installer = $installerPath
}

exit 0
