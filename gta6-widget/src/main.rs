use include_dir::{include_dir, Dir};
use std::{borrow::Cow, process::Command};
use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use wry::{http::Response, WebViewBuilder};

static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

const PAGES: [&str; 2] = [
    "https://www.rockstargames.com/VI/downloads",
    "https://www.rockstargames.com/VI/downloads/screenshots",
];
const PREFIX: &str = "/VI/_next/static/media/";

// Holt die aktuellen Bild-Adressen von Rockstars offiziellen Download-Seiten (die Dateinamen
// enthalten einen Hash, der sich bei Seiten-Updates ändert). Schlägt das fehl, gilt die eingebaute Liste.
fn images_json() -> (String, String) {
    let baked = ASSETS
        .get_file("images.json")
        .map(|f| String::from_utf8_lossy(f.contents()).to_string())
        .unwrap_or_else(|| "[]".into());
    let baked_logo = ASSETS
        .get_file("logo.url")
        .map(|f| String::from_utf8_lossy(f.contents()).trim().to_string())
        .unwrap_or_default();
    let mut logo = baked_logo.clone();
    let mut found: Vec<(String, String)> = Vec::new();
    for page_url in PAGES {
        let page = match Command::new("/usr/bin/curl").args(["-sL", "--max-time", "5", page_url]).output() {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => continue,
        };
        let mut rest = page.as_str();
        while let Some(i) = rest.find(PREFIX) {
            let tail = &rest[i..];
            let end = tail
                .find(|c: char| !(c.is_ascii_alphanumeric() || "/_.~%-".contains(c)))
                .unwrap_or(tail.len());
            let path = &tail[..end];
            if path.ends_with(".png") && path[PREFIX.len()..].split('.').next() == Some("vi") {
                logo = format!("https://www.rockstargames.com{}", path);
            }
            if path.ends_with(".jpg") {
                let base = path[PREFIX.len()..].split('.').next().unwrap_or("").to_string();
                found.push((base, format!("https://www.rockstargames.com{}", path)));
            }
            rest = &tail[end.max(1)..];
        }
    }
    // in der eingebauten Liste nur die Adressen durch die frischen ersetzen (Kategorie und Ausschnitt bleiben)
    let mut out = baked.clone();
    for entry in baked.split("},") {
        if let (Some(n), Some(old)) = (field(entry, "\"n\":\""), field(entry, "\"u\":\"")) {
            if let Some((_, fresh)) = found.iter().find(|(b, _)| *b == n) {
                out = out.replace(&old, fresh);
            }
        }
    }
    (out, logo)
}

fn field(s: &str, key: &str) -> Option<String> {
    let i = s.find(key)? + key.len();
    let e = s[i..].find('"')?;
    Some(s[i..i + e].to_string())
}

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
    let (images, logo) = images_json();
    let event_loop = EventLoopBuilder::<MenuEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(e);
    }));
    let window = WindowBuilder::new()
        .with_title("GTA6 COUNTDOWN")
        .with_inner_size(LogicalSize::new(405.0, 720.0))
        .with_min_inner_size(LogicalSize::new(270.0, 480.0))
        .build(&event_loop)
        .unwrap();

    let webview = WebViewBuilder::new()
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
        .build(&window)?;

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
