use std::process::Command;

const PAGES: [&str; 2] = [
    "https://www.rockstargames.com/VI/downloads",
    "https://www.rockstargames.com/VI/downloads/screenshots",
];
const PREFIX: &str = "/VI/_next/static/media/";

// Holt die aktuellen Bild-Adressen von Rockstars offiziellen Download-Seiten (die Dateinamen
// enthalten einen Hash, der sich bei Seiten-Updates ändert). Schlägt das fehl, gilt die eingebaute Liste.
pub fn images_json(baked: String, baked_logo: String) -> (String, String) {
    let mut logo = baked_logo.clone();
    let mut found: Vec<(String, String)> = Vec::new();
    for page_url in PAGES {
        let page = match Command::new(if cfg!(target_os = "macos") { "/usr/bin/curl" } else { "curl" }).args(["-sL", "--max-time", "5", page_url]).output() {
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

