$ErrorActionPreference = "Stop"

$appName = "LeagueAkari Plus"
$defaultInstallDir = Join-Path $env:LOCALAPPDATA $appName

if (![string]::IsNullOrWhiteSpace($env:LEAGUEAKARI_INSTALL_DIR)) {
  $installDir = $env:LEAGUEAKARI_INSTALL_DIR
} else {
  New-Item -ItemType Directory -Force -Path $defaultInstallDir | Out-Null
  Add-Type -AssemblyName System.Windows.Forms

  $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
  $dialog.Description = "Choose the LeagueAkari Plus install folder"
  $dialog.SelectedPath = $defaultInstallDir
  $dialog.ShowNewFolderButton = $true

  $result = $dialog.ShowDialog()
  if ($result -ne [System.Windows.Forms.DialogResult]::OK -or [string]::IsNullOrWhiteSpace($dialog.SelectedPath)) {
    throw "Installation canceled."
  }

  $installDir = $dialog.SelectedPath
}

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
