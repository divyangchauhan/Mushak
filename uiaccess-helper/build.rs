const UIACCESS_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="true"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#;

fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set("CompanyName", "Divyang Chauhan");
    res.set("ProductName", "Mushak UIAccess Wheel Helper");
    res.set(
        "FileDescription",
        "Restricted wheel-input broker for elevated foreground windows",
    );
    res.set("LegalCopyright", "Copyright (c) 2026 Divyang Chauhan");
    res.set_manifest(UIACCESS_MANIFEST);
    res.compile().expect("embed UIAccess helper manifest");
}
