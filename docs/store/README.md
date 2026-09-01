# Panopticon Microsoft Store submission runbook

This directory is the source of truth for publishing Panopticon through Microsoft Store as a packaged full-trust desktop application.

## Distribution model

Panopticon has two intentionally separate Windows channels:

```text
Microsoft Store
  -> reserved Partner Center identity
  -> x64 MSIX package
  -> PANOPTICON_DISTRIBUTION_CHANNEL=store
  -> Microsoft Store-managed signing and updates

GitHub/direct
  -> portable ZIP and Inno Setup installer
  -> PANOPTICON_DISTRIBUTION_CHANNEL=direct (default)
  -> GitHub Releases update check
  -> publisher-owned Authenticode signing and timestamping
```

The Store package must not run the GitHub Releases update checker. The direct build keeps its current bounded update request.

Microsoft Store accepts raw `.msix` packages. Projects with a Visual Studio packaging project should normally prefer `.msixupload`. Panopticon is a Rust/Slint desktop application without a WAP project, so this repository builds and validates a raw x64 `.msix`.

## Current reservation status

The product has not yet been associated with a Partner Center identity. Machine-readable state lives in:

```text
packaging/msix/store-identity.json
```

Reserve Panopticon in Partner Center, then apply the exact values:

```powershell
.\scripts\Set-StoreIdentity.ps1 `
  -Name '<Package/Identity/Name>' `
  -Publisher '<Package/Identity/Publisher>' `
  -PublisherDisplayName '<Package/Properties/PublisherDisplayName>' `
  -StoreId '<Store ID>' `
  -PackageFamilyName '<PFN>' `
  -PackageSid '<Package SID>'
```

The script rejects leading, trailing, and non-breaking whitespace. It stores PFN and Package SID only for verification; neither derived value belongs in the manifest.

## Package implementation

The package source consists of:

```text
packaging/msix/AppxManifest.template.xml
packaging/msix/store-identity.json
scripts/Build-StoreAssets.ps1
scripts/Build-StoreMsix.ps1
scripts/Set-StoreIdentity.ps1
scripts/Test-StoreReadiness.ps1
```

Build flow:

```text
reserved identity
  -> validate Store contract
  -> generate Store PNG assets from assets/icon-xl.png
  -> compile release with PANOPTICON_DISTRIBUTION_CHANNEL=store
  -> stage panopticon.exe + LICENSE + README + Assets
  -> render AppxManifest.xml
  -> MakeAppx pack
  -> temporary matching certificate
  -> SignTool validation signature
  -> SHA-256 + evidence JSON
```

The temporary certificate exists only to produce and test a correctly signed MSIX before upload. Microsoft Store replaces the package signature after certification. It is not valid as a public direct-distribution certificate.

## Commands

Structural validation, allowed while identity is pending:

```powershell
.\scripts\Test-StoreReadiness.ps1
```

Strict validation before packaging:

```powershell
.\scripts\Test-StoreReadiness.ps1 -RequireReservedIdentity
```

Build the x64 Store candidate:

```powershell
.\scripts\Build-StoreMsix.ps1
```

The build output is written under:

```text
artifacts\store\x64
```

## Partner Center submission

### 1. Pricing and availability

Review every field instead of accepting defaults without consideration:

- Markets: choose where the app and support material can be offered.
- Audience: use Public only after first-release qualification.
- Discoverability: choose searchable or direct-link-only intentionally.
- Schedule: use a manual publishing hold for the first submission.
- Base price: choose Free unless a separate commercial decision exists.

A publishing hold lets certification complete before the listing becomes visible.

### 2. Properties

Recommended initial values:

- Primary category: Productivity or Utilities & tools, according to the current Partner Center taxonomy.
- Privacy policy URL: stable public rendering of [`../../PRIVACY.md`](../../PRIVACY.md).
- Website: public project/product page.
- Support: issue tracker, support page, or monitored email.
- Minimum OS: Windows 10 version 1809.
- Architecture: x64 for the first Store release.
- Display/XR options: disabled.

Panopticon accesses window titles, application/process metadata, monitor geometry, live DWM previews, and local settings. Publish a privacy policy even though the app is local-first.

### 3. Age ratings

Complete every question. Panopticon does not include publisher-supplied violence, sexual content, gambling, unrestricted browser access, or public user-generated content.

The app can display previews of arbitrary windows already opened by the user. Explain this accurately if the questionnaire asks about user-selected or user-generated content. Panopticon does not provide or upload that source content.

### 4. Packages

Upload the reviewed `.msix` from `artifacts/store/x64`.

Before upload, confirm:

- exact Partner Center `Name`, `Publisher`, and `PublisherDisplayName`;
- `Windows.Desktop` is the only target family;
- package version is greater than every prior applicable Store package;
- architecture is x64;
- `runFullTrust` is the only restricted capability;
- application ID remains `App`;
- executable is `panopticon.exe`;
- Store assets are present and readable;
- no `.pfx`, private key, password, build staging directory, or direct installer is inside the package/output bundle;
- the build evidence identifies the exact source commit;
- the binary was compiled with `PANOPTICON_DISTRIBUTION_CHANNEL=store`.

Partner Center may report the uploaded file as Validated while the Packages section remains Incomplete. Finish device-family availability and all package controls before submission.

### 5. Store listings

Prepare English and Spanish listings using [`LISTING.md`](LISTING.md).

At least one screenshot is required; prepare at least five:

1. multi-window grid overview;
2. alternate layout such as Mosaic or Bento;
3. filters/grouping/tags;
4. Settings with themes or background controls;
5. tray or appbar/dock workflow;
6. optional workspace or command-palette view.

Screenshots can expose window titles and source-window contents. Use dedicated synthetic windows and fixtures, never a normal personal desktop.

### 6. Submission options

Panopticon declares `runFullTrust`. Explain the concrete desktop operations:

- enumerate top-level windows and related metadata;
- register live DWM thumbnails;
- activate, restore, minimize, close, or arrange windows when requested;
- register a tray icon and global hotkey;
- optionally register appbar/dock behavior and startup preference;
- persist local settings/workspaces;
- use WinHTTP only in the direct channel, not the Store build.

For the first submission, describe how to open several harmless windows, switch layouts, activate a thumbnail, test tray behavior, and verify that the Store build does not contact GitHub Releases.

## Package versioning

Cargo uses SemVer:

```text
0.1.23
```

MSIX uses four numeric components:

```text
0.1.23.0
```

Rules:

- each MSIX component is 0–65535;
- pre-release suffixes never appear in `Identity.Version`;
- package name and publisher remain stable after first Store publication;
- every applicable submission version is greater than the previous one;
- source tag, Cargo version, MSIX version, artifact hash, and commit are recorded together.

## Store build update policy

The compile-time distribution channel is implemented in `src/app/distribution.rs`.

Direct/default behavior:

```text
channel = direct
GitHub release checker = enabled
```

Store behavior:

```text
channel = store
GitHub release checker = skipped
update status = current version / Store-managed
```

Release validation must inspect runtime logs or network behavior to prove the Store binary skips the GitHub endpoint.

## Lifecycle and desktop qualification

| Scenario | Required result |
| --- | --- |
| Clean install | Package installs without Rust, Visual Studio, or developer tools |
| First launch | Dashboard opens from Start |
| Window discovery | Normal top-level windows appear; excluded/system windows remain excluded |
| DWM previews | Live previews render and release correctly |
| Activation | Selecting a thumbnail activates its source window |
| Layouts | Grid, Mosaic, Bento, Fibonacci, Columns, Row, and Column remain usable |
| Filters/grouping/tags | Rules and grouping update the displayed set correctly |
| Settings | Theme, background, shortcuts, and workspaces persist |
| Tray | Hide/show/exit works and icon recovers after Explorer restart |
| Global hotkey | Activation hotkey registers and unregisters safely |
| Appbar/dock | Optional mode positions and unregisters correctly |
| Multiple instances/workspaces | Instance behavior matches documented policy |
| Store update policy | No GitHub release request is made |
| Upgrade | Settings and workspaces remain intact |
| Uninstall | App registration, tray, hotkey, and appbar state are removed |
| Reinstall | No stale shell state prevents startup |
| Standard user | Normal use requires no elevation |
| Product language | English UI remains complete and usable |
| High DPI/multiple monitors | Layout and previews remain aligned |
| Sensitive-content review | No private source content appears in release screenshots/evidence |

## Privacy and security review

Confirm every release matches [`../../PRIVACY.md`](../../PRIVACY.md):

- live DWM previews remain local;
- no desktop metadata is uploaded;
- Store update checks do not call GitHub;
- direct update checks retrieve release metadata only;
- logs and screenshots may contain sensitive window metadata and must be reviewed;
- user-selected background paths remain local;
- normal operation remains non-elevated.

## Direct-channel signing backlog

This runbook does not invent a production certificate. Before presenting the Inno Setup installer or portable executables as trusted public binaries:

1. obtain an Authenticode signing solution suitable for the publisher;
2. sign `panopticon.exe` before building the ZIP and installer;
3. timestamp the signature with RFC 3161;
4. build the Inno installer from signed binaries;
5. sign and timestamp the final installer;
6. verify every signature;
7. generate SHA-256 hashes and release provenance.

Store signing and direct Authenticode signing are separate decisions.

## Release gates

- [ ] Product reserved in Partner Center.
- [ ] `store-identity.json` contains exact reserved values.
- [ ] `Test-StoreReadiness.ps1 -RequireReservedIdentity` passes.
- [ ] Cargo format, Clippy, tests, audit, and release build pass.
- [ ] Store MSIX built from the submission commit.
- [ ] Store binary proves GitHub update checks are disabled.
- [ ] Hash, version, architecture, and source commit are recorded locally.
- [ ] Clean install, launch, DWM, layouts, tray, hotkey, appbar, upgrade, uninstall, and reinstall are evidenced.
- [ ] Public privacy URL is stable.
- [ ] English and Spanish listing copy is reviewed.
- [ ] Screenshots use synthetic non-sensitive windows.
- [ ] Age rating is complete.
- [ ] `runFullTrust` explanation is entered.
- [ ] First release uses an intentional publishing hold.

## First-submission sequence

1. Reserve Panopticon in Partner Center.
2. Run `Set-StoreIdentity.ps1` with exact Product identity values.
3. Update `Cargo.toml` version if required and commit the release version.
4. Run `Test-StoreReadiness.ps1 -RequireReservedIdentity`.
5. Run the complete Rust CI sequence.
6. Run `Build-StoreMsix.ps1`.
7. Record identity, hash, commit, and qualification notes locally. Do not commit credentials, private Partner Center screenshots, certificate private keys, or source-window contents.
8. Qualify the MSIX on a clean Windows VM/profile.
9. Complete all six Partner Center sections.
10. Upload only the reviewed `.msix`.
11. Submit with a manual publishing hold.
12. Review the certification report, final listing, and delivered package before the listing becomes visible.

## Non-automatable account steps

The repository cannot safely choose or submit:

- Store reservation and identity;
- markets, audience, discoverability, schedule, or price;
- company contact information;
- age-rating answers;
- public privacy/support URL ownership;
- screenshots and promotional assets;
- restricted-capability form content;
- final certification and publishing decision.

## Official references

- [Create an app submission for an MSIX app](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/create-app-submission)
- [Upload MSIX app packages](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/upload-app-packages)
- [Manage submission options](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/manage-submission-options)
- [App capability declarations](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations)
- [Package a desktop app with MSIX](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-root)
- [Microsoft Store policies](https://learn.microsoft.com/en-us/windows/apps/publish/store-policies)
