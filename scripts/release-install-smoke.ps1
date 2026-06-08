param(
  [string] $InstallerPath = "",
  [switch] $AllowExistingInstall
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
$nsisDir = Join-Path $repoRoot "src-tauri\target\release\bundle\nsis"
$uninstallSubKey = "Software\Microsoft\Windows\CurrentVersion\Uninstall\Shanka"
$productSubKey = "Software\shanka\Shanka"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "ShankaInstallerSmoke"
$installDir = Join-Path $tempRoot "Install"
$backupDir = Join-Path $tempRoot "ShortcutBackup"
$installedExe = Join-Path $installDir "shanka.exe"
$uninstaller = Join-Path $installDir "uninstall.exe"
$startMenuShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Shanka.lnk"
$desktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "Shanka.lnk"

function Get-FullPath {
  param([Parameter(Mandatory = $true)][string] $Path)
  return [System.IO.Path]::GetFullPath($Path)
}

function Test-PathWithin {
  param(
    [Parameter(Mandatory = $true)][string] $Path,
    [Parameter(Mandatory = $true)][string] $Root
  )

  $fullPath = Get-FullPath -Path $Path
  $fullRoot = (Get-FullPath -Path $Root).TrimEnd([char[]] @(
      [System.IO.Path]::DirectorySeparatorChar,
      [System.IO.Path]::AltDirectorySeparatorChar
    ))
  $rootPrefix = "$fullRoot$([System.IO.Path]::DirectorySeparatorChar)"

  return $fullPath.Equals($fullRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
    $fullPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)
}

function Assert-PathWithin {
  param(
    [Parameter(Mandatory = $true)][string] $Path,
    [Parameter(Mandatory = $true)][string] $Root,
    [Parameter(Mandatory = $true)][string] $Label
  )

  if (-not (Test-PathWithin -Path $Path -Root $Root)) {
    throw "$Label path is outside expected root. Path=$Path Root=$Root"
  }
}

function Get-HkcuValue {
  param(
    [Parameter(Mandatory = $true)][string] $SubKey,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string] $Name
  )

  $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($SubKey)
  if ($null -eq $key) {
    return $null
  }

  try {
    return $key.GetValue($Name)
  }
  finally {
    $key.Close()
  }
}

function Test-HkcuKey {
  param([Parameter(Mandatory = $true)][string] $SubKey)

  $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($SubKey)
  if ($null -eq $key) {
    return $false
  }

  $key.Close()
  return $true
}

function Remove-HkcuKeyTree {
  param([Parameter(Mandatory = $true)][string] $SubKey)

  try {
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($SubKey, $false)
  }
  catch [System.ArgumentException] {
  }
}

function Normalize-RegistryPath {
  param($Value)

  if ($null -eq $Value) {
    return $null
  }

  $text = ([string] $Value).Trim()
  if ($text.Length -eq 0) {
    return $null
  }

  return $text.Trim('"')
}

function Assert-NoExistingInstall {
  $uninstallPath = Normalize-RegistryPath (Get-HkcuValue -SubKey $uninstallSubKey -Name "InstallLocation")
  $productPath = Normalize-RegistryPath (Get-HkcuValue -SubKey $productSubKey -Name "")
  $hasUninstallKey = Test-HkcuKey -SubKey $uninstallSubKey
  $hasProductKey = Test-HkcuKey -SubKey $productSubKey
  $knownPaths = @(@($uninstallPath, $productPath) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
  $foreignPaths = @($knownPaths | Where-Object { -not (Test-PathWithin -Path $_ -Root $tempRoot) })

  if ((($hasUninstallKey -or $hasProductKey) -and $knownPaths.Count -eq 0) -and -not $AllowExistingInstall) {
    throw "Existing Shanka install registry keys found without a known path. Remove the existing install or rerun with -AllowExistingInstall."
  }

  if ($foreignPaths.Count -gt 0 -and -not $AllowExistingInstall) {
    throw "Existing Shanka install detected outside the smoke temp directory: $($foreignPaths -join '; '). Remove it or rerun with -AllowExistingInstall."
  }
}

function Invoke-ProcessChecked {
  param(
    [Parameter(Mandatory = $true)][string] $FilePath,
    [Parameter(Mandatory = $true)][string[]] $ArgumentList,
    [Parameter(Mandatory = $true)][string] $Label
  )

  $process = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -Wait -PassThru -WindowStyle Hidden
  if ($process.ExitCode -ne 0) {
    throw "$Label failed with exit code $($process.ExitCode)."
  }
}

function Get-ShortcutTarget {
  param([Parameter(Mandatory = $true)][string] $Path)

  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut($Path)
  return $shortcut.TargetPath
}

function Assert-ShortcutTarget {
  param(
    [Parameter(Mandatory = $true)][string] $ShortcutPath,
    [Parameter(Mandatory = $true)][string] $ExpectedTarget,
    [Parameter(Mandatory = $true)][string] $Label
  )

  if (-not (Test-Path -LiteralPath $ShortcutPath -PathType Leaf)) {
    throw "$Label shortcut was not created at $ShortcutPath."
  }

  $target = Get-ShortcutTarget -Path $ShortcutPath
  if (-not $target.Equals($ExpectedTarget, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "$Label shortcut points to $target, expected $ExpectedTarget."
  }

  Write-Host "[release-install-smoke] $Label shortcut OK: $ShortcutPath"
}

function Backup-Shortcut {
  param(
    [Parameter(Mandatory = $true)][string] $ShortcutPath,
    [Parameter(Mandatory = $true)][string] $BackupPath
  )

  if (Test-Path -LiteralPath $ShortcutPath -PathType Leaf) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $BackupPath) | Out-Null
    Move-Item -LiteralPath $ShortcutPath -Destination $BackupPath -Force
    Write-Host "[release-install-smoke] backed up existing shortcut: $ShortcutPath"
    return $true
  }

  return $false
}

function Restore-Shortcut {
  param(
    [Parameter(Mandatory = $true)][string] $ShortcutPath,
    [Parameter(Mandatory = $true)][string] $BackupPath
  )

  if (Test-Path -LiteralPath $BackupPath -PathType Leaf) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $ShortcutPath) | Out-Null
    if (Test-Path -LiteralPath $ShortcutPath -PathType Leaf) {
      Remove-Item -LiteralPath $ShortcutPath -Force
    }
    Move-Item -LiteralPath $BackupPath -Destination $ShortcutPath -Force
    Write-Host "[release-install-smoke] restored shortcut: $ShortcutPath"
  }
}

function Remove-SmokeRegistryKeys {
  $uninstallPath = Normalize-RegistryPath (Get-HkcuValue -SubKey $uninstallSubKey -Name "InstallLocation")
  $productPath = Normalize-RegistryPath (Get-HkcuValue -SubKey $productSubKey -Name "")

  if ($null -ne $uninstallPath -and (Test-PathWithin -Path $uninstallPath -Root $tempRoot)) {
    Remove-HkcuKeyTree -SubKey $uninstallSubKey
  }

  if ($null -ne $productPath -and (Test-PathWithin -Path $productPath -Root $tempRoot)) {
    Remove-HkcuKeyTree -SubKey $productSubKey
    try {
      [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKey("Software\shanka", $false)
    }
    catch {
    }
  }
}

function Remove-SmokeShortcut {
  param([Parameter(Mandatory = $true)][string] $ShortcutPath)

  if (-not (Test-Path -LiteralPath $ShortcutPath -PathType Leaf)) {
    return
  }

  $target = Get-ShortcutTarget -Path $ShortcutPath
  if ($target.Equals($installedExe, [System.StringComparison]::OrdinalIgnoreCase)) {
    Remove-Item -LiteralPath $ShortcutPath -Force
  }
}

if ([string]::IsNullOrWhiteSpace($InstallerPath)) {
  if (-not (Test-Path -LiteralPath $nsisDir -PathType Container)) {
    throw "NSIS bundle directory not found at $nsisDir. Run 'bun run tauri build' first."
  }

  $installer = Get-ChildItem -LiteralPath $nsisDir -Filter "Shanka_*_x64-setup.exe" -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

  if ($null -eq $installer) {
    throw "NSIS installer not found in $nsisDir. Run 'bun run tauri build' first."
  }

  $InstallerPath = $installer.FullName
}

$InstallerPath = (Resolve-Path -LiteralPath $InstallerPath).Path
if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
  throw "Installer not found at $InstallerPath."
}

$existingProcesses = Get-Process shanka -ErrorAction SilentlyContinue
if ($existingProcesses) {
  $processList = ($existingProcesses | ForEach-Object { "$($_.Id):$($_.Path)" }) -join "; "
  throw "Existing shanka.exe process found before install smoke: $processList. Close it before running release install smoke."
}

Assert-NoExistingInstall

Assert-PathWithin -Path $tempRoot -Root ([System.IO.Path]::GetTempPath()) -Label "smoke temp root"
if (Test-Path -LiteralPath $tempRoot) {
  Remove-Item -LiteralPath $tempRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $tempRoot, $backupDir | Out-Null

$desktopBackup = Join-Path $backupDir "desktop-Shanka.lnk"
$startMenuBackup = Join-Path $backupDir "startmenu-Shanka.lnk"
$appProcess = $null
$secondProcess = $null

try {
  [void] (Backup-Shortcut -ShortcutPath $desktopShortcut -BackupPath $desktopBackup)
  [void] (Backup-Shortcut -ShortcutPath $startMenuShortcut -BackupPath $startMenuBackup)

  Write-Host "[release-install-smoke] installing NSIS package: $InstallerPath"
  Invoke-ProcessChecked -FilePath $InstallerPath -ArgumentList @("/S", "/D=$installDir") -Label "NSIS install"

  if (-not (Test-Path -LiteralPath $installedExe -PathType Leaf)) {
    throw "Installed executable not found at $installedExe."
  }
  if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
    throw "Uninstaller not found at $uninstaller."
  }

  $installedExeItem = Get-Item -LiteralPath $installedExe
  Write-Host "[release-install-smoke] installed executable OK: $($installedExeItem.FullName) ($($installedExeItem.Length) bytes)"

  $registeredInstall = Normalize-RegistryPath (Get-HkcuValue -SubKey $uninstallSubKey -Name "InstallLocation")
  if ($null -eq $registeredInstall -or -not (Test-PathWithin -Path $registeredInstall -Root $installDir)) {
    throw "Install registry entry does not point to the smoke install directory: $registeredInstall"
  }
  Write-Host "[release-install-smoke] uninstall registry entry OK"

  Assert-ShortcutTarget -ShortcutPath $startMenuShortcut -ExpectedTarget $installedExe -Label "Start menu"
  Assert-ShortcutTarget -ShortcutPath $desktopShortcut -ExpectedTarget $installedExe -Label "Desktop"

  $appProcess = Start-Process -FilePath $installedExe -WorkingDirectory $installDir -PassThru -WindowStyle Hidden
  Start-Sleep -Seconds 5
  $appProcess.Refresh()
  if ($appProcess.HasExited) {
    throw "Installed executable exited early with code $($appProcess.ExitCode)."
  }
  Write-Host "[release-install-smoke] installed executable started successfully with pid $($appProcess.Id)"

  $secondProcess = Start-Process -FilePath $installedExe -WorkingDirectory $installDir -PassThru -WindowStyle Hidden
  Start-Sleep -Seconds 3
  $secondProcess.Refresh()
  $appProcess.Refresh()
  if (-not $secondProcess.HasExited) {
    throw "Second installed instance kept running with pid $($secondProcess.Id)."
  }
  if ($appProcess.HasExited) {
    throw "First installed instance exited after second launch with code $($appProcess.ExitCode)."
  }
  Write-Host "[release-install-smoke] second launch exited while first installed instance stayed alive"

  if (-not $appProcess.CloseMainWindow()) {
    throw "Could not send close request to the installed executable main window."
  }

  Start-Sleep -Seconds 3
  $appProcess.Refresh()
  if ($appProcess.HasExited) {
    throw "Installed executable exited after main window close request with code $($appProcess.ExitCode)."
  }
  Write-Host "[release-install-smoke] close request hid the installed settings window without exiting"
}
finally {
  if ($null -ne $secondProcess) {
    $secondProcess.Refresh()
    if (-not $secondProcess.HasExited) {
      Stop-Process -Id $secondProcess.Id -Force
      Write-Host "[release-install-smoke] stopped second pid $($secondProcess.Id)"
    }
  }

  if ($null -ne $appProcess) {
    $appProcess.Refresh()
    if (-not $appProcess.HasExited) {
      Stop-Process -Id $appProcess.Id -Force
      Start-Sleep -Seconds 1
      Write-Host "[release-install-smoke] stopped pid $($appProcess.Id)"
    }
  }

  if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
    Write-Host "[release-install-smoke] uninstalling smoke package"
    Invoke-ProcessChecked -FilePath $uninstaller -ArgumentList @("/S") -Label "NSIS uninstall"
  }

  Remove-SmokeShortcut -ShortcutPath $desktopShortcut
  Remove-SmokeShortcut -ShortcutPath $startMenuShortcut
  Restore-Shortcut -ShortcutPath $desktopShortcut -BackupPath $desktopBackup
  Restore-Shortcut -ShortcutPath $startMenuShortcut -BackupPath $startMenuBackup
  Remove-SmokeRegistryKeys

  if (Test-Path -LiteralPath $tempRoot) {
    Assert-PathWithin -Path $tempRoot -Root ([System.IO.Path]::GetTempPath()) -Label "smoke temp root"
    Remove-Item -LiteralPath $tempRoot -Recurse -Force
  }
}

if (Test-Path -LiteralPath $installDir) {
  throw "Smoke install directory still exists after uninstall: $installDir"
}

if (Test-HkcuKey -SubKey $uninstallSubKey) {
  throw "Uninstall registry key still exists after smoke uninstall."
}

Write-Host "[release-install-smoke] NSIS install smoke completed"
