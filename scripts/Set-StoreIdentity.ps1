#Requires -Version 5.1

[CmdletBinding(SupportsShouldProcess, ConfirmImpact = 'Medium')]
param(
    [Parameter(Mandatory)] [string] $Name,
    [Parameter(Mandatory)] [string] $Publisher,
    [Parameter(Mandatory)] [string] $PublisherDisplayName,
    [Parameter(Mandatory)] [string] $PackageFamilyName,
    [Parameter(Mandatory)] [string] $PackageSid,
    [Parameter(Mandatory)] [string] $StoreId
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$identityPath = Join-Path $repoRoot 'packaging\msix\store-identity.json'
$privacyPath = Join-Path $repoRoot 'PRIVACY.md'
$readinessScript = Join-Path $PSScriptRoot 'Test-StoreReadiness.ps1'

function Assert-ExactInput {
    param(
        [Parameter(Mandatory)] [string] $Field,
        [AllowEmptyString()] [string] $Value
    )

    if ([string]::IsNullOrWhiteSpace($Value)) {
        throw "$Field is required."
    }
    if (-not [string]::Equals($Value, $Value.Trim(), [StringComparison]::Ordinal)) {
        throw "$Field contains leading, trailing, or hidden whitespace. Copy the exact Partner Center value without surrounding spaces."
    }
    if ($Value.IndexOf([char] 0x00A0) -ge 0) {
        throw "$Field contains a non-breaking space. Remove it before continuing."
    }
}

foreach ($item in @(
    @{ Field = 'Package/Identity/Name'; Value = $Name },
    @{ Field = 'Package/Identity/Publisher'; Value = $Publisher },
    @{ Field = 'PublisherDisplayName'; Value = $PublisherDisplayName },
    @{ Field = 'Package Family Name'; Value = $PackageFamilyName },
    @{ Field = 'Package SID'; Value = $PackageSid },
    @{ Field = 'Store ID'; Value = $StoreId }
)) {
    Assert-ExactInput -Field $item.Field -Value $item.Value
}

if (-not $Publisher.StartsWith('CN=', [StringComparison]::Ordinal)) {
    throw "Publisher must be the exact Partner Center distinguished name and normally starts with CN=: $Publisher"
}
if ($PackageSid -notmatch '^S-1-15-2-(?:\d+-){6}\d+$') {
    throw "Package SID does not match the expected app-container SID form: $PackageSid"
}
if ($StoreId -notmatch '^[A-Z0-9]{12}$') {
    throw "Store ID must contain 12 uppercase letters or digits: $StoreId"
}

$identity = Get-Content -LiteralPath $identityPath -Raw | ConvertFrom-Json
$privacy = Get-Content -LiteralPath $privacyPath -Raw

$identity.reservationStatus = 'reserved'
$identity.packageIdentity.name = $Name
$identity.packageIdentity.publisher = $Publisher
$identity.packageIdentity.publisherDisplayName = $PublisherDisplayName
$identity.packageIdentity.packageFamilyName = $PackageFamilyName
$identity.packageIdentity.packageSid = $PackageSid
$identity.store.productId = $StoreId
$identity.store.deepLink = $null
$identity.store.webStoreUrl = $null

$privacy = $privacy.Replace(
    '**Publisher:** To be completed with the verified Microsoft Store publisher name before submission',
    "**Publisher:** $PublisherDisplayName"
)
$privacy = $privacy.Replace(
    '**Publicador:** debe completarse con el nombre verificado de Microsoft Store antes de la submission',
    "**Publicador:** $PublisherDisplayName"
)

if (-not $PSCmdlet.ShouldProcess('Panopticon Store identity metadata and privacy publisher', 'Apply Partner Center identity')) {
    return
}

$identityTemp = "$identityPath.tmp"
$privacyTemp = "$privacyPath.tmp"
try {
    $identity | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $identityTemp -Encoding utf8
    [System.IO.File]::WriteAllText($privacyTemp, $privacy, [System.Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $identityTemp -Destination $identityPath -Force
    Move-Item -LiteralPath $privacyTemp -Destination $privacyPath -Force
}
finally {
    foreach ($tempPath in @($identityTemp, $privacyTemp)) {
        if (Test-Path -LiteralPath $tempPath) {
            Remove-Item -LiteralPath $tempPath -Force
        }
    }
}

Write-Host 'Panopticon Store identity applied.' -ForegroundColor Green
Write-Host "Name: $Name"
Write-Host "Publisher: $Publisher"
Write-Host "Publisher display name: $PublisherDisplayName"
Write-Host "PFN (verification only): $PackageFamilyName"
Write-Host "Package SID (verification only): $PackageSid"
Write-Host "Store ID: $StoreId"

& $readinessScript -RequireReservedIdentity
if ($LASTEXITCODE -ne 0) {
    throw 'Store identity was written, but readiness validation failed.'
}
