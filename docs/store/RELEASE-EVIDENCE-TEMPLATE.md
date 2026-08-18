# Panopticon Microsoft Store release evidence

Copy this file for each submission. Do not commit credentials, private Partner Center screenshots, certificate private keys, or source-window contents.

## Identity

| Field | Value |
| --- | --- |
| Product | Panopticon |
| Store ID | `[value]` |
| Package identity name | `[value]` |
| Publisher | `[value]` |
| Publisher display name | `[value]` |
| PFN | `[verification value]` |
| Package SID | `[verification value]` |
| Cargo version | `[0.0.0]` |
| MSIX version | `[0.0.0.0]` |
| Architecture | `x64` |
| Source commit | `[SHA]` |
| Source tag | `[tag/n-a]` |
| Build date UTC | `[timestamp]` |

## Artifact

| Field | Value |
| --- | --- |
| Package file | `[Panopticon-...-x64.msix]` |
| Size | `[bytes]` |
| SHA-256 | `[hash]` |
| Target device family | `Windows.Desktop` |
| Minimum OS | `10.0.17763.0` |
| Application ID | `App` |
| Executable | `panopticon.exe` |
| Restricted capability | `runFullTrust` |
| Distribution channel | `store` |
| Build command/workflow | `[reference]` |

## Automated gates

| Check | Command/run | Result | Evidence |
| --- | --- | --- | --- |
| Store structure | `.\scripts\Test-StoreReadiness.ps1 -RequireReservedIdentity` | `[pass/fail]` | |
| Format | `cargo fmt -- --check` | `[pass/fail]` | |
| Clippy | `cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic` | `[pass/fail]` | |
| Tests | `cargo test --all-targets --locked` | `[pass/fail]` | |
| Release build | `cargo build --release --locked` | `[pass/fail]` | |
| Dependency audit | `cargo audit` | `[pass/fail]` | |
| Store MSIX | `.\scripts\Build-StoreMsix.ps1` | `[pass/fail]` | |
| Signature inspection | `Get-AuthenticodeSignature` / `signtool verify` | `[pass/fail]` | |

## Test environment

| Field | Value |
| --- | --- |
| Windows edition/version/build | `[value]` |
| CPU architecture | `[value]` |
| Clean VM/profile | `[yes/no]` |
| Developer tools installed | `[yes/no]` |
| Account type | `[standard/admin]` |
| Monitor count/resolution/scaling | `[values]` |
| Explorer version/state | `[value]` |

## Lifecycle and functionality

| Scenario | Result | Evidence/notes |
| --- | --- | --- |
| Clean install | `[pass/fail]` | |
| Start-menu launch | `[pass/fail]` | |
| Normal-window discovery | `[pass/fail]` | source apps listed |
| Panopticon/self exclusion | `[pass/fail]` | |
| DWM preview registration | `[pass/fail]` | |
| Preview release after close | `[pass/fail]` | |
| Window activation | `[pass/fail]` | |
| Minimize/restore | `[pass/fail]` | |
| Close action on disposable window | `[pass/fail]` | |
| Grid | `[pass/fail]` | |
| Mosaic | `[pass/fail]` | |
| Bento | `[pass/fail]` | |
| Fibonacci | `[pass/fail]` | |
| Columns | `[pass/fail]` | |
| Row | `[pass/fail]` | |
| Column | `[pass/fail]` | |
| Filters/grouping/tags | `[pass/fail]` | |
| Per-app rules | `[pass/fail]` | |
| Settings persistence | `[pass/fail]` | |
| Workspace persistence | `[pass/fail]` | |
| English UI | `[pass/fail]` | |
| Spanish UI | `[pass/fail]` | |
| Tray hide/show/exit | `[pass/fail]` | |
| Explorer restart recovery | `[pass/fail]` | |
| Global hotkey register/unregister | `[pass/fail]` | |
| Appbar/dock register/unregister | `[pass/fail]` | |
| High-DPI alignment | `[pass/fail]` | |
| Multiple monitors | `[pass/fail/n-a]` | |
| Offline launch | `[pass/fail]` | |
| Upgrade from prior Store version | `[pass/fail/n-a]` | |
| Settings/workspaces preserved | `[pass/fail/n-a]` | |
| Uninstall | `[pass/fail]` | |
| No stale tray/hotkey/appbar state | `[pass/fail]` | |
| Reinstall | `[pass/fail]` | |
| Standard-user operation | `[pass/fail]` | |

## Store update-channel proof

- Build environment variable: `PANOPTICON_DISTRIBUTION_CHANNEL=store`
- Runtime log reports Store-managed updates: `[yes/no + excerpt]`
- GitHub Releases endpoint contacted during startup: `[no required]`
- GitHub Releases endpoint contacted after manual update action: `[no required]`
- Microsoft Store package update behavior reviewed: `[yes/no]`

Use a network monitor or controlled endpoint test if needed. Do not infer this only from UI text.

## Privacy and screenshot review

- [ ] Public privacy URL loads without authentication.
- [ ] Policy matches window metadata, DWM, local storage, and update behavior.
- [ ] Screenshot session used only synthetic/disposable windows.
- [ ] No personal email, chat, browser account, notification, terminal history, path, machine name, source code, customer document, or credential is visible.
- [ ] Logs contain no unnecessary sensitive titles or paths.
- [ ] No source-window screenshot or recording is attached to public evidence without review.
- [ ] No `.pfx`, private key, certificate password, or Partner Center secret is present.
- [ ] Normal use requires no elevation.

## Partner Center

### Pricing and availability

- Markets: `[selection]`
- Audience: `[selection]`
- Discoverability: `[selection]`
- Schedule: `[selection]`
- Base price: `[selection]`
- Publishing hold: `[selection]`

### Properties

- Category/subcategory: `[selection]`
- Privacy policy URL: `[url]`
- Product website: `[url]`
- Support: `[url/email]`
- Contact information complete: `[yes/no]`
- Requirements complete: `[yes/no]`

### Age ratings

- Questionnaire complete: `[yes/no]`
- User-selected window content answer reviewed: `[yes/no]`
- Assigned rating: `[value]`

### Packages

- Upload validation: `[result]`
- Packages section complete: `[yes/no]`
- Device family: `Windows Desktop only`
- Architecture: `x64`
- Warnings: `[none/list]`

### Store listings

- English listing reviewed: `[yes/no]`
- Spanish listing reviewed: `[yes/no]`
- What's new updated: `[yes/no]`
- Screenshot count: `[number]`
- Synthetic-content review complete: `[yes/no]`

### Submission options

- Notes date: `[date]`
- DWM/privacy testing notice included: `[yes/no]`
- `runFullTrust` explanation entered: `[yes/no]`
- Store-managed update behavior included: `[yes/no]`
- Notification audience reviewed: `[yes/no]`
- Publishing hold confirmed: `[yes/no]`

## Certification outcome

| Field | Value |
| --- | --- |
| Submitted | `[timestamp]` |
| Result | `[passed/failed/cancelled]` |
| Findings | `[summary]` |
| Remediation | `[commit/submission]` |
| Approved | `[timestamp]` |
| Published/held | `[value]` |
| Live Store URL | `[url]` |
| Deep link | `[value]` |

## Approval

- Engineering: `[name/date]`
- Product/listing: `[name/date]`
- Privacy/security: `[name/date]`
- Publisher owner: `[name/date]`
- Decision: `[publish/hold/reject]`
