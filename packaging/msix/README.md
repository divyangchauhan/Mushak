# Microsoft Store package

Mushak ships to the Microsoft Store as an x64 MSIX package. Partner Center
already assigned the package identity in `AppxManifest.xml`:

- Name: `DivyangChauhan.Mushak`
- Publisher: `CN=E262040C-30A8-40A4-800F-6C715C2EF7CF`

These values are case-sensitive and must continue to match Partner Center.

## Build

Run the build from a clean release commit:

```powershell
pwsh packaging/msix/build-msix.ps1
```

The script rebuilds with `Cargo.lock`, rejects identity placeholders, packs and
reopens the finished archive, checks its identity, and prints the package
SHA-256. Mushak `0.0.4` produces:

```text
target/msix/mushak-0.0.4-x64.msix
Store package version: 1.0.4.0
```

Microsoft requires four numeric package version fields. The first field cannot
be zero, and the fourth is reserved for Store use. The default mapping adds one
to the Cargo major version. For example, `0.0.4` maps to `1.0.4.0`.

## Test

Quit every running Mushak process before certification testing, then run:

```powershell
pwsh packaging/msix/test-msix.ps1
```

The script runs the Windows App Certification Kit and fails unless the report's
overall result is `PASS`. The report is written under `target/msix/`.

Partner Center accepts an unsigned MSIX and signs it after certification. A
local installation needs a test certificate whose subject matches the manifest
publisher. Never commit that certificate or upload it to Partner Center.

## Package behavior

The packaged build uses the `windows.startupTask` extension for the "Start with
Windows" setting. Its `MushakStartup` task ID must match `STARTUP_TASK_ID` in
`src/startup.rs`.

The manifest declares `runFullTrust` because Mushak is a Win32 tray application
that communicates with the mouse through Windows HID APIs and installs a mouse
hook for the user's button mappings. Add the justification from
`docs/ms-store-submission.md` to the Partner Center submission options.

Store listing copy, screenshots, certification notes, and the manual submission
checklist live under `docs/`.
