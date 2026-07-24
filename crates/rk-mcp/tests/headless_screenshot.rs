//! Headless rendering smoke test. Skips (with a message) when no GPU
//! adapter is available, e.g. on bare CI runners.

use std::sync::Arc;

use rk_engine::{Command, Engine, PrimitiveSpec};
use rk_mcp::headless::{HeadlessRenderer, ScreenshotOptions, ViewPreset};

#[test]
fn screenshot_renders_a_valid_png() {
    let renderer = match HeadlessRenderer::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("skipping headless screenshot test (no GPU): {e}");
            return;
        }
    };

    let mut engine = Engine::new(Arc::from(rk_cad::default_kernel()));
    engine
        .apply(Command::CreatePrimitive {
            id: None,
            primitive: PrimitiveSpec::Box {
                size: [0.2, 0.1, 0.05],
            },
            name: Some("base".into()),
        })
        .unwrap();

    let opts = ScreenshotOptions {
        width: 320,
        height: 240,
        ..Default::default()
    };
    let png = renderer.screenshot(&mut engine, &opts).unwrap();

    // PNG signature + decodable at the requested size
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let decoder = png::Decoder::new(std::io::Cursor::new(&png));
    let reader = decoder.read_info().unwrap();
    assert_eq!(reader.info().width, 320);
    assert_eq!(reader.info().height, 240);

    // A second shot from another preset must work with the same device
    let opts = ScreenshotOptions {
        width: 320,
        height: 240,
        view: ViewPreset::Top,
        ..Default::default()
    };
    let png2 = renderer.screenshot(&mut engine, &opts).unwrap();
    assert_eq!(&png2[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn screenshot_rejects_bad_dimensions() {
    let renderer = match HeadlessRenderer::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("skipping headless screenshot test (no GPU): {e}");
            return;
        }
    };
    let mut engine = Engine::new(Arc::from(rk_cad::default_kernel()));
    let opts = ScreenshotOptions {
        width: 0,
        height: 240,
        ..Default::default()
    };
    assert!(renderer.screenshot(&mut engine, &opts).is_err());
}
