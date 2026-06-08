param(
  [string] $ConfigDir = "",
  [string] $ReleaseExe = "",
  [switch] $ReuseConfig,
  [switch] $Wait,
  [switch] $SmokeOnly
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
$releaseDir = Join-Path $repoRoot "src-tauri\target\release"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "ShankaManualTextTest"
$defaultConfigDir = Join-Path $tempRoot "config"
$notepadTextPath = Join-Path $tempRoot "notepad-multiline.txt"
$browserTextPath = Join-Path $tempRoot "browser-readonly.html"

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

$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = $ReleaseExe
$startInfo.WorkingDirectory = Split-Path -Parent $ReleaseExe
$startInfo.UseShellExecute = $false
$startInfo.EnvironmentVariables["SHANKA_CONFIG_DIR"] = $ConfigDir
$process = [System.Diagnostics.Process]::Start($startInfo)
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

if ($SmokeOnly) {
  Stop-Process -Id $process.Id -Force
  Start-Sleep -Seconds 1
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
Write-Host "5. Record results in docs/RELEASE_TEST_MATRIX.md."
Write-Host ""
Write-Host "Close Shanka from tray when done. Isolated config remains at: $ConfigDir"

if ($Wait) {
  Wait-Process -Id $process.Id
}
