import ScreenSaver
import WebKit

// macOS-Bildschirmschoner: zeigt dieselbe Seite (saver.html) wie die Windows-Version.
// Bilder und Logo kommen zur Laufzeit von rockstargames.com, nichts davon steckt im Paket.

private let pages = ["https://www.rockstargames.com/VI/downloads",
                     "https://www.rockstargames.com/VI/downloads/screenshots"]
private let prefix = "/VI/_next/static/media/"

private func fetchFresh(baked: Data, bakedLogo: String) -> (Data, String) {
    var logo = bakedLogo
    var found: [String: String] = [:]
    let allowed = CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789/_.~%-")
    for p in pages {
        guard let url = URL(string: p) else { continue }
        var req = URLRequest(url: url); req.timeoutInterval = 5
        let sem = DispatchSemaphore(value: 0)
        var html = ""
        URLSession.shared.dataTask(with: req) { d, _, _ in
            if let d = d { html = String(decoding: d, as: UTF8.self) }
            sem.signal()
        }.resume()
        _ = sem.wait(timeout: .now() + 6)
        var rest = Substring(html)
        while let r = rest.range(of: prefix) {
            let tail = rest[r.lowerBound...]
            var end = tail.endIndex
            for i in tail.indices where !tail[i].unicodeScalars.allSatisfy({ allowed.contains($0) }) { end = i; break }
            let path = String(tail[..<end])
            let name = String(path.dropFirst(prefix.count)).split(separator: ".").first.map(String.init) ?? ""
            if path.hasSuffix(".png") && name == "vi" { logo = "https://www.rockstargames.com" + path }
            if path.hasSuffix(".jpg") { found[name] = "https://www.rockstargames.com" + path }
            rest = tail[tail.index(after: tail.startIndex)...]
            if end > tail.startIndex { rest = tail[end...] }
        }
    }
    guard var arr = (try? JSONSerialization.jsonObject(with: baked)) as? [[String: Any]] else { return (baked, logo) }
    for i in arr.indices { if let n = arr[i]["n"] as? String, let u = found[n] { arr[i]["u"] = u } }
    let out = (try? JSONSerialization.data(withJSONObject: arr)) ?? baked
    return (out, logo)
}

private final class Scheme: NSObject, WKURLSchemeHandler {
    var images = Data("[]".utf8)
    var logo = ""
    let ready = DispatchSemaphore(value: 0)
    let html: Data
    init(html: Data) { self.html = html }
    func webView(_ webView: WKWebView, start task: WKURLSchemeTask) {
        let path = task.request.url?.path ?? "/"
        var body = html, mime = "text/html"
        if path.hasSuffix("images.json") { body = images; mime = "application/json" }
        else if path.hasSuffix("logo.url") { body = Data(logo.utf8); mime = "text/plain" }
        let resp = URLResponse(url: task.request.url!, mimeType: mime, expectedContentLength: body.count, textEncodingName: "utf-8")
        task.didReceive(resp); task.didReceive(body); task.didFinish()
    }
    func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {}
}

@objc(GTA6SaverView)
final class GTA6SaverView: ScreenSaverView {
    private var web: WKWebView!

    override init?(frame: NSRect, isPreview: Bool) { super.init(frame: frame, isPreview: isPreview); setup() }
    required init?(coder: NSCoder) { super.init(coder: coder); setup() }

    private func setup() {
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.cgColor
        let b = Bundle(for: GTA6SaverView.self)
        let html = (try? Data(contentsOf: b.url(forResource: "saver", withExtension: "html")!)) ?? Data()
        let baked = (try? Data(contentsOf: b.url(forResource: "images", withExtension: "json")!)) ?? Data("[]".utf8)
        let bakedLogo = ((try? String(contentsOf: b.url(forResource: "logo", withExtension: "url")!)) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        let scheme = Scheme(html: html)
        scheme.images = baked; scheme.logo = bakedLogo
        let cfg = WKWebViewConfiguration()
        cfg.setURLSchemeHandler(scheme, forURLScheme: "gta")
        web = WKWebView(frame: bounds, configuration: cfg)
        web.autoresizingMask = [.width, .height]
        addSubview(web)
        DispatchQueue.global().async {
            let (img, logo) = fetchFresh(baked: baked, bakedLogo: bakedLogo)
            scheme.images = img; scheme.logo = logo
            DispatchQueue.main.async { self.web.load(URLRequest(url: URL(string: "gta://localhost/saver.html")!)) }
        }
    }

    override func hitTest(_ point: NSPoint) -> NSView? { self }
    override var hasConfigureSheet: Bool { false }
    override var configureSheet: NSWindow? { nil }
}
