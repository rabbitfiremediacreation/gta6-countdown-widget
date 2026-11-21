use include_dir::{include_dir, Dir};
use std::{borrow::Cow, rc::Rc};

mod fetch;
use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use wry::{http::Response, WebViewBuilder};

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn build_menu() -> Menu {
    let menu = Menu::new();
    let app = Submenu::new("GTA6 COUNTDOWN", true);
    let settings = MenuItem::with_id("settings", "Settings…", true, Some("CmdOrCtrl+,".parse().unwrap()));
    let _ = app.append_items(&[&settings, &PredefinedMenuItem::separator(), &PredefinedMenuItem::quit(None)]);
    let _ = menu.append_items(&[&app]);
    menu
}

fn main() -> wry::Result<()> {
    let baked = ASSETS
        .get_file("images.json")
        .map(|f| String::from_utf8_lossy(f.contents()).to_string())
        .unwrap_or_else(|| "[]".into());
    let baked_logo = ASSETS
        .get_file("logo.url")
        .map(|f| String::from_utf8_lossy(f.contents()).trim().to_string())
        .unwrap_or_default();
    let (images, logo) = fetch::images_json(baked, baked_logo);
    let event_loop = EventLoopBuilder::<MenuEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(e);
    }));
    let window = Rc::new(WindowBuilder::new()
        .with_title("GTA6 COUNTDOWN")
        .with_inner_size(LogicalSize::new(405.0, 720.0))
        .with_min_inner_size(LogicalSize::new(270.0, 480.0))
        .build(&event_loop)
        .unwrap());
    let win2 = window.clone();

    let webview = WebViewBuilder::new()
        .with_ipc_handler(move |req| {
            // Format-Wechsel aus den Settings: Fenster auf Hoch- oder Querformat bringen
            match req.body().as_str() {
                "fmt:169" => {
                    win2.set_min_inner_size(Some(LogicalSize::new(480.0, 270.0)));
                    let s = win2.inner_size().to_logical::<f64>(win2.scale_factor());
                    if s.width < s.height {
                        win2.set_inner_size(LogicalSize::new(960.0, 540.0));
                    }
                }
                "fmt:916" => {
                    win2.set_min_inner_size(Some(LogicalSize::new(270.0, 480.0)));
                    let s = win2.inner_size().to_logical::<f64>(win2.scale_factor());
                    if s.width > s.height {
                        win2.set_inner_size(LogicalSize::new(405.0, 720.0));
                    }
                }
                _ => {}
            }
        })
        .with_custom_protocol("gta".into(), move |_id, req| {
            let mut path = req.uri().path().trim_start_matches('/').to_string();
            if path.is_empty() {
                path = "index.html".into();
            }
            let body: Option<Vec<u8>> = if path == "images.json" {
                Some(images.clone().into_bytes())
            } else if path == "logo.url" {
                Some(logo.clone().into_bytes())
            } else {
                ASSETS.get_file(&path).map(|f| f.contents().to_vec())
            };
            match body {
                Some(b) => Response::builder()
                    .header("Content-Type", mime(&path))
                    .header("Cache-Control", "no-store")
                    .body(Cow::Owned(b))
                    .unwrap(),
                None => Response::builder().status(404).body(Cow::Borrowed(&b""[..])).unwrap(),
            }
        })
        .with_url("gta://localhost/index.html")
        .build(&*window)?;

    let mut menu_keep: Option<Menu> = None;
    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                let m = build_menu();
                #[cfg(target_os = "macos")]
                m.init_for_nsapp();
                menu_keep = Some(m);
            }
            Event::UserEvent(e) => {
                let js = match e.id().as_ref() {
                    "settings" => "window.gtaOpenMenu&&window.gtaOpenMenu()",
                    _ => "",
                };
                if !js.is_empty() {
                    let _ = webview.evaluate_script(js);
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *flow = ControlFlow::Exit,
            _ => {}
        }
        let _ = &menu_keep;
    });
}
