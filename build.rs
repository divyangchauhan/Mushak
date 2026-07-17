//! Rasterises `assets/icons/*.svg` to PNG at build time.
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

/// Rasterisation size for UI glyphs, in pixels. The design draws them between
/// 12 and 26pt; 64px covers that at 200% scaling with room to spare, and Ply
/// samples down with a linear filter.
const UI_SIZE: u32 = 64;
/// The app icon is also rendered into a 256px .ico, so it needs the detail.
const APP_SIZE: u32 = 256;
/// Windows tray icons are 16pt; 32px covers a 200% display.
const TRAY_SIZE: u32 = 32;

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

        // The tray takes raw RGBA, and lives in the resident process which has
        // no image decoder linked. Emit the pixels directly so tray.rs can
        // include_bytes! them.
        if stem.starts_with("modak") {
            let (_, rgba) = render(&path, TRAY_SIZE);
            std::fs::write(icon_out.join(format!("{stem}.rgba")), rgba).expect("write rgba");
        }
        count += 1;
    }
    println!("cargo:warning=rasterised {count} icons");
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
    // The SVGs use assorted viewBoxes (12, 13, 24, 64); scale each to `size`
    // rather than assuming one.
    let vb = tree.size();
    let scale = size as f32 / vb.width().max(vb.height());
    resvg::render(
        &tree,
        usvg::Transform::from_scale(scale, scale),
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
