#Requires -Version 5.1
<#
.SYNOPSIS
    Bump the Panopticon version, commit, tag, and push to trigger the GitHub release workflow.

.PARAMETER Bump
    Which component to increment:
      patch  → 0.1.x  (bug fixes)
      minor  → 0.x.0  (new features, backwards-compatible)
      major  → x.0.0  (breaking changes)

.EXAMPLE
    .\scripts\bump-version.ps1 -Bump patch
    .\scripts\bump-version.ps1 -Bump minor
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet("patch", "minor", "major")]
    [string]$Bump
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
switch ($Bump) {
    "major" { $maj++; $min = 0; $pat = 0 }
    "minor" { $min++; $pat = 0 }
    "patch" { $pat++ }
}
$newVersion = "$maj.$min.$pat"
$newTag     = "v$newVersion"

Write-Host ""
Write-Host "  $oldVersion  =>  $newVersion  ($Bump bump)"
Write-Host ""

# ── 4. Update Cargo.toml (package version only, not dependency versions) ─────
# The pattern '^version = "..."' only matches a bare line — never an inline table.
$updated = $cargo -replace '(?m)^(version\s*=\s*")\d+\.\d+\.\d+"', "`${1}$newVersion`""
Set-Content $cargoPath $updated -NoNewline

# ── 5. Sync Cargo.lock ────────────────────────────────────────────────────────
Write-Host "Updating Cargo.lock ..."
cargo check --quiet
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# ── 6. Commit + tag ───────────────────────────────────────────────────────────
git add Cargo.toml Cargo.lock
git commit -m "chore: release $newTag"
git tag $newTag

# ── 7. Push commit + tag ──────────────────────────────────────────────────────
Write-Host "Pushing commit and tag $newTag ..."
git push
git push origin $newTag

Write-Host ""
Write-Host "Done. GitHub Actions will build and publish $newTag."
