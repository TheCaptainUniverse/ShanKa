$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
$releaseDir = Join-Path $repoRoot "src-tauri\target\release"
$releaseExe = Join-Path $releaseDir "shanka.exe"
$msiDir = Join-Path $releaseDir "bundle\msi"
$nsisDir = Join-Path $releaseDir "bundle\nsis"

function Assert-FileExists {
  param(
    [Parameter(Mandatory = $true)]
    [string] $Path,
    [Parameter(Mandatory = $true)]
    [string] $Label
  )

  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
    throw "$Label not found at $Path. Run 'bun run tauri build' first."
  }

  $item = Get-Item -LiteralPath $Path
  if ($item.Length -le 0) {
    throw "$Label is empty at $Path."
  }

  Write-Host "[release-smoke] $Label OK: $($item.FullName) ($($item.Length) bytes)"
}

function Assert-BundleExists {
  param(
    [Parameter(Mandatory = $true)]
    [string] $Directory,
    [Parameter(Mandatory = $true)]
    [string] $Pattern,
    [Parameter(Mandatory = $true)]
    [string] $Label
  )

  if (-not (Test-Path -LiteralPath $Directory -PathType Container)) {
    throw "$Label directory not found at $Directory. Run 'bun run tauri build' first."
  }

  $bundle = Get-ChildItem -LiteralPath $Directory -Filter $Pattern -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

  if ($null -eq $bundle) {
    throw "$Label bundle matching $Pattern not found in $Directory."
  }

  if ($bundle.Length -le 0) {
    throw "$Label bundle is empty at $($bundle.FullName)."
  }

  Write-Host "[release-smoke] $Label OK: $($bundle.FullName) ($($bundle.Length) bytes)"
}

$existingProcesses = Get-Process shanka -ErrorAction SilentlyContinue
if ($existingProcesses) {
  $processList = ($existingProcesses | ForEach-Object { "$($_.Id):$($_.Path)" }) -join "; "
  throw "Existing shanka.exe process found before smoke test: $processList. Close it before running release smoke."
}

Assert-FileExists -Path $releaseExe -Label "release executable"
Assert-BundleExists -Directory $msiDir -Pattern "Shanka_*_x64_en-US.msi" -Label "MSI"
Assert-BundleExists -Directory $nsisDir -Pattern "Shanka_*_x64-setup.exe" -Label "NSIS"

$process = $null
$secondProcess = $null
try {
  $process = Start-Process -FilePath $releaseExe -WorkingDirectory $releaseDir -PassThru -WindowStyle Hidden
  Start-Sleep -Seconds 5
  $process.Refresh()

  if ($process.HasExited) {
    throw "Release executable exited early with code $($process.ExitCode)."
  }

  Write-Host "[release-smoke] release executable started successfully with pid $($process.Id)"

  $secondProcess = Start-Process -FilePath $releaseExe -WorkingDirectory $releaseDir -PassThru -WindowStyle Hidden
  Start-Sleep -Seconds 3
  $secondProcess.Refresh()
  $process.Refresh()

  if (-not $secondProcess.HasExited) {
    throw "Second release executable instance kept running with pid $($secondProcess.Id). Single-instance protection is not working."
  }

  if ($process.HasExited) {
    throw "First release executable exited after second launch with code $($process.ExitCode)."
  }

  Write-Host "[release-smoke] second launch exited while first instance stayed alive"
}
finally {
  if ($null -ne $secondProcess) {
    $secondProcess.Refresh()
    if (-not $secondProcess.HasExited) {
      Stop-Process -Id $secondProcess.Id -Force
      Write-Host "[release-smoke] stopped second pid $($secondProcess.Id)"
    }
  }

  if ($null -ne $process) {
    $process.Refresh()
    if (-not $process.HasExited) {
      Stop-Process -Id $process.Id -Force
      Start-Sleep -Seconds 1
      Write-Host "[release-smoke] stopped pid $($process.Id)"
    }
  }
}
