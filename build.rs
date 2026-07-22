//! Rasterises `assets/icons/*.svg` to PNG at build time, and embeds the app
//! icon into the exe as a Win32 icon resource (so shells show it, not a
//! generic default).
//!
//! Ply can render SVG only through its `tinyvg` feature, which wants TinyVG
//! rather than SVG, and a runtime rasteriser would ship resvg inside the
//! binary. resvg is a *build* dependency instead: the shipped binary carries
//! only the finished pixels.
//!
//! Icons are emitted white-on-transparent. Ply tints an image by the element's
//! `background_color` (defaulting to white when transparent), which is how the
//! design's `currentColor` icons take the palette colour at each call site.

use std::path::{Path, PathBuf};

/// Application manifest embedded into the exe. Declares Per-Monitor v2 DPI
/// awareness (with the pre-1607 `true/pm` fallback) and the standard asInvoker
/// run level.
const APP_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
"#;

/// Rasterisation size for UI glyphs, in pixels. The design draws them between
/// 12 and 26pt; 64px covers that at 200% scaling with room to spare, and Ply
/// samples down with a linear filter.
const UI_SIZE: u32 = 64;
/// The app icon is also rendered into a 256px .ico, so it needs the detail.
const APP_SIZE: u32 = 256;
/// Raw-RGBA sizes emitted for the app icon.
///
/// 32 is the tray icon (16pt, doubled for a 200% display). 16/32/64 are the
/// three levels miniquad's `conf::Icon` requires for the window — and so the
/// taskbar button — as fixed-size arrays.
const RGBA_SIZES: [u32; 3] = [16, 32, 64];

fn main() {
    println!("cargo:rerun-if-changed=assets/icons");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let icon_out = out_dir.join("icons");
    std::fs::create_dir_all(&icon_out).expect("create icon out dir");

    let src = Path::new("assets/icons");
    let entries = std::fs::read_dir(src).expect("read assets/icons");

    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("svg") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).expect("stem");
        let size = if stem.starts_with("modak") {
            APP_SIZE
        } else {
            UI_SIZE
        };
        let (png, _) = render(&path, size);
        std::fs::write(icon_out.join(format!("{stem}.png")), png).expect("write png");

        // The tray and the window icon both take raw RGBA, and both live in
        // processes with no image decoder linked. Emit the pixels directly so
        // they can be include_bytes!'d.
        if stem.starts_with("modak") {
            for size in RGBA_SIZES {
                let (_, rgba) = render(&path, size);
                std::fs::write(icon_out.join(format!("{stem}_{size}.rgba")), rgba)
                    .expect("write rgba");
            }
        }
        count += 1;
    }
    println!("cargo:warning=rasterised {count} icons");

    // Embed the app icon into the exe as a Win32 icon resource. Without this
    // the binary has no icon, so Explorer, Start-menu shortcuts (Scoop's
    // included), and pinned launchers all fall back to a generic default.
    // Built from the same SVG source as every other icon.
    let ico_path = build_app_ico(&src.join("modak_active.svg"), &out_dir);
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico_path.to_str().expect("ico path is valid utf-8"));
    // Populate the exe's version-info block. CompanyName in particular is what
    // Task Manager's Startup "Publisher" column and Explorer's Details tab
    // read; without it (winresource derives it from the empty `authors` field)
    // the app shows a blank publisher everywhere.
    res.set("CompanyName", "Divyang Chauhan");
    res.set("ProductName", "Mushak");
    res.set(
        "FileDescription",
        "Lightweight replacement for Logitech Options+ for the MX Master 2S",
    );
    res.set("LegalCopyright", "Copyright (c) 2026 Divyang Chauhan");
    // Declare Per-Monitor v2 DPI awareness in the application manifest so it is
    // set at process start. The GUI backend (miniquad) sets awareness at
    // runtime, but via a dynamically-resolved call the certification tooling
    // can't see statically, so it flagged the app as "not DPI aware". Declaring
    // it here makes the app crisp on mixed-DPI setups and clears that warning.
    // trustInfo/asInvoker keeps the standard (non-elevating) run level.
    res.set_manifest(APP_MANIFEST);
    res.compile().expect("embed windows icon resource");
}

/// Assemble a multi-resolution `.ico` for the exe's Win32 icon resource and
/// return its path. Windows shells pick the best-fit size from these.
fn build_app_ico(svg: &Path, out_dir: &Path) -> PathBuf {
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16u32, 24, 32, 48, 64, 128, 256] {
        let (_, rgba) = render(svg, size);
        let image = ico::IconImage::from_rgba_data(size, size, rgba);
        dir.add_entry(ico::IconDirEntry::encode(&image).expect("encode ico entry"));
    }
    let path = out_dir.join("mushak.ico");
    let file = std::fs::File::create(&path).expect("create mushak.ico");
    dir.write(file).expect("write mushak.ico");
    path
}

/// Render one SVG to `size`x`size`, returning (PNG bytes, raw RGBA bytes).
fn render(path: &Path, size: u32) -> (Vec<u8>, Vec<u8>) {
    // Use resvg's re-exports rather than depending on usvg/tiny-skia directly:
    // separate versions drag a second tiny_skia_path into the graph and the
    // types stop unifying.
    use resvg::{tiny_skia, usvg};

    let data = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default())
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("alloc pixmap");
    // The SVGs use assorted viewBoxes (12, 13, 24, and a tall one for the app
    // icon); scale each to fit `size` rather than assuming one, and centre it —
    // a non-square viewBox would otherwise be pinned to the top-left corner.
    let vb = tree.size();
    let scale = size as f32 / vb.width().max(vb.height());
    let tx = (size as f32 - vb.width() * scale) / 2.0;
    let ty = (size as f32 - vb.height() * scale) / 2.0;
    resvg::render(
        &tree,
        usvg::Transform::from_translate(tx, ty).pre_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let png = pixmap.encode_png().expect("encode png");
    // tiny-skia stores premultiplied alpha; tray_icon wants straight RGBA.
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let a = p.alpha();
            if a == 0 {
                [0, 0, 0, 0]
            } else {
                let f = 255.0 / a as f32;
                [
                    (p.red() as f32 * f).min(255.0) as u8,
                    (p.green() as f32 * f).min(255.0) as u8,
                    (p.blue() as f32 * f).min(255.0) as u8,
                    a,
                ]
            }
        })
        .collect();
    (png, rgba)
}
