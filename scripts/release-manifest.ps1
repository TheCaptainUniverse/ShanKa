$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
$tauriConfigPath = Join-Path $repoRoot "src-tauri\tauri.conf.json"
$releaseDir = Join-Path $repoRoot "src-tauri\target\release"
$bundleRoot = Join-Path $releaseDir "bundle"
$outputDir = Join-Path $repoRoot "docs\release"

if (-not (Test-Path -LiteralPath $tauriConfigPath -PathType Leaf)) {
  throw "Tauri config not found at $tauriConfigPath"
}

if (-not (Test-Path -LiteralPath $bundleRoot -PathType Container)) {
  throw "Bundle directory not found at $bundleRoot. Run 'bun run tauri build' first."
}

$tauriConfig = Get-Content -Raw -LiteralPath $tauriConfigPath | ConvertFrom-Json
$version = [string] $tauriConfig.version
$productName = [string] $tauriConfig.productName
$commit = (git -C $repoRoot rev-parse --short HEAD).Trim()
$generatedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$bundles = @(
  Get-ChildItem -LiteralPath (Join-Path $bundleRoot "msi") -Filter "Shanka_*_x64_en-US.msi" -File -ErrorAction SilentlyContinue
  Get-ChildItem -LiteralPath (Join-Path $bundleRoot "nsis") -Filter "Shanka_*_x64-setup.exe" -File -ErrorAction SilentlyContinue
) | Sort-Object FullName

if ($bundles.Count -eq 0) {
  throw "No Windows release bundles found in $bundleRoot. Run 'bun run tauri build' first."
}

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
$outputPath = Join-Path $outputDir "Shanka_$($version)_$($commit)_manifest.md"

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# $productName $version Release Manifest")
$lines.Add("")
$lines.Add("| Field | Value |")
$lines.Add("| --- | --- |")
$lines.Add("| Generated At | $generatedAt |")
$lines.Add("| Git Commit | $commit |")
$lines.Add("| Product | $productName |")
$lines.Add("| Version | $version |")
$lines.Add("")
$lines.Add("## Bundles")
$lines.Add("")
$lines.Add("| File | Size Bytes | SHA256 |")
$lines.Add("| --- | ---: | --- |")

foreach ($bundle in $bundles) {
  $hash = (Get-FileHash -LiteralPath $bundle.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  $relativePath = Resolve-Path -LiteralPath $bundle.FullName -Relative
  $relativePath = $relativePath.TrimStart(".", "\")
  $lines.Add("| ``$relativePath`` | $($bundle.Length) | ``$hash`` |")
}

$lines.Add("")
$lines.Add("## Verification")
$lines.Add("")
$lines.Add("- Run ``bun run check`` before packaging.")
$lines.Add("- Run ``bun run tauri build`` to regenerate bundles.")
$lines.Add("- Run ``bun run release:smoke`` after packaging.")
$lines.Add("- Run ``bun run release:install-smoke`` on Windows before publishing.")
$lines.Add("- Or run ``bun run release:preflight`` to execute the full local release gate.")
$lines.Add("- Use ``bun run release:manual-text-test`` to launch an isolated manual Windows text-link test profile.")
$lines.Add("- Real installer smoke testing should still be recorded in ``docs/RELEASE_TEST_MATRIX.md``.")

Set-Content -LiteralPath $outputPath -Value $lines -Encoding UTF8
Write-Host "[release-manifest] wrote $outputPath"
