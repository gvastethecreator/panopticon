#Requires -Version 5.1
<#
.SYNOPSIS
    Bump or set the Panopticon version, commit, tag, and push to trigger the GitHub release workflow.

.PARAMETER Bump
    Which component to increment:
      patch  → 0.1.x  (bug fixes)
      minor  → 0.x.0  (new features, backwards-compatible)
      major  → x.0.0  (breaking changes)

.PARAMETER Version
    Set an explicit target version instead of calculating the next patch/minor/major value.

.EXAMPLE
    .\scripts\bump-version.ps1 -Bump patch
    .\scripts\bump-version.ps1 -Bump minor

.EXAMPLE
    .\scripts\bump-version.ps1 -Version 0.1.21
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory, ParameterSetName = "Bump")]
    [ValidateSet("patch", "minor", "major")]
    [string]$Bump,

    [Parameter(Mandatory, ParameterSetName = "Version")]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── 1. Guard: working tree must be clean ─────────────────────────────────────
$dirty = git status --porcelain 2>&1
if ($dirty) {
    Write-Error "Working tree is not clean. Commit or stash your changes first.`n$dirty"
    exit 1
}

# ── 2. Parse current version from Cargo.toml ─────────────────────────────────
$cargoPath = Join-Path $PSScriptRoot ".." "Cargo.toml"
$cargo = Get-Content $cargoPath -Raw
if ($cargo -notmatch '(?m)^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"') {
    Write-Error "Could not parse version from Cargo.toml"
    exit 1
}
[int]$maj = $Matches[1]
[int]$min = $Matches[2]
[int]$pat = $Matches[3]
$oldVersion = "$maj.$min.$pat"

# ── 3. Compute next version ───────────────────────────────────────────────────
if ($PSCmdlet.ParameterSetName -eq "Version") {
    $newVersion = $Version
} else {
    switch ($Bump) {
        "major" { $maj++; $min = 0; $pat = 0 }
        "minor" { $min++; $pat = 0 }
        "patch" { $pat++ }
    }
    $newVersion = "$maj.$min.$pat"
}

if ([version]$newVersion -le [version]$oldVersion) {
    Write-Error "New version '$newVersion' must be greater than current version '$oldVersion'"
    exit 1
}

$newTag     = "v$newVersion"

Write-Host ""
if ($PSCmdlet.ParameterSetName -eq "Version") {
    Write-Host "  $oldVersion  =>  $newVersion  (explicit version)"
} else {
    Write-Host "  $oldVersion  =>  $newVersion  ($Bump bump)"
}
Write-Host ""

# ── 4. Update Cargo.toml (package version only, not dependency versions) ─────
# The pattern '^version = "..."' only matches a bare line — never an inline table.
$updated = ([regex]'(?m)^(version\s*=\s*")\d+\.\d+\.\d+"').Replace(
    $cargo,
    {
        param($match)
        $match.Groups[1].Value + $newVersion + '"'
    },
    1
)
Set-Content $cargoPath $updated -NoNewline

# ── 4b. Keep the PRD version in sync ──────────────────────────────────────────
$prdPath = Join-Path $PSScriptRoot ".." "docs" "PRD.md"
$prdContent = Get-Content $prdPath -Raw
$prdUpdated = ([regex]'(?m)^(\*\*Documented product version:\*\*\s*)\d+\.\d+\.\d+').Replace(
    $prdContent,
    {
        param($match)
        $match.Groups[1].Value + $newVersion
    },
    1
)
Set-Content $prdPath $prdUpdated -NoNewline

# ── 4c. Guard: changelog must already contain the target entry ────────────────
$changelogPath = Join-Path $PSScriptRoot ".." "CHANGELOG.md"
$changelog = Get-Content $changelogPath -Raw
$escapedVersion = [regex]::Escape($newVersion)
if ($changelog -notmatch "(?m)^## \[$escapedVersion\]\s+-\s+\d{4}-\d{2}-\d{2}$") {
    Write-Error "CHANGELOG.md must contain an entry like '## [$newVersion] - YYYY-MM-DD' before releasing"
    exit 1
}

# ── 5. Sync Cargo.lock ────────────────────────────────────────────────────────
Write-Host "Updating Cargo.lock ..."
cargo check --quiet
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# ── 6. Commit + tag ───────────────────────────────────────────────────────────
git add Cargo.toml Cargo.lock CHANGELOG.md docs/PRD.md
git commit -m "chore: release $newTag"
git tag $newTag

# ── 7. Push commit + tag ──────────────────────────────────────────────────────
Write-Host "Pushing commit and tag $newTag ..."
git push
git push origin $newTag

Write-Host ""
Write-Host "Done. GitHub Actions will build and publish $newTag."
