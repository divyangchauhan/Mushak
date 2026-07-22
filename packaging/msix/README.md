# MSIX package (Microsoft Store build)

Scaffolding for the Microsoft Store build of Mushak. The Store re-signs the
package for distribution, so no paid code-signing certificate is needed for
this channel.

## Before the first build

1. Register in Partner Center as an Individual and reserve the name "Mushak".
2. On the app's **Product identity** page, copy these into
   `AppxManifest.xml` `<Identity>`, replacing the `REPLACEME` placeholders:
   - `Name=` -> Package/Identity/Name (e.g. `1234Publisher.Mushak`)
   - `Publisher=` -> Package/Identity/Publisher (e.g. `CN=ABCD1234-...`)

   They must match Partner Center exactly or the upload is rejected.

The `windows.startupTask` `TaskId` in the manifest (`MushakStartup`) must stay
in sync with `STARTUP_TASK_ID` in `src/startup.rs`; that is how the packaged
build's "start with Windows" toggle works.

## Build

```powershell
# Structural pack (version comes from Cargo.toml, coerced to x.x.x.0):
pwsh packaging/msix/build-msix.ps1
```

Output: `target/msix/mushak-<version>-x64.msix`.

## Test on your own machine

The Store signs the package for release, but to install it locally you must
sign it with a cert your machine trusts (local testing only, never shipped):

```powershell
# One-time: create + trust a self-signed cert whose subject matches Publisher
# (see the full runbook for the exact New-SelfSignedCertificate command).
pwsh packaging/msix/build-msix.ps1 -Sign -CertSubject "CN=<your-publisher-id>"
Add-AppxPackage target/msix/mushak-<version>-x64.msix
```

Then run the Windows App Certification Kit (`appcert.exe`) against the package
before submitting.

## Files

- `AppxManifest.xml` - package manifest (fill in identity placeholders).
- `Assets/` - Store/tile logos generated from the Modak icon.
- `build-msix.ps1` - stages and packs the MSIX.

The full step-by-step submission runbook (Partner Center flow, WACK, listing,
publish) is kept locally as `docs/ms-store-submission.md`.
