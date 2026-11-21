#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
// Bildschirmschoner (Windows: als .scr umbenennen; macOS: zum Testen im Fenster/Vollbild).
// Argumente nach Windows-Konvention: /s = Schoner starten, /c = Einstellungen (keine), /p = Vorschau (nichts tun).
use include_dir::{include_dir, Dir};
use std::{borrow::Cow, time::Instant};
use tao::{
    dpi::LogicalSize,
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Fullscreen, WindowBuilder},
};
use wry::{http::Response, WebViewBuilder};

#[path = "../fetch.rs"]
mod fetch;

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

fn main() -> wry::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).map(|a| a.to_lowercase()).collect();
    let first = args.first().cloned().unwrap_or_default();
    if first.starts_with("/c") || first.starts_with("/p") {
        return Ok(()); // keine Einstellungen, keine Vorschau im Windows-Dialog
    }
    let windowed = args.iter().any(|a| a == "--window");

    let baked = ASSETS
        .get_file("images.json")
        .map(|f| String::from_utf8_lossy(f.contents()).to_string())
        .unwrap_or_else(|| "[]".into());
    let baked_logo = ASSETS
        .get_file("logo.url")
        .map(|f| String::from_utf8_lossy(f.contents()).trim().to_string())
        .unwrap_or_default();
    let (images, logo) = fetch::images_json(baked, baked_logo);

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let mut wb = WindowBuilder::new().with_title("GTA6");
    if windowed {
        wb = wb.with_inner_size(LogicalSize::new(960.0, 540.0));
    } else {
        wb = wb
            .with_decorations(false)
            .with_always_on_top(true)
            .with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    let window = wb.build(&event_loop).unwrap();
    if !windowed {
        window.set_cursor_visible(false);
    }

    // Auf Windows fängt WebView2 Maus und Tastatur ab, daher meldet die Seite Eingaben selbst.
    let exit_js = r#"(function(){var t0=Date.now(),ox=null,oy=null;function bye(){if(Date.now()-t0>1000)window.ipc.postMessage('exit')}
addEventListener('keydown',bye);addEventListener('mousedown',bye);addEventListener('touchstart',bye);addEventListener('wheel',bye);
addEventListener('mousemove',function(e){if(ox===null){ox=e.screenX;oy=e.screenY;return}if(Math.abs(e.screenX-ox)+Math.abs(e.screenY-oy)>12)bye()})})();"#;
    let _webview = WebViewBuilder::new()
        .with_initialization_script(if windowed { "" } else { exit_js })
        .with_ipc_handler(move |_req| {
            let _ = proxy.send_event(());
        })
        .with_custom_protocol("gta".into(), move |_id, req| {
            let mut path = req.uri().path().trim_start_matches('/').to_string();
            if path.is_empty() {
                path = "saver.html".into();
            }
            let body: Option<Vec<u8>> = match path.as_str() {
                "images.json" => Some(images.clone().into_bytes()),
                "logo.url" => Some(logo.clone().into_bytes()),
                _ => ASSETS.get_file(&path).map(|f| f.contents().to_vec()),
            };
            let mime = if path.ends_with(".html") { "text/html; charset=utf-8" } else { "application/octet-stream" };
            match body {
                Some(b) => Response::builder()
                    .header("Content-Type", mime)
                    .header("Cache-Control", "no-store")
                    .body(Cow::Owned(b))
                    .unwrap(),
                None => Response::builder().status(404).body(Cow::Borrowed(&b""[..])).unwrap(),
            }
        })
        .with_url("gta://localhost/saver.html")
        .build(&window)?;

    let start = Instant::now();
    let mut origin: Option<(f64, f64)> = None;
    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        if let Event::UserEvent(()) = event {
            *flow = ControlFlow::Exit;
        }
        if let Event::WindowEvent { event, .. } = event {
            let armed = !windowed && start.elapsed().as_millis() > 1000;
            match event {
                WindowEvent::CloseRequested => *flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { event, .. } if armed && event.state == ElementState::Pressed => *flow = ControlFlow::Exit,
                WindowEvent::MouseInput { .. } if armed => *flow = ControlFlow::Exit,
                WindowEvent::CursorMoved { position, .. } if armed => match origin {
                    None => origin = Some((position.x, position.y)),
                    Some((x, y)) if (position.x - x).abs() + (position.y - y).abs() > 12.0 => *flow = ControlFlow::Exit,
                    _ => {}
                },
                _ => {}
            }
        }
    });
}
