# Mushak UIAccess wheel helper

This executable is an intentionally narrow security boundary. It accepts only
bounded 32-bit vertical wheel deltas on an inherited standard-input pipe and
calls `SendInput`. It verifies that its actual parent is the protected sibling
`mushak.exe`, rate-limits the protocol, and has no keyboard commands, named IPC
endpoint, config, filesystem writes, network access, or UI.

Windows grants the embedded `uiAccess="true"` request only when the executable
is Authenticode-signed by a trusted certificate and installed in an
administrator-protected directory such as `Program Files`. The helper verifies
`TokenUIAccess` at runtime and exits before accepting commands if Windows did
not grant it.

The normal Mushak resident treats helper loss as a hard failure and requests
native wheel reporting from the mouse, so a missing, unsigned, or crashed
helper cannot leave scrolling permanently diverted.
