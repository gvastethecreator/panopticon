#Requires -Version 5.1

[CmdletBinding()]
param(
    [Parameter()] [switch] $RequireReservedIdentity,
    [Parameter()] [string] $EvidencePath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$identityPath = Join-Path $repoRoot 'packaging\msix\store-identity.json'
$templatePath = Join-Path $repoRoot 'packaging\msix\AppxManifest.template.xml'
$cargoPath = Join-Path $repoRoot 'Cargo.toml'
$privacyPath = Join-Path $repoRoot 'PRIVACY.md'
$runbookPath = Join-Path $repoRoot 'docs\store\README.md'
$listingPath = Join-Path $repoRoot 'docs\store\LISTING.md'
$distributionPath = Join-Path $repoRoot 'src\app\distribution.rs'
$runtimeSupportPath = Join-Path $repoRoot 'src\app\runtime_support.rs'
$buildAssetsPath = Join-Path $repoRoot 'scripts\Build-StoreAssets.ps1'
$buildPackagePath = Join-Path $repoRoot 'scripts\Build-StoreMsix.ps1'
$setIdentityPath = Join-Path $repoRoot 'scripts\Set-StoreIdentity.ps1'
$iconSourcePath = Join-Path $repoRoot 'assets\icon-xl.png'

$checks = [System.Collections.Generic.List[object]]::new()
$errors = [System.Collections.Generic.List[string]]::new()
$warnings = [System.Collections.Generic.List[string]]::new()

function Get-RelativePath {
    param([Parameter(Mandatory)] [string] $Path)
    $root = [System.IO.Path]::GetFullPath($repoRoot).TrimEnd('\') + '\'
    $full = [System.IO.Path]::GetFullPath($Path)
    if ($full.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) {
        return $full.Substring($root.Length)
    }
    return $full
}

function Add-Check {
    param(
        [Parameter(Mandatory)] [string] $Name,
        [Parameter(Mandatory)] [bool] $Passed,
        [Parameter(Mandatory)] [string] $Details
    )
    $checks.Add([ordered]@{ name = $Name; passed = $Passed; details = $Details })
    if (-not $Passed) {
        $errors.Add("$Name`: $Details")
    }
}

function Add-Warning {
    param([Parameter(Mandatory)] [string] $Message)
    $warnings.Add($Message)
}

function Test-ExactText {
    param(
        [Parameter(Mandatory)] [string] $Name,
        [AllowNull()] [string] $Actual,
        [AllowNull()] [string] $Expected
    )
    Add-Check -Name $Name -Passed ([string]::Equals($Actual, $Expected, [StringComparison]::Ordinal)) -Details "expected '$Expected', actual '$Actual'"
    if ($null -ne $Actual) {
        Add-Check `
            -Name "$Name has no surrounding whitespace" `
            -Passed ([string]::Equals($Actual, $Actual.Trim(), [StringComparison]::Ordinal) -and $Actual.IndexOf([char] 0x00A0) -lt 0) `
            -Details 'value must not contain leading, trailing, or non-breaking whitespace'
    }
}

$requiredFiles = @(
    $identityPath,
    $templatePath,
    $cargoPath,
    $privacyPath,
    $runbookPath,
    $listingPath,
    $distributionPath,
    $runtimeSupportPath,
    $buildAssetsPath,
    $setIdentityPath,
    $iconSourcePath
)

foreach ($requiredFile in $requiredFiles) {
    Add-Check -Name "Required file: $(Get-RelativePath $requiredFile)" -Passed (Test-Path -LiteralPath $requiredFile -PathType Leaf) -Details 'file must exist'
}

# Build-StoreMsix.ps1 is added by this Store publication layer. Treat a missing
# file as a normal validation failure rather than aborting before evidence is written.
Add-Check -Name 'Required file: scripts\Build-StoreMsix.ps1' -Passed (Test-Path -LiteralPath $buildPackagePath -PathType Leaf) -Details 'Store package builder must exist'

if (-not (Test-Path -LiteralPath $identityPath -PathType Leaf) -or -not (Test-Path -LiteralPath $templatePath -PathType Leaf)) {
    throw "Store identity or manifest template is missing.`n - $($errors -join "`n - ")"
}

$identity = Get-Content -LiteralPath $identityPath -Raw | ConvertFrom-Json
[xml] $template = Get-Content -LiteralPath $templatePath -Raw
$templateText = Get-Content -LiteralPath $templatePath -Raw

$ns = [System.Xml.XmlNamespaceManager]::new($template.NameTable)
$ns.AddNamespace('f', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
$ns.AddNamespace('uap', 'http://schemas.microsoft.com/appx/manifest/uap/windows10')
$ns.AddNamespace('uap10', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/10')
$ns.AddNamespace('rescap', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities')

$identityNode = $template.SelectSingleNode('/f:Package/f:Identity', $ns)
$applicationNode = $template.SelectSingleNode('/f:Package/f:Applications/f:Application[@Id="App"]', $ns)
$fullTrustNode = $template.SelectSingleNode('/f:Package/f:Capabilities/rescap:Capability[@Name="runFullTrust"]', $ns)
$targetFamilies = @($template.SelectNodes('/f:Package/f:Dependencies/f:TargetDeviceFamily', $ns))
$resources = @($template.SelectNodes('/f:Package/f:Resources/f:Resource', $ns) | ForEach-Object { $_.GetAttribute('Language') })

Add-Check -Name 'Manifest template Identity' -Passed ($null -ne $identityNode) -Details 'template must contain Package/Identity'
Add-Check -Name 'Manifest template application ID' -Passed ($null -ne $applicationNode) -Details 'Application Id must remain App'
Add-Check -Name 'Manifest template runFullTrust' -Passed ($null -ne $fullTrustNode) -Details 'full-trust desktop operations require a restricted-capability declaration'
Add-Check -Name 'One target device family' -Passed ($targetFamilies.Count -eq 1) -Details "expected one Windows.Desktop target; found $($targetFamilies.Count)"
if ($targetFamilies.Count -gt 0) {
    Add-Check -Name 'Desktop-only package' -Passed ($targetFamilies[0].GetAttribute('Name') -eq 'Windows.Desktop') -Details "actual target: $($targetFamilies[0].GetAttribute('Name'))"
    Add-Check -Name 'Minimum Windows version' -Passed ($targetFamilies[0].GetAttribute('MinVersion') -eq '10.0.17763.0') -Details "expected 10.0.17763.0; actual $($targetFamilies[0].GetAttribute('MinVersion'))"
}

if ($identityNode) {
    Test-ExactText -Name 'Identity name placeholder' -Actual $identityNode.GetAttribute('Name') -Expected '__PACKAGE_NAME__'
    Test-ExactText -Name 'Publisher placeholder' -Actual $identityNode.GetAttribute('Publisher') -Expected '__PUBLISHER__'
    Test-ExactText -Name 'Version placeholder' -Actual $identityNode.GetAttribute('Version') -Expected '__PACKAGE_VERSION__'
    Test-ExactText -Name 'Package architecture' -Actual $identityNode.GetAttribute('ProcessorArchitecture') -Expected 'x64'
}

if ($applicationNode) {
    Test-ExactText -Name 'Packaged executable' -Actual $applicationNode.GetAttribute('Executable') -Expected 'panopticon.exe'
    Test-ExactText -Name 'Desktop entry point' -Actual $applicationNode.GetAttribute('EntryPoint') -Expected 'Windows.FullTrustApplication'
    Test-ExactText -Name 'Runtime behavior' -Actual $applicationNode.GetAttribute('RuntimeBehavior', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/10') -Expected 'packagedClassicApp'
    Test-ExactText -Name 'Trust level' -Actual $applicationNode.GetAttribute('TrustLevel', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/10') -Expected 'mediumIL'
}

foreach ($language in @('en-US', 'es-ES')) {
    Add-Check -Name "Resource language $language" -Passed ($resources -contains $language) -Details "declared languages: $($resources -join ', ')"
}

foreach ($placeholder in @('__PACKAGE_NAME__', '__PUBLISHER__', '__PACKAGE_VERSION__', '__PUBLISHER_DISPLAY_NAME__')) {
    Add-Check -Name "Manifest placeholder $placeholder" -Passed ($templateText.IndexOf($placeholder, [StringComparison]::Ordinal) -ge 0) -Details 'all identity placeholders must remain available for deterministic rendering'
}

$status = [string] $identity.reservationStatus
Add-Check -Name 'Known reservation status' -Passed ($status -in @('pending', 'reserved')) -Details "expected pending or reserved; actual '$status'"

if ($status -eq 'reserved') {
    foreach ($field in @(
        @{ Name = 'Reserved identity name'; Value = [string] $identity.packageIdentity.name },
        @{ Name = 'Reserved publisher'; Value = [string] $identity.packageIdentity.publisher },
        @{ Name = 'Reserved publisher display name'; Value = [string] $identity.packageIdentity.publisherDisplayName },
        @{ Name = 'Reserved PFN'; Value = [string] $identity.packageIdentity.packageFamilyName },
        @{ Name = 'Reserved Package SID'; Value = [string] $identity.packageIdentity.packageSid },
        @{ Name = 'Reserved Store ID'; Value = [string] $identity.store.productId }
    )) {
        Add-Check -Name $field.Name -Passed (-not [string]::IsNullOrWhiteSpace($field.Value)) -Details 'reserved values must be populated'
        if (-not [string]::IsNullOrWhiteSpace($field.Value)) {
            Add-Check -Name "$($field.Name) whitespace" -Passed ([string]::Equals($field.Value, $field.Value.Trim(), [StringComparison]::Ordinal) -and $field.Value.IndexOf([char] 0x00A0) -lt 0) -Details 'reserved values must not contain hidden/surrounding whitespace'
        }
    }

    Add-Check -Name 'PFN remains metadata-only' -Passed ($templateText.IndexOf([string] $identity.packageIdentity.packageFamilyName, [StringComparison]::Ordinal) -lt 0) -Details 'PFN must not be written into the manifest template'
    Add-Check -Name 'Package SID remains metadata-only' -Passed ($templateText.IndexOf([string] $identity.packageIdentity.packageSid, [StringComparison]::Ordinal) -lt 0) -Details 'Package SID must not be written into the manifest template'

    $privacy = Get-Content -LiteralPath $privacyPath -Raw
    Add-Check -Name 'Privacy publisher finalized' -Passed ($privacy.IndexOf('To be completed with the verified Microsoft Store publisher name', [StringComparison]::OrdinalIgnoreCase) -lt 0 -and $privacy.IndexOf('debe completarse con el nombre verificado', [StringComparison]::OrdinalIgnoreCase) -lt 0) -Details 'run Set-StoreIdentity.ps1 to finalize the privacy publisher'
}
else {
    Add-Warning 'Partner Center identity is pending. Structural validation can pass, but no submission package may be built as final.'
    if ($RequireReservedIdentity) {
        Add-Check -Name 'Reserved Partner Center identity required' -Passed $false -Details 'reserve Panopticon and run Set-StoreIdentity.ps1 before packaging'
    }
}

$cargoText = Get-Content -LiteralPath $cargoPath -Raw
$cargoVersion = $null
if ($cargoText -match '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"') {
    $cargoVersion = $Matches.version
}
Add-Check -Name 'Cargo semantic version' -Passed ($null -ne $cargoVersion) -Details "could not parse major.minor.patch from Cargo.toml"

$distributionText = Get-Content -LiteralPath $distributionPath -Raw
$runtimeText = Get-Content -LiteralPath $runtimeSupportPath -Raw
Add-Check -Name 'Store distribution aliases' -Passed ($distributionText.IndexOf('updates_managed_by_store', [StringComparison]::Ordinal) -ge 0 -and $distributionText.IndexOf('microsoft-store', [StringComparison]::OrdinalIgnoreCase) -ge 0) -Details 'compile-time distribution policy must recognize Store builds'
Add-Check -Name 'Runtime Store update guard' -Passed ($runtimeText.IndexOf('updates_managed_by_store()', [StringComparison]::Ordinal) -ge 0 -and $runtimeText.IndexOf('skipped GitHub release check', [StringComparison]::Ordinal) -ge 0) -Details 'runtime update request must short-circuit for Store builds'

$result = [ordered]@{
    schema = 'panopticon.store-readiness.v1'
    generatedAt = [DateTimeOffset]::UtcNow.ToString('O')
    repository = 'gvastethecreator/panopticon'
    reservationStatus = $status
    cargoVersion = $cargoVersion
    expectedMsixVersion = $(if ($cargoVersion) { "$cargoVersion.0" } else { $null })
    passed = $errors.Count -eq 0
    warnings = $warnings
    checks = $checks
}

if ($EvidencePath) {
    $resolved = if ([System.IO.Path]::IsPathRooted($EvidencePath)) { $EvidencePath } else { Join-Path $repoRoot $EvidencePath }
    $directory = Split-Path -Parent $resolved
    if ($directory) {
        New-Item -ItemType Directory -Force -Path $directory | Out-Null
    }
    $result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $resolved -Encoding utf8
    Write-Host "Evidence: $resolved" -ForegroundColor DarkGray
}

foreach ($warning in $warnings) {
    Write-Host "WARNING: $warning" -ForegroundColor Yellow
}

if ($errors.Count -gt 0) {
    Write-Host ''
    Write-Host 'STORE READINESS FAILED' -ForegroundColor Red
    foreach ($message in $errors) {
        Write-Host " - $message" -ForegroundColor Red
    }
    exit 1
}

Write-Host ''
Write-Host "STORE READINESS PASSED ($($checks.Count) checks)" -ForegroundColor Green
Write-Host "Reservation status: $status"
Write-Host "Cargo version: $cargoVersion"
