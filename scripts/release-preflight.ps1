$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")

function Invoke-Step {
  param(
    [Parameter(Mandatory = $true)]
    [string] $Name,
    [Parameter(Mandatory = $true)]
    [string[]] $Command
  )

  Write-Host "[release-preflight] starting $Name"
  Push-Location -LiteralPath $repoRoot
  try {
    $arguments = @()
    if ($Command.Length -gt 1) {
      $arguments = $Command[1..($Command.Length - 1)]
    }

    & $Command[0] @arguments
    $exitCode = if ($null -eq $global:LASTEXITCODE) { 0 } else { $global:LASTEXITCODE }
  }
  finally {
    Pop-Location
  }

  if ($exitCode -ne 0) {
    throw "$Name failed with exit code $exitCode."
  }

  Write-Host "[release-preflight] completed $Name"
}

Invoke-Step -Name "quality gate" -Command @("bun", "run", "check")
Invoke-Step -Name "Tauri bundles" -Command @("bun", "run", "tauri", "build")
Invoke-Step -Name "release executable smoke" -Command @("bun", "run", "release:smoke")
Invoke-Step -Name "NSIS installer smoke" -Command @("bun", "run", "release:install-smoke")
Invoke-Step -Name "MSI package smoke" -Command @("bun", "run", "release:msi-smoke")
Invoke-Step -Name "release manifest" -Command @("bun", "run", "release:manifest")

Write-Host "[release-preflight] all checks completed"
