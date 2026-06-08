param(
  [string] $ConfigDir = "",
  [string] $ReleaseExe = "",
  [string] $ReportPath = "",
  [string] $LogPath = "",
  [switch] $ReuseConfig,
  [switch] $Wait,
  [switch] $SmokeOnly,
  [switch] $OpenReport,
  [switch] $CaptureLog
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
$releaseDir = Join-Path $repoRoot "src-tauri\target\release"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "ShankaManualTextTest"
$defaultConfigDir = Join-Path $tempRoot "config"
$notepadTextPath = Join-Path $tempRoot "notepad-multiline.txt"
$browserTextPath = Join-Path $tempRoot "browser-readonly.html"
$manualReportDir = Join-Path $repoRoot "docs\release\manual"

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

function Write-IsolatedConfig {
  param([Parameter(Mandatory = $true)][string] $Path)

  New-Item -ItemType Directory -Force -Path $Path | Out-Null
  $config = @{
    schema_version = 1
    hotkeys = @{
      safe_mode = "Ctrl+Alt+Shift+KeyS"
      magic_mode = "Ctrl+Alt+Shift+KeyM"
    }
    settings = @{
      provider = "openai"
      api_key_ref = ""
      base_url = "https://api.openai.com/v1"
      model = ""
      timeout_ms = 8000
      debug_logging = $false
      history_enabled = $true
      launch_at_login = $false
    }
  }

  $json = $config | ConvertTo-Json -Depth 8
  $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText((Join-Path $Path "config.json"), "$json`n", $utf8NoBom)
}

function Write-ManualFixtures {
  New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

  $notepadText = @"
个人介绍
李安然，女，1999年3月出生，现居上海。

专业技能
Python
SQL
Excel
Power BI
Tableau
"@
  $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText($notepadTextPath, "$notepadText`n", $utf8NoBom)

  $browserHtml = @"
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8">
    <title>Shanka Manual Text Test</title>
    <style>
      body { font-family: system-ui, sans-serif; max-width: 760px; margin: 48px auto; line-height: 1.7; }
      textarea, input { display: block; width: 100%; box-sizing: border-box; margin: 12px 0 24px; font: inherit; }
      textarea { min-height: 120px; }
    </style>
  </head>
  <body>
    <h1>Shanka Manual Text Test</h1>
    <p>组织校内学术分享、职业规划讲座与跨院交流活动共15场</p>
    <label>
      Browser input
      <input value="负责用户调研和数据分析，并输出周报给业务团队">
    </label>
    <label>
      Browser textarea
      <textarea>负责清洗销售数据，制作 Power BI 仪表盘，并向管理层解释关键变化</textarea>
    </label>
  </body>
</html>
"@
  [System.IO.File]::WriteAllText($browserTextPath, "$browserHtml`n", $utf8NoBom)
}

function Get-GitValue {
  param([Parameter(Mandatory = $true)][string[]] $Arguments)

  Push-Location -LiteralPath $repoRoot
  try {
    $output = & git @Arguments 2>$null
    if ($LASTEXITCODE -ne 0) {
      return ""
    }

    return ([string] $output).Trim()
  }
  finally {
    Pop-Location
  }
}

function Get-PackageVersion {
  $packageJsonPath = Join-Path $repoRoot "package.json"
  if (-not (Test-Path -LiteralPath $packageJsonPath -PathType Leaf)) {
    return ""
  }

  $packageJson = Get-Content -LiteralPath $packageJsonPath -Raw | ConvertFrom-Json
  return [string] $packageJson.version
}

function Get-DefaultReportPath {
  if ($SmokeOnly) {
    return Join-Path $tempRoot "manual-text-test-smoke-report.md"
  }

  $version = Get-PackageVersion
  if ([string]::IsNullOrWhiteSpace($version)) {
    $version = "unknown"
  }

  $commit = Get-GitValue -Arguments @("rev-parse", "--short", "HEAD")
  if ([string]::IsNullOrWhiteSpace($commit)) {
    $commit = "nogit"
  }

  $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
  return Join-Path $manualReportDir "Shanka_$($version)_$($commit)_windows-text_$timestamp.md"
}

function Get-DefaultLogPath {
  if ($SmokeOnly) {
    return Join-Path $tempRoot "manual-text-test-smoke-process.log"
  }

  $version = Get-PackageVersion
  if ([string]::IsNullOrWhiteSpace($version)) {
    $version = "unknown"
  }

  $commit = Get-GitValue -Arguments @("rev-parse", "--short", "HEAD")
  if ([string]::IsNullOrWhiteSpace($commit)) {
    $commit = "nogit"
  }

  $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
  return Join-Path $manualReportDir "Shanka_$($version)_$($commit)_windows-text_$timestamp.log"
}

function Get-ErrorLogPath {
  param([Parameter(Mandatory = $true)][string] $Path)

  $directory = Split-Path -Parent $Path
  $baseName = [System.IO.Path]::GetFileNameWithoutExtension($Path)
  return Join-Path $directory "$baseName.stderr.log"
}

function Write-ManualReport {
  param(
    [Parameter(Mandatory = $true)][string] $Path,
    [Parameter(Mandatory = $true)][string] $ConfigDirectory,
    [Parameter(Mandatory = $true)][string] $ExecutablePath,
    [Parameter(Mandatory = $true)][string] $ProcessLogPath,
    [Parameter(Mandatory = $true)][string] $ErrorLogPath
  )

  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null

  $fullCommit = Get-GitValue -Arguments @("rev-parse", "HEAD")
  $shortCommit = Get-GitValue -Arguments @("rev-parse", "--short", "HEAD")
  $version = Get-PackageVersion
  $createdAt = Get-Date -Format "yyyy-MM-dd HH:mm:ss zzz"

  $lines = [System.Collections.Generic.List[string]]::new()
  $lines.Add("# Shanka Windows Manual Text Test Report")
  $lines.Add("")
  $lines.Add("| Field | Value |")
  $lines.Add("| --- | --- |")
  $lines.Add("| Created At | $createdAt |")
  $lines.Add("| Git Commit | $fullCommit |")
  $lines.Add("| Short Commit | $shortCommit |")
  $lines.Add("| App Version | $version |")
  $lines.Add("| Release Exe | ``$ExecutablePath`` |")
  $lines.Add("| Config Dir | ``$ConfigDirectory`` |")
  $lines.Add("| Process Log | ``$ProcessLogPath`` |")
  $lines.Add("| Error Log | ``$ErrorLogPath`` |")
  $lines.Add("| Notepad Fixture | ``$notepadTextPath`` |")
  $lines.Add("| Browser Fixture | ``$browserTextPath`` |")
  $lines.Add("| Safe Mode Hotkey | ``Ctrl+Alt+Shift+S`` |")
  $lines.Add("| Magic Mode Hotkey | ``Ctrl+Alt+Shift+M`` |")
  $lines.Add("| Provider | mock rewrite / DeepSeek / OpenAI / OpenRouter / Other |")
  $lines.Add("| Overall Result | Pass / Blocked / Needs Fix |")
  $lines.Add("")
  $lines.Add("## How To Run")
  $lines.Add("")
  $lines.Add('```powershell')
  $lines.Add("bun run release:manual-text-test")
  $lines.Add('```')
  $lines.Add("")
  $lines.Add("To capture Shanka process logs to files while testing, use:")
  $lines.Add("")
  $lines.Add('```powershell')
  $lines.Add("powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release-manual-text-test.ps1 -CaptureLog")
  $lines.Add('```')
  $lines.Add("")
  $lines.Add("Use the fixtures opened by the launcher. Record exact failures, target app, selected text shape, and whether the clipboard was restored.")
  $lines.Add("")
  $lines.Add("## Core Text Link Results")
  $lines.Add("")
  $lines.Add("| ID | Target | Scenario | Expected | Result | Notes |")
  $lines.Add("| --- | --- | --- | --- | --- | --- |")
  $lines.Add("| WIN-CORE-01 | Notepad | Safe Preview on single-line Chinese text | Preview card appears near cursor; copy, replace, regenerate work. |  |  |")
  $lines.Add("| WIN-CORE-02 | Notepad | Magic Replace on multi-line Chinese text | Selected text is replaced; history and undo affordance remain available. |  |  |")
  $lines.Add("| WIN-CORE-03 | Browser input | Safe Preview, edit result, replace | Replacement uses edited preview text. |  |  |")
  $lines.Add("| WIN-CORE-04 | Browser read-only paragraph | Safe copy | Result is copied without mutating page content. |  |  |")
  $lines.Add("| WIN-CORE-05 | Notepad++ | Safe and Magic on multi-line list | Clipboard capture stays stable without app-specific adapter. |  |  |")
  $lines.Add("| WIN-CORE-06 | Office/WPS | Safe Replace on paragraph | Replaces if allowed; otherwise keeps result on clipboard with clear HUD state. |  |  |")
  $lines.Add("| WIN-CORE-07 | Elevated target | Magic Replace from non-elevated Shanka | Paste blocked path preserves result on clipboard. |  |  |")
  $lines.Add("| WIN-CORE-08 | No selection | Safe and Magic without selected text | Localized error; no model call; clipboard is not polluted. |  |  |")
  $lines.Add("")
  $lines.Add("## Safe Preview Results")
  $lines.Add("")
  $lines.Add("| ID | Scenario | Expected | Result | Notes |")
  $lines.Add("| --- | --- | --- | --- | --- |")
  $lines.Add("| SAFE-01 | Loading state | Spinner appears until rewrite completes; no stale ready state. |  |  |")
  $lines.Add("| SAFE-02 | Copy | Copies edited result and shows success feedback. |  |  |")
  $lines.Add("| SAFE-03 | Replace | Replaces selection and closes after completion feedback. |  |  |")
  $lines.Add("| SAFE-04 | Regenerate | Keeps preview card; updates result on success; preserves old result on failure. |  |  |")
  $lines.Add("| SAFE-05 | Temporary persona switch | Regenerates with selected persona without changing global default. |  |  |")
  $lines.Add("| SAFE-06 | Blur close | Clicking outside or pressing Escape closes preview; stale requests do not reopen it. |  |  |")
  $lines.Add("| SAFE-07 | Long text | Preview scrolls; controls remain stable; text does not overflow. |  |  |")
  $lines.Add("")
  $lines.Add("## Provider And Error Results")
  $lines.Add("")
  $lines.Add("| ID | Scenario | Expected | Result | Notes |")
  $lines.Add("| --- | --- | --- | --- | --- |")
  $lines.Add("| CFG-01 | Provider preset | Preset updates Base URL, model, JSON mode policy. |  |  |")
  $lines.Add("| CFG-02 | Connection test success | Localized success message. |  |  |")
  $lines.Add("| CFG-03 | Connection test failure | Friendly localized error; no full API key. |  |  |")
  $lines.Add("| CFG-04 | Keychain save | Config file stores key reference, not plaintext key. |  |  |")
  $lines.Add("| CFG-05 | Privacy logging default | Logs contain length, duration, error code, not full selected text. |  |  |")
  $lines.Add("| CFG-06 | Debug logging | Useful diagnostics; API key remains masked. |  |  |")
  $lines.Add("| CFG-07 | Hotkey recording | Recording current hotkeys does not trigger Safe/Magic flows. |  |  |")
  $lines.Add("| CFG-08 | Hotkey validation | Invalid/duplicate shortcuts show localized friendly messages. |  |  |")
  $lines.Add("")
  $lines.Add("## Blocking Issues")
  $lines.Add("")
  $lines.Add("| ID | Severity | Platform | Scenario | Symptom | Repro Steps | Status |")
  $lines.Add("| --- | --- | --- | --- | --- | --- | --- |")
  $lines.Add("|  |  | Windows |  |  |  |  |")
  $lines.Add("")
  $lines.Add("## Terminal Log Excerpt")
  $lines.Add("")
  $lines.Add("Paste relevant terminal lines or process log lines for Blocker/High issues. Keep privacy logging rules in mind and avoid full API keys.")
  $lines.Add("")
  $lines.Add('```text')
  $lines.Add("")
  $lines.Add('```')
  $lines.Add("")
  $lines.Add("## Closeout")
  $lines.Add("")
  $lines.Add("- Copy confirmed pass/fail details into ``docs/RELEASE_TEST_MATRIX.md``.")
  $lines.Add("- Include terminal or process log excerpts when reporting Blocker/High issues.")
  $lines.Add("- Blocker/High issues must be fixed and committed before RC.")
  $lines.Add("- Close Shanka from the tray after the session.")

  $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText($Path, (($lines -join "`n") + "`n"), $utf8NoBom)
}

if ([string]::IsNullOrWhiteSpace($ReleaseExe)) {
  $ReleaseExe = Join-Path $releaseDir "shanka.exe"
}
$ReleaseExe = (Resolve-Path -LiteralPath $ReleaseExe).Path
if (-not (Test-Path -LiteralPath $ReleaseExe -PathType Leaf)) {
  throw "Release executable not found at $ReleaseExe. Run 'bun run tauri build' first."
}

if ([string]::IsNullOrWhiteSpace($ConfigDir)) {
  $ConfigDir = $defaultConfigDir
}
$ConfigDir = Get-FullPath -Path $ConfigDir

if ([string]::IsNullOrWhiteSpace($ReportPath)) {
  $ReportPath = Get-DefaultReportPath
}
$ReportPath = Get-FullPath -Path $ReportPath

$shouldCaptureLog = $CaptureLog -or -not [string]::IsNullOrWhiteSpace($LogPath)
if ($shouldCaptureLog -and [string]::IsNullOrWhiteSpace($LogPath)) {
  $LogPath = Get-DefaultLogPath
}
if (-not [string]::IsNullOrWhiteSpace($LogPath)) {
  $LogPath = Get-FullPath -Path $LogPath
}
$errorLogPath = if ($shouldCaptureLog) { Get-ErrorLogPath -Path $LogPath } else { "not captured" }
$reportProcessLogPath = if ($shouldCaptureLog) { $LogPath } else { "not captured" }

$existingProcesses = Get-Process shanka -ErrorAction SilentlyContinue
if ($existingProcesses) {
  $processList = ($existingProcesses | ForEach-Object { "$($_.Id):$($_.Path)" }) -join "; "
  throw "Existing shanka.exe process found before manual text test: $processList. Close it before starting an isolated manual test."
}

Assert-PathWithin -Path $tempRoot -Root ([System.IO.Path]::GetTempPath()) -Label "manual text test temp root"
if ((Test-Path -LiteralPath $tempRoot) -and -not $ReuseConfig) {
  Remove-Item -LiteralPath $tempRoot -Recurse -Force
}

Write-ManualFixtures
if ((-not (Test-Path -LiteralPath (Join-Path $ConfigDir "config.json"))) -or -not $ReuseConfig) {
  Write-IsolatedConfig -Path $ConfigDir
}
Write-ManualReport -Path $ReportPath -ConfigDirectory $ConfigDir -ExecutablePath $ReleaseExe -ProcessLogPath $reportProcessLogPath -ErrorLogPath $errorLogPath

if ($shouldCaptureLog) {
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LogPath), (Split-Path -Parent $errorLogPath) | Out-Null
  Remove-Item -LiteralPath $LogPath, $errorLogPath -Force -ErrorAction SilentlyContinue

  $previousConfigDir = $env:SHANKA_CONFIG_DIR
  $env:SHANKA_CONFIG_DIR = $ConfigDir
  try {
    $process = Start-Process `
      -FilePath $ReleaseExe `
      -WorkingDirectory (Split-Path -Parent $ReleaseExe) `
      -RedirectStandardOutput $LogPath `
      -RedirectStandardError $errorLogPath `
      -PassThru
  }
  finally {
    if ($null -eq $previousConfigDir) {
      Remove-Item Env:SHANKA_CONFIG_DIR -ErrorAction SilentlyContinue
    }
    else {
      $env:SHANKA_CONFIG_DIR = $previousConfigDir
    }
  }
}
else {
  $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = $ReleaseExe
  $startInfo.WorkingDirectory = Split-Path -Parent $ReleaseExe
  $startInfo.UseShellExecute = $false
  $startInfo.EnvironmentVariables["SHANKA_CONFIG_DIR"] = $ConfigDir
  $process = [System.Diagnostics.Process]::Start($startInfo)
}
Start-Sleep -Seconds 5
$process.Refresh()
if ($process.HasExited) {
  throw "Shanka exited early with code $($process.ExitCode)."
}

$configPath = Join-Path $ConfigDir "config.json"
if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) {
  throw "Isolated config was not created at $configPath."
}

Write-Host "[release-manual-text-test] Shanka started with isolated config: $ConfigDir"
Write-Host "[release-manual-text-test] Safe Mode hotkey: Ctrl+Alt+Shift+S"
Write-Host "[release-manual-text-test] Magic Mode hotkey: Ctrl+Alt+Shift+M"
Write-Host "[release-manual-text-test] Notepad fixture: $notepadTextPath"
Write-Host "[release-manual-text-test] Browser fixture: $browserTextPath"
Write-Host "[release-manual-text-test] Manual report: $ReportPath"
if ($shouldCaptureLog) {
  Write-Host "[release-manual-text-test] Process log: $LogPath"
  Write-Host "[release-manual-text-test] Error log: $errorLogPath"
}

if ($SmokeOnly) {
  Stop-Process -Id $process.Id -Force
  [void] $process.WaitForExit(5000)
  Start-Sleep -Seconds 1
  if ($shouldCaptureLog -and -not (Test-Path -LiteralPath $LogPath -PathType Leaf)) {
    throw "Process log was not created at $LogPath."
  }
  if (Test-Path -LiteralPath $tempRoot) {
    Assert-PathWithin -Path $tempRoot -Root ([System.IO.Path]::GetTempPath()) -Label "manual text test temp root"
    Remove-Item -LiteralPath $tempRoot -Recurse -Force
  }
  Write-Host "[release-manual-text-test] smoke completed"
  exit 0
}

Start-Process -FilePath "notepad.exe" -ArgumentList @($notepadTextPath) | Out-Null
Start-Process -FilePath $browserTextPath | Out-Null

Write-Host ""
Write-Host "Manual checklist:"
Write-Host "1. In Notepad, select one line and press Ctrl+Alt+Shift+S. Verify preview/copy/replace/regenerate."
Write-Host "2. In Notepad, select multiple lines and press Ctrl+Alt+Shift+M. Verify direct replacement and history."
Write-Host "3. In the browser input/textarea, test Safe replacement with edited preview text."
Write-Host "4. On the read-only paragraph, test Safe copy without changing the page."
Write-Host "5. Record results in the generated report, then copy the final status to docs/RELEASE_TEST_MATRIX.md."
Write-Host ""
Write-Host "Generated report: $ReportPath"
if ($shouldCaptureLog) {
  Write-Host "Process log: $LogPath"
  Write-Host "Error log: $errorLogPath"
}
Write-Host "Close Shanka from tray when done. Isolated config remains at: $ConfigDir"

if ($OpenReport) {
  Start-Process -FilePath $ReportPath | Out-Null
}

if ($Wait) {
  Wait-Process -Id $process.Id
}
