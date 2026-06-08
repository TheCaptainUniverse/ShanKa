param(
  [string] $InstallerPath = "",
  [switch] $MetadataOnly,
  [switch] $RequireInstall,
  [switch] $AllowExistingInstall
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
$msiDir = Join-Path $repoRoot "src-tauri\target\release\bundle\msi"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "ShankaMsiSmoke"
$installDir = Join-Path $tempRoot "Install"
$logDir = Join-Path $tempRoot "Logs"
$installLog = Join-Path $logDir "install.log"
$uninstallLog = Join-Path $logDir "uninstall.log"
$installedExe = Join-Path $installDir "shanka.exe"
$expectedUpgradeCode = "{C73D6704-1EE6-502C-8C03-74EC42C15910}"

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

function Test-IsElevated {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = New-Object Security.Principal.WindowsPrincipal($identity)
  return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
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

function Get-MsiRows {
  param(
    [Parameter(Mandatory = $true)] $Database,
    [Parameter(Mandatory = $true)][string] $Sql
  )

  $rows = @()
  $view = $Database.GetType().InvokeMember("OpenView", "InvokeMethod", $null, $Database, @($Sql))
  try {
    $view.GetType().InvokeMember("Execute", "InvokeMethod", $null, $view, $null) | Out-Null
    while ($true) {
      $record = $view.GetType().InvokeMember("Fetch", "InvokeMethod", $null, $view, $null)
      if ($null -eq $record) {
        break
      }

      $fieldCount = $record.GetType().InvokeMember("FieldCount", "GetProperty", $null, $record, $null)
      $fields = @()
      for ($index = 1; $index -le $fieldCount; $index++) {
        $fields += $record.GetType().InvokeMember("StringData", "GetProperty", $null, $record, @($index))
      }
      $rows += [PSCustomObject] @{ Fields = [string[]] $fields }
    }
  }
  finally {
    if ($null -ne $view) {
      $view.GetType().InvokeMember("Close", "InvokeMethod", $null, $view, $null) | Out-Null
    }
  }

  return $rows
}

function Get-MsiSummary {
  param([Parameter(Mandatory = $true)][string] $Path)

  $installer = New-Object -ComObject WindowsInstaller.Installer
  $database = $installer.GetType().InvokeMember("OpenDatabase", "InvokeMethod", $null, $installer, @($Path, 0))

  $propertyRows = Get-MsiRows -Database $database -Sql "SELECT Property, Value FROM Property"
  $properties = @{}
  foreach ($row in $propertyRows) {
    $properties[$row.Fields[0]] = $row.Fields[1]
  }

  $fileRows = Get-MsiRows -Database $database -Sql "SELECT ``File``, FileName FROM ``File``"
  $shortcutRows = Get-MsiRows -Database $database -Sql "SELECT Shortcut, Directory_, Name, Target FROM Shortcut"

  return [PSCustomObject] @{
    Properties = $properties
    Files = $fileRows
    Shortcuts = $shortcutRows
  }
}

function Get-LongMsiName {
  param([Parameter(Mandatory = $true)][string] $Name)

  $parts = $Name -split "\|"
  return $parts[$parts.Length - 1]
}

function Assert-MsiMetadata {
  param(
    [Parameter(Mandatory = $true)][string] $Path,
    [Parameter(Mandatory = $true)] $Summary
  )

  $packageJson = Get-Content -LiteralPath (Join-Path $repoRoot "package.json") -Raw | ConvertFrom-Json
  $expectedVersion = [string] $packageJson.version
  $properties = $Summary.Properties

  $checks = @(
    @("ProductName", "Shanka"),
    @("ProductVersion", $expectedVersion),
    @("Manufacturer", "shanka"),
    @("UpgradeCode", $expectedUpgradeCode),
    @("ALLUSERS", "1")
  )

  foreach ($check in $checks) {
    $name = $check[0]
    $expected = $check[1]
    if (-not $properties.ContainsKey($name)) {
      throw "MSI metadata is missing property $name."
    }
    if (-not ([string] $properties[$name]).Equals($expected, [System.StringComparison]::OrdinalIgnoreCase)) {
      throw "MSI property $name is '$($properties[$name])', expected '$expected'."
    }
  }

  if (-not $properties.ContainsKey("ProductCode") -or [string]::IsNullOrWhiteSpace([string] $properties["ProductCode"])) {
    throw "MSI metadata is missing ProductCode."
  }

  $hasExecutable = $false
  foreach ($fileRow in $Summary.Files) {
    if ((Get-LongMsiName -Name $fileRow.Fields[1]).Equals("shanka.exe", [System.StringComparison]::OrdinalIgnoreCase)) {
      $hasExecutable = $true
      break
    }
  }
  if (-not $hasExecutable) {
    throw "MSI File table does not contain shanka.exe."
  }

  $shortcutNames = @($Summary.Shortcuts | ForEach-Object { Get-LongMsiName -Name $_.Fields[2] })
  if (-not ($shortcutNames -contains "Shanka")) {
    throw "MSI Shortcut table does not contain the Shanka application shortcut."
  }
  if (-not ($shortcutNames -contains "Uninstall Shanka")) {
    throw "MSI Shortcut table does not contain the uninstall shortcut."
  }

  $msiItem = Get-Item -LiteralPath $Path
  Write-Host "[release-msi-smoke] metadata OK: $($msiItem.FullName) ($($msiItem.Length) bytes)"
  Write-Host "[release-msi-smoke] product code: $($properties["ProductCode"])"
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

function Get-UninstallEntries {
  $locations = @(
    @([Microsoft.Win32.RegistryHive]::CurrentUser, [Microsoft.Win32.RegistryView]::Default, "Software\Microsoft\Windows\CurrentVersion\Uninstall"),
    @([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry64, "Software\Microsoft\Windows\CurrentVersion\Uninstall"),
    @([Microsoft.Win32.RegistryHive]::LocalMachine, [Microsoft.Win32.RegistryView]::Registry32, "Software\Microsoft\Windows\CurrentVersion\Uninstall")
  )

  $entries = @()
  foreach ($location in $locations) {
    try {
      $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey($location[0], $location[1])
      $root = $base.OpenSubKey($location[2])
      if ($null -eq $root) {
        continue
      }

      foreach ($subKeyName in $root.GetSubKeyNames()) {
        $subKey = $root.OpenSubKey($subKeyName)
        if ($null -eq $subKey) {
          continue
        }

        try {
          $displayName = [string] $subKey.GetValue("DisplayName")
          if ($displayName.Equals("Shanka", [System.StringComparison]::OrdinalIgnoreCase)) {
            $entries += [PSCustomObject] @{
              Hive = [string] $location[0]
              View = [string] $location[1]
              Key = "$($location[2])\$subKeyName"
              InstallLocation = Normalize-RegistryPath $subKey.GetValue("InstallLocation")
              UninstallString = [string] $subKey.GetValue("UninstallString")
            }
          }
        }
        finally {
          $subKey.Close()
        }
      }
    }
    finally {
      if ($null -ne $root) {
        $root.Close()
      }
      if ($null -ne $base) {
        $base.Close()
      }
    }
  }

  return $entries
}

function Assert-NoExistingInstall {
  $entries = @(Get-UninstallEntries)
  $foreignEntries = @($entries | Where-Object {
      [string]::IsNullOrWhiteSpace($_.InstallLocation) -or
      -not (Test-PathWithin -Path $_.InstallLocation -Root $tempRoot)
    })

  if ($foreignEntries.Count -gt 0 -and -not $AllowExistingInstall) {
    $entryText = ($foreignEntries | ForEach-Object {
        "$($_.Hive)/$($_.View)/$($_.Key) InstallLocation=$($_.InstallLocation)"
      }) -join "; "
    throw "Existing Shanka install detected before MSI smoke: $entryText. Remove it or rerun with -AllowExistingInstall."
  }
}

function Get-ShortcutTarget {
  param([Parameter(Mandatory = $true)][string] $Path)

  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut($Path)
  return $shortcut.TargetPath
}

function Backup-Shortcut {
  param(
    [Parameter(Mandatory = $true)][string] $ShortcutPath,
    [Parameter(Mandatory = $true)][string] $BackupPath
  )

  if (Test-Path -LiteralPath $ShortcutPath -PathType Leaf) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $BackupPath) | Out-Null
    Move-Item -LiteralPath $ShortcutPath -Destination $BackupPath -Force
    Write-Host "[release-msi-smoke] backed up existing shortcut: $ShortcutPath"
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
    Write-Host "[release-msi-smoke] restored shortcut: $ShortcutPath"
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

function Assert-AnyShortcutTarget {
  param(
    [Parameter(Mandatory = $true)][string[]] $ShortcutPaths,
    [Parameter(Mandatory = $true)][string] $ExpectedTarget,
    [Parameter(Mandatory = $true)][string] $Label
  )

  foreach ($shortcutPath in $ShortcutPaths) {
    if (-not (Test-Path -LiteralPath $shortcutPath -PathType Leaf)) {
      continue
    }

    $target = Get-ShortcutTarget -Path $shortcutPath
    if ($target.Equals($ExpectedTarget, [System.StringComparison]::OrdinalIgnoreCase)) {
      Write-Host "[release-msi-smoke] $Label shortcut OK: $shortcutPath"
      return
    }
  }

  throw "$Label shortcut was not created with target $ExpectedTarget."
}

function Invoke-MsiInstallSmoke {
  param(
    [Parameter(Mandatory = $true)][string] $Path,
    [Parameter(Mandatory = $true)][string] $ProductCode
  )

  $existingProcesses = Get-Process shanka -ErrorAction SilentlyContinue
  if ($existingProcesses) {
    $processList = ($existingProcesses | ForEach-Object { "$($_.Id):$($_.Path)" }) -join "; "
    throw "Existing shanka.exe process found before MSI smoke: $processList. Close it before running release MSI smoke."
  }

  Assert-NoExistingInstall
  Assert-PathWithin -Path $tempRoot -Root ([System.IO.Path]::GetTempPath()) -Label "MSI smoke temp root"
  if (Test-Path -LiteralPath $tempRoot) {
    Remove-Item -LiteralPath $tempRoot -Recurse -Force
  }
  New-Item -ItemType Directory -Force -Path $tempRoot, $logDir | Out-Null

  $shortcutCandidates = @(
    Join-Path ([Environment]::GetFolderPath("Desktop")) "Shanka.lnk",
    Join-Path ([Environment]::GetFolderPath("CommonDesktopDirectory")) "Shanka.lnk",
    Join-Path ([Environment]::GetFolderPath("Programs")) "Shanka\Shanka.lnk",
    Join-Path ([Environment]::GetFolderPath("CommonPrograms")) "Shanka\Shanka.lnk",
    Join-Path $installDir "Uninstall Shanka.lnk"
  )
  $backupDir = Join-Path $tempRoot "ShortcutBackup"
  $backups = @()
  $appProcess = $null
  $secondProcess = $null
  $installed = $false

  try {
    foreach ($shortcut in $shortcutCandidates) {
      $backupPath = Join-Path $backupDir (($shortcut -replace "[:\\]", "_") + ".bak")
      [void] (Backup-Shortcut -ShortcutPath $shortcut -BackupPath $backupPath)
      $backups += [PSCustomObject] @{ Shortcut = $shortcut; Backup = $backupPath }
    }

    Write-Host "[release-msi-smoke] installing MSI package: $Path"
    Invoke-ProcessChecked -FilePath "msiexec.exe" -ArgumentList @(
      "/i", "`"$Path`"",
      "/qn",
      "/norestart",
      "INSTALLDIR=`"$installDir`"",
      "/L*v", "`"$installLog`""
    ) -Label "MSI install"
    $installed = $true

    if (-not (Test-Path -LiteralPath $installedExe -PathType Leaf)) {
      throw "Installed executable not found at $installedExe."
    }

    $installedExeItem = Get-Item -LiteralPath $installedExe
    Write-Host "[release-msi-smoke] installed executable OK: $($installedExeItem.FullName) ($($installedExeItem.Length) bytes)"

    Assert-AnyShortcutTarget -ShortcutPaths @(
      Join-Path ([Environment]::GetFolderPath("Programs")) "Shanka\Shanka.lnk",
      Join-Path ([Environment]::GetFolderPath("CommonPrograms")) "Shanka\Shanka.lnk"
    ) -ExpectedTarget $installedExe -Label "Start menu"

    Assert-AnyShortcutTarget -ShortcutPaths @(
      Join-Path ([Environment]::GetFolderPath("Desktop")) "Shanka.lnk",
      Join-Path ([Environment]::GetFolderPath("CommonDesktopDirectory")) "Shanka.lnk"
    ) -ExpectedTarget $installedExe -Label "Desktop"

    $appProcess = Start-Process -FilePath $installedExe -WorkingDirectory $installDir -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 5
    $appProcess.Refresh()
    if ($appProcess.HasExited) {
      throw "Installed executable exited early with code $($appProcess.ExitCode)."
    }
    Write-Host "[release-msi-smoke] installed executable started successfully with pid $($appProcess.Id)"

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
    Write-Host "[release-msi-smoke] second launch exited while first installed instance stayed alive"

    if (-not $appProcess.CloseMainWindow()) {
      throw "Could not send close request to the installed executable main window."
    }

    Start-Sleep -Seconds 3
    $appProcess.Refresh()
    if ($appProcess.HasExited) {
      throw "Installed executable exited after main window close request with code $($appProcess.ExitCode)."
    }
    Write-Host "[release-msi-smoke] close request hid the installed settings window without exiting"
  }
  finally {
    if ($null -ne $secondProcess) {
      $secondProcess.Refresh()
      if (-not $secondProcess.HasExited) {
        Stop-Process -Id $secondProcess.Id -Force
        Write-Host "[release-msi-smoke] stopped second pid $($secondProcess.Id)"
      }
    }

    if ($null -ne $appProcess) {
      $appProcess.Refresh()
      if (-not $appProcess.HasExited) {
        Stop-Process -Id $appProcess.Id -Force
        Start-Sleep -Seconds 1
        Write-Host "[release-msi-smoke] stopped pid $($appProcess.Id)"
      }
    }

    if ($installed) {
      Write-Host "[release-msi-smoke] uninstalling smoke package"
      Invoke-ProcessChecked -FilePath "msiexec.exe" -ArgumentList @(
        "/x", $ProductCode,
        "/qn",
        "/norestart",
        "/L*v", "`"$uninstallLog`""
      ) -Label "MSI uninstall"
    }

    foreach ($shortcut in $shortcutCandidates) {
      Remove-SmokeShortcut -ShortcutPath $shortcut
    }
    foreach ($backup in $backups) {
      Restore-Shortcut -ShortcutPath $backup.Shortcut -BackupPath $backup.Backup
    }

    if (Test-Path -LiteralPath $tempRoot) {
      Assert-PathWithin -Path $tempRoot -Root ([System.IO.Path]::GetTempPath()) -Label "MSI smoke temp root"
      Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
  }

  if (Test-Path -LiteralPath $installDir) {
    throw "MSI smoke install directory still exists after uninstall: $installDir"
  }

  Write-Host "[release-msi-smoke] MSI install smoke completed"
}

if ([string]::IsNullOrWhiteSpace($InstallerPath)) {
  if (-not (Test-Path -LiteralPath $msiDir -PathType Container)) {
    throw "MSI bundle directory not found at $msiDir. Run 'bun run tauri build' first."
  }

  $installer = Get-ChildItem -LiteralPath $msiDir -Filter "Shanka_*_x64_en-US.msi" -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

  if ($null -eq $installer) {
    throw "MSI installer not found in $msiDir. Run 'bun run tauri build' first."
  }

  $InstallerPath = $installer.FullName
}

$InstallerPath = (Resolve-Path -LiteralPath $InstallerPath).Path
if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
  throw "MSI installer not found at $InstallerPath."
}

$summary = Get-MsiSummary -Path $InstallerPath
Assert-MsiMetadata -Path $InstallerPath -Summary $summary

$isElevated = Test-IsElevated
if ($MetadataOnly) {
  Write-Host "[release-msi-smoke] metadata-only mode requested; skipping install smoke"
  exit 0
}

if (-not $isElevated) {
  if ($RequireInstall) {
    throw "MSI install smoke requires an elevated PowerShell session because this package is per-machine."
  }

  Write-Host "[release-msi-smoke] current PowerShell is not elevated; verified MSI metadata and skipped install smoke"
  Write-Host "[release-msi-smoke] rerun from an elevated PowerShell or pass -RequireInstall to make install coverage mandatory"
  exit 0
}

Invoke-MsiInstallSmoke -Path $InstallerPath -ProductCode ([string] $summary.Properties["ProductCode"])
