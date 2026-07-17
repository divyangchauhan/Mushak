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
