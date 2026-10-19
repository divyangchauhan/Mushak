# Microsoft Store submission runbook

This runbook covers the first Microsoft Store submission for Mushak. The
repository produces an x64 MSIX. Partner Center signs it after certification.

## Repository status

The following work is in the repository:

- The Partner Center identity is set in `packaging/msix/AppxManifest.xml`.
- Packaged startup uses `windows.startupTask` with task ID `MushakStartup`.
- `build-msix.ps1` rebuilds the executable and inspects the finished package.
- The x64 MSVC build links the C runtime statically. Package inspection rejects
  external Visual C++ runtime DLL imports, including delay-loaded imports.
- `test-msix.ps1` runs the Windows App Certification Kit.
- Store listing copy, five desktop screenshots, and a 300 by 300 logo are under
  `docs/`.
- `PRIVACY.md` provides a stable public policy URL after this work reaches
  `main`.

Partner Center submission state is not stored in Git. Confirm the live status
there before uploading a package or starting a new submission.

## Version mapping

The app and Store package have separate versions:

| App version | Store package version |
| --- | --- |
| `0.0.3` | `1.0.3.0` |
| `0.0.4` | `1.0.4.0` |
| `0.0.6` | `1.0.6.0` |
| `0.1.0` | `1.1.0.0` |
| `1.0.0` | `2.0.0.0` |

The mapping adds one to the Cargo major version. It keeps Store versions
increasing, avoids a forbidden zero first field, and leaves the fourth field
zero for Microsoft.

## Build the candidate

Build only from a clean commit that you intend to publish:

```powershell
git status --short
cargo test --locked
pwsh packaging/msix/build-msix.ps1
```

For app version `0.0.4`, the output is
`target/msix/mushak-0.0.4-x64.msix`. Record the printed SHA-256 with the
submission notes.

The package is unsigned. That is correct for a Partner Center MSIX upload.
Microsoft replaces the package signature after certification.

## Test the exact package

Do not rely on an older WACK report. Quit the installed Mushak process and test
the exact file you plan to upload:

```powershell
pwsh packaging/msix/test-msix.ps1 `
  -PackagePath target/msix/mushak-0.0.4-x64.msix
```

Review the XML report even when the overall result is `PASS`. Record any
optional failures or warnings in the certification notes.

A full local install test needs a temporary certificate whose subject matches
the manifest publisher. Keep its private key and exported certificate outside
the repository. Verify install, first launch, "Start with Windows", update, and
uninstall before shipping.

## Partner Center fields

Use these values for the first submission:

- Product name: Mushak
- Price: Free
- Audience: Public
- Markets: All intended markets
- Category: Utilities and tools
- Device family: Windows Desktop
- Architecture: x64
- Support URL: <https://github.com/divyangchauhan/Mushak/issues>
- Website: <https://github.com/divyangchauhan/Mushak>
- Privacy URL: <https://github.com/divyangchauhan/Mushak/blob/main/PRIVACY.md>

Complete every age-rating question. Mushak has no advertising, purchases,
user-generated content, or network service.

Upload the MSIX from `target/msix/`. Partner Center should report identity
`DivyangChauhan.Mushak`, architecture `x64`, and the mapped Store package
version.

Use `docs/ms-store-listing.md` for the English listing. Upload the screenshots
in their numbered order and use `StoreLogo-300x300.png` as the app tile icon.

## Restricted capability justification

The package declares `runFullTrust`. Paste this into the restricted capability
field:

> Mushak is a Win32 tray utility for Logitech HID++ mice. It uses Windows
> HID APIs to communicate with the mouse and a low-level mouse hook to apply
> user-configured button mappings. It does not install a driver or service,
> collect data, or run code received from a network.

## Certification notes

Use this note for the reviewer:

> Mushak 0.0.6 adds experimental support for the MX Master family and MX Vertical
> through HID++ capability discovery over Bluetooth, compatible receivers, and
> supported wired USB. The settings window opens without a mouse and remembers
> the last mouse name as disconnected. Live device settings require a connected
> mouse and appear only when supported. Physical validation across all added
> models is pending. Mushak does not install a driver or service. The
> `runFullTrust` capability permits the Win32 tray process, Windows HID APIs,
> and a low-level mouse hook for user-configured mappings.

For the policy 10.2.4.1 resubmission, add this after verifying the replacement
package:

> The previous package imported VCRUNTIME140.dll without declaring a runtime
> dependency. This replacement statically links the runtime. The executable
> extracted from the final MSIX has no external Visual C++ runtime DLL imports
> and does not require a separate Visual C++ Redistributable installation.

Local WACK verification on 2026-10-19 used kit `10.0.26100.8249` against a
test-signed copy of the corrected `1.0.6.0` package. The executable, manifest,
assets, and block map matched the unsigned Store candidate. Overall result was
`PASS`, with `PARTIAL_RUN=FALSE` and all 13 required tests passing. Of the 24
checks, 23 passed and one optional check, `Blocked executables`, failed.

Include this explanation with the resubmission notes:

> WACK reports an overall PASS. Its optional Blocked executables check flags
> CreateProcessW and cmd.exe strings. Mushak's process launch uses Rust's
> std::process::Command to open its own executable with the fixed --settings
> argument. It does not launch command scripts or cmd.exe. Rust's Windows
> process implementation includes cmd.exe support for batch files, which
> accounts for the strings detected by the static check.

The matching Rust toolchain source implements that batch-file handling in
[windows.rs](https://github.com/rust-lang/rust/blob/2d8144b7880597b6e6d3dfd63a9a9efae3f533d3/library/std/src/sys/process/windows.rs#L275-L302).
Keep the XML report and package hash with the submission record. These local
results do not establish Partner Center certification or publication.

## Final checklist

- Confirm the branch is clean and the commit matches the intended release.
- Build the candidate once and record its SHA-256.
- Run WACK against that exact package.
- Complete a signed local install, startup, update, and uninstall test.
- Confirm the privacy policy URL is public.
- Upload the package and all required listing fields.
- Review Partner Center warnings and package availability.
- Submit for certification.
- After publication, add the Store product link to the README.

Do not add a Store badge or claim Store availability before the public product
page works in a signed-out browser.
