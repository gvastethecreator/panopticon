#Requires -Version 5.1

[CmdletBinding()]
param(
    [Parameter()]
    [ValidateSet('x64')]
    [string] $Platform = 'x64',

    [Parameter()]
    [switch] $SkipTests,

    [Parameter()]
    [string] $PackageCertificateKeyFile = $env:PANOPTICON_STORE_CERTIFICATE_PATH,

    [Parameter()]
    [string] $PackageCertificatePassword = $env:PANOPTICON_STORE_CERTIFICATE_PASSWORD,

    [Parameter()]
    [string] $OutputDirectory,

    [Parameter()]
    [string] $MakeAppxPath,

    [Parameter()]
    [string] $SignToolPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$identityPath = Join-Path $repoRoot 'packaging\msix\store-identity.json'
$templatePath = Join-Path $repoRoot 'packaging\msix\AppxManifest.template.xml'
$cargoPath = Join-Path $repoRoot 'Cargo.toml'
$readinessScript = Join-Path $PSScriptRoot 'Test-StoreReadiness.ps1'
$assetScript = Join-Path $PSScriptRoot 'Build-StoreAssets.ps1'

if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repoRoot 'artifacts\store\x64'
}
elseif (-not [System.IO.Path]::IsPathRooted($OutputDirectory)) {
    $OutputDirectory = Join-Path $repoRoot $OutputDirectory
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)

$storeRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'artifacts\store'))
$workRoot = Join-Path $storeRoot 'work\x64'
$stagingRoot = Join-Path $workRoot 'staging'
$cargoTarget = Join-Path $workRoot 'cargo-target'

function Invoke-External {
    param(
        [Parameter(Mandatory)] [string] $Name,
        [Parameter(Mandatory)] [string] $FilePath,
        [Parameter(Mandatory)] [string[]] $Arguments
    )

    Write-Host "==> $Name" -ForegroundColor Cyan
    $global:LASTEXITCODE = 0
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE."
    }
}

function Assert-SafeStorePath {
    param([Parameter(Mandatory)] [string] $Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $rootPrefix = $storeRoot.TrimEnd('\') + '\'
    if (-not $fullPath.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to modify a path outside artifacts/store: $fullPath"
    }
}

function Reset-Directory {
    param([Parameter(Mandatory)] [string] $Path)

    Assert-SafeStorePath -Path $Path
    if (Test-Path -LiteralPath $Path) {
        [System.IO.Directory]::Delete($Path, $true)
    }
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Resolve-WindowsSdkTool {
    param(
        [Parameter(Mandatory)] [string] $ToolName,
        [AllowNull()] [string] $ExplicitPath
    )

    if ($ExplicitPath) {
        return (Resolve-Path -LiteralPath $ExplicitPath).Path
    }

    $kitsRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (-not (Test-Path -LiteralPath $kitsRoot -PathType Container)) {
        throw "Windows SDK bin directory was not found: $kitsRoot"
    }

    $versions = Get-ChildItem -LiteralPath $kitsRoot -Directory |
        Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
        Sort-Object Name -Descending

    foreach ($version in $versions) {
        foreach ($architecture in @('x64', 'x86')) {
            $candidate = Join-Path $version.FullName "$architecture\$ToolName.exe"
            if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                return $candidate
            }
        }
    }

    throw "$ToolName.exe was not found in the installed Windows SDK."
}

function New-RandomPassword {
    $bytes = New-Object byte[] 32
    $generator = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    try {
        $generator.GetBytes($bytes)
    }
    finally {
        $generator.Dispose()
    }
    return [Convert]::ToBase64String($bytes)
}

function New-TemporaryStoreCertificate {
    param([Parameter(Mandatory)] [string] $Publisher)

    $password = New-RandomPassword
    $securePassword = ConvertTo-SecureString -String $password -AsPlainText -Force
    $temporaryRoot = if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
        [System.IO.Path]::GetTempPath()
    }
    else {
        $env:RUNNER_TEMP
    }
    $pfxPath = Join-Path $temporaryRoot "Panopticon-store-build-$([Guid]::NewGuid().ToString('N')).pfx"

    $certificate = New-SelfSignedCertificate `
        -Type Custom `
        -Subject $Publisher `
        -FriendlyName 'Panopticon Store build certificate (temporary)' `
        -KeyAlgorithm RSA `
        -KeyLength 2048 `
        -HashAlgorithm SHA256 `
        -KeyUsage DigitalSignature `
        -KeyExportPolicy Exportable `
        -CertStoreLocation 'Cert:\CurrentUser\My' `
        -NotAfter (Get-Date).AddDays(7) `
        -TextExtension '2.5.29.37={text}1.3.6.1.5.5.7.3.3'

    Export-PfxCertificate `
        -Cert $certificate `
        -FilePath $pfxPath `
        -Password $securePassword `
        -Force | Out-Null

    return [ordered]@{
        path = $pfxPath
        password = $password
        thumbprint = $certificate.Thumbprint
        generated = $true
    }
}

function Escape-XmlValue {
    param([Parameter(Mandatory)] [string] $Value)
    return [System.Security.SecurityElement]::Escape($Value)
}

& $readinessScript -RequireReservedIdentity
if ($LASTEXITCODE -ne 0) {
    throw 'Reserved Partner Center identity and Store readiness are required.'
}

$identity = Get-Content -LiteralPath $identityPath -Raw | ConvertFrom-Json
$cargoText = Get-Content -LiteralPath $cargoPath -Raw
if ($cargoText -notmatch '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"') {
    throw 'Could not parse the package version from Cargo.toml.'
}
$cargoVersion = $Matches.version
$packageVersion = "$cargoVersion.0"

$packageName = [string] $identity.packageIdentity.name
$publisher = [string] $identity.packageIdentity.publisher
$publisherDisplayName = [string] $identity.packageIdentity.publisherDisplayName
$storeId = [string] $identity.store.productId

$certificateState = $null
$temporaryCertificate = $false
$previousChannel = $env:PANOPTICON_DISTRIBUTION_CHANNEL
try {
    if ($PackageCertificateKeyFile) {
        $resolvedPfx = (Resolve-Path -LiteralPath $PackageCertificateKeyFile).Path
        if (-not $PackageCertificatePassword) {
            throw 'PANOPTICON_STORE_CERTIFICATE_PASSWORD or -PackageCertificatePassword is required with a supplied PFX.'
        }

        $certificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
            $resolvedPfx,
            $PackageCertificatePassword
        )
        try {
            if (-not [string]::Equals($certificate.Subject, $publisher, [StringComparison]::Ordinal)) {
                throw "Certificate subject '$($certificate.Subject)' does not match manifest publisher '$publisher'."
            }
            $certificateState = [ordered]@{
                path = $resolvedPfx
                password = $PackageCertificatePassword
                thumbprint = $certificate.Thumbprint
                generated = $false
            }
        }
        finally {
            $certificate.Dispose()
        }
    }
    else {
        Write-Host 'Generating a short-lived certificate for Store package construction.' -ForegroundColor Yellow
        Write-Host 'Microsoft Store replaces the MSIX signature after certification; this certificate is not for public distribution.' -ForegroundColor Yellow
        $certificateState = New-TemporaryStoreCertificate -Publisher $publisher
        $temporaryCertificate = $true
    }

    Reset-Directory -Path $OutputDirectory
    Reset-Directory -Path $workRoot
    New-Item -ItemType Directory -Force -Path $stagingRoot | Out-Null

    $env:PANOPTICON_DISTRIBUTION_CHANNEL = 'store'

    Push-Location $repoRoot
    try {
        if (-not $SkipTests) {
            Invoke-External -Name 'Cargo format check' -FilePath 'cargo' -Arguments @('fmt', '--', '--check')
            Invoke-External -Name 'Cargo clippy' -FilePath 'cargo' -Arguments @(
                'clippy',
                '--all-targets',
                '--locked',
                '--target-dir', $cargoTarget,
                '--',
                '-D', 'warnings',
                '-W', 'clippy::pedantic'
            )
            Invoke-External -Name 'Cargo tests' -FilePath 'cargo' -Arguments @(
                'test',
                '--all-targets',
                '--locked',
                '--target-dir', $cargoTarget
            )
            if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
                Invoke-External -Name 'Cargo dependency audit' -FilePath 'cargo' -Arguments @('audit')
            }
            else {
                Write-Host 'cargo-audit is not installed; dependency audit remains a CI/release gate.' -ForegroundColor Yellow
            }
        }

        Invoke-External -Name 'Build Store-channel release executable' -FilePath 'cargo' -Arguments @(
            'build',
            '--release',
            '--locked',
            '--target-dir', $cargoTarget
        )
    }
    finally {
        Pop-Location
    }

    $executablePath = Join-Path $cargoTarget 'release\panopticon.exe'
    if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf)) {
        throw "Store-channel executable was not produced: $executablePath"
    }

    Copy-Item -LiteralPath $executablePath -Destination (Join-Path $stagingRoot 'panopticon.exe')
    Copy-Item -LiteralPath (Join-Path $repoRoot 'README.md') -Destination (Join-Path $stagingRoot 'README.md')
    Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE') -Destination (Join-Path $stagingRoot 'LICENSE')

    $assetsPath = Join-Path $stagingRoot 'Assets'
    & $assetScript -OutputDirectory $assetsPath
    if ($LASTEXITCODE -ne 0) {
        throw 'Store asset generation failed.'
    }

    $manifestText = Get-Content -LiteralPath $templatePath -Raw
    $manifestText = $manifestText.Replace('__PACKAGE_NAME__', (Escape-XmlValue $packageName))
    $manifestText = $manifestText.Replace('__PUBLISHER__', (Escape-XmlValue $publisher))
    $manifestText = $manifestText.Replace('__PACKAGE_VERSION__', (Escape-XmlValue $packageVersion))
    $manifestText = $manifestText.Replace('__PUBLISHER_DISPLAY_NAME__', (Escape-XmlValue $publisherDisplayName))

    if ($manifestText -match '__[A-Z0-9_]+__') {
        throw 'Rendered AppxManifest.xml still contains an unresolved placeholder.'
    }

    $manifestPath = Join-Path $stagingRoot 'AppxManifest.xml'
    [System.IO.File]::WriteAllText($manifestPath, $manifestText, [System.Text.UTF8Encoding]::new($false))
    [xml] $renderedManifest = $manifestText

    $resolvedMakeAppx = Resolve-WindowsSdkTool -ToolName 'MakeAppx' -ExplicitPath $MakeAppxPath
    $resolvedSignTool = Resolve-WindowsSdkTool -ToolName 'SignTool' -ExplicitPath $SignToolPath

    $packagePath = Join-Path $OutputDirectory "Panopticon-$cargoVersion-windows-x64-store.msix"
    Invoke-External -Name 'Pack MSIX' -FilePath $resolvedMakeAppx -Arguments @(
        'pack',
        '/d', $stagingRoot,
        '/p', $packagePath,
        '/o'
    )

    Invoke-External -Name 'Sign MSIX for build validation' -FilePath $resolvedSignTool -Arguments @(
        'sign',
        '/fd', 'SHA256',
        '/f', $certificateState.path,
        '/p', $certificateState.password,
        $packagePath
    )

    if (-not (Test-Path -LiteralPath $packagePath -PathType Leaf)) {
        throw "MSIX package was not created: $packagePath"
    }

    $signature = Get-AuthenticodeSignature -LiteralPath $packagePath
    if ($null -eq $signature.SignerCertificate) {
        throw 'The generated MSIX does not contain a signer certificate.'
    }
    if ($signature.Status -in @(
        [System.Management.Automation.SignatureStatus]::NotSigned,
        [System.Management.Automation.SignatureStatus]::HashMismatch
    )) {
        throw "The generated MSIX signature is unusable: $($signature.Status) $($signature.StatusMessage)"
    }
    if (-not [string]::Equals($signature.SignerCertificate.Subject, $publisher, [StringComparison]::Ordinal)) {
        throw "MSIX signer '$($signature.SignerCertificate.Subject)' does not match publisher '$publisher'."
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($packagePath)
    try {
        $entries = @($archive.Entries | ForEach-Object { $_.FullName.Replace('/', '\') })
    }
    finally {
        $archive.Dispose()
    }

    $requiredEntries = @(
        'AppxManifest.xml',
        'AppxBlockMap.xml',
        'AppxSignature.p7x',
        'panopticon.exe',
        'LICENSE',
        'README.md',
        'Assets\StoreLogo.png',
        'Assets\Square44x44Logo.png',
        'Assets\Square150x150Logo.png',
        'Assets\Wide310x150Logo.png',
        'Assets\SplashScreen.png'
    )
    $missingEntries = @($requiredEntries | Where-Object { $entries -notcontains $_ })
    if ($missingEntries.Count -gt 0) {
        throw "The generated MSIX is missing required entries: $($missingEntries -join ', ')"
    }

    $sourceCommit = (& git -C $repoRoot rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to resolve the source commit.'
    }

    $packageItem = Get-Item -LiteralPath $packagePath
    $evidence = [ordered]@{
        schema = 'panopticon.store-build.v1'
        generatedAt = [DateTimeOffset]::UtcNow.ToString('O')
        sourceCommit = $sourceCommit
        cargoVersion = $cargoVersion
        packageVersion = $packageVersion
        packageName = $packageName
        publisher = $publisher
        publisherDisplayName = $publisherDisplayName
        storeId = $storeId
        architecture = $Platform
        distributionChannel = 'store'
        targetDeviceFamily = 'Windows.Desktop'
        minimumWindowsVersion = '10.0.17763.0'
        artifact = [ordered]@{
            name = $packageItem.Name
            path = $packageItem.FullName
            bytes = $packageItem.Length
            sha256 = (Get-FileHash -LiteralPath $packageItem.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
        executable = [ordered]@{
            name = 'panopticon.exe'
            sha256 = (Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash.ToLowerInvariant()
        }
        buildCertificate = [ordered]@{
            subject = $signature.SignerCertificate.Subject
            thumbprint = $signature.SignerCertificate.Thumbprint
            signatureStatus = $signature.Status.ToString()
            temporary = $certificateState.generated
            note = 'Build/test signature only. Microsoft Store replaces MSIX/AppX signatures after certification.'
        }
        requiredEntries = $requiredEntries
    }

    $evidencePath = Join-Path $OutputDirectory 'store-build-manifest.json'
    $evidence | ConvertTo-Json -Depth 7 | Set-Content -LiteralPath $evidencePath -Encoding utf8

    $checksumsPath = Join-Path $OutputDirectory 'SHA256SUMS.txt'
    @($packagePath, $evidencePath) | ForEach-Object {
        $item = Get-Item -LiteralPath $_
        $hash = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $($item.Name)"
    } | Set-Content -LiteralPath $checksumsPath -Encoding ascii

    Write-Host ''
    Write-Host 'PANOPTICON STORE MSIX READY FOR REVIEW' -ForegroundColor Green
    Write-Host "Package: $packagePath"
    Write-Host "SHA-256: $($evidence.artifact.sha256)"
    Write-Host "Evidence: $evidencePath"
    Write-Host 'Do not upload until clean-machine lifecycle, DWM, shell, privacy, listing, and Partner Center gates are complete.' -ForegroundColor Yellow
}
finally {
    if ($null -eq $previousChannel) {
        Remove-Item Env:PANOPTICON_DISTRIBUTION_CHANNEL -ErrorAction SilentlyContinue
    }
    else {
        $env:PANOPTICON_DISTRIBUTION_CHANNEL = $previousChannel
    }

    if (Test-Path -LiteralPath $workRoot) {
        Assert-SafeStorePath -Path $workRoot
        [System.IO.Directory]::Delete($workRoot, $true)
    }

    if ($temporaryCertificate -and $certificateState) {
        if (Test-Path -LiteralPath $certificateState.path) {
            Remove-Item -LiteralPath $certificateState.path -Force
        }
        if ($certificateState.thumbprint) {
            $storePath = "Cert:\CurrentUser\My\$($certificateState.thumbprint)"
            if (Test-Path -LiteralPath $storePath) {
                Remove-Item -LiteralPath $storePath -Force
            }
        }
    }
}
