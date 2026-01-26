#include <WiFi.h>
#include <WebServer.h>
#include <ESPmDNS.h>

#include "USB.h"
#include "USBHIDKeyboard.h"

// ====== WIFI ======
const char* WIFI_SSID = "YOUR_WIFI_SSID";
const char* WIFI_PASS = "YOUR_WIFI_PASSWORD";

// ====== HOSTNAME ======
static const char* HOSTNAME = "space_keyboard";

// ====== AUTH ======
static const char* API_KEY_HEADER = "X-API-Key";
// Change this to your secret:
static const char* API_KEY_VALUE  = "CHANGE_ME_SUPER_SECRET";

// ====== HTTP & HID ======
WebServer server(80);
USBHIDKeyboard Keyboard;

// Public UI HTML (no auth required to load)
static const char INDEX_HTML[] PROGMEM = R"HTML(
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>space_keyboard</title>
  <style>
    :root { --r: 18px; }
    body { font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Arial;
           margin: 0; background: #0b1020; color: #e8ecff; }
    .wrap { max-width: 720px; margin: 0 auto; padding: 28px 18px 40px; }
    .card { background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.10);
            border-radius: var(--r); padding: 18px; box-shadow: 0 10px 30px rgba(0,0,0,0.25); }
    h1 { margin: 0 0 6px; font-size: 22px; letter-spacing: 0.2px; }
    .sub { margin: 0 0 16px; opacity: 0.85; font-size: 14px; }
    .row { display: flex; gap: 12px; flex-wrap: wrap; align-items: center; }
    .grow { flex: 1; min-width: 220px; }
    input { width: 100%; padding: 12px 12px; border-radius: 12px;
            border: 1px solid rgba(255,255,255,0.14); background: rgba(0,0,0,0.25);
            color: #e8ecff; outline: none; }
    input:focus { border-color: rgba(130,180,255,0.8); box-shadow: 0 0 0 3px rgba(80,140,255,0.25); }
    button { padding: 12px 14px; border-radius: 14px; border: 0;
             background: rgba(130,180,255,0.95); color: #071028;
             font-weight: 700; cursor: pointer; }
    button.secondary { background: rgba(255,255,255,0.10); color: #e8ecff; border: 1px solid rgba(255,255,255,0.14); }
    button:active { transform: translateY(1px); }
    .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-top: 14px; }
    .big { padding: 18px 14px; font-size: 16px; border-radius: 18px; }
    .status { margin-top: 14px; font-size: 13px; opacity: 0.95; line-height: 1.35; }
    .pill { display: inline-block; padding: 4px 10px; border-radius: 999px; background: rgba(255,255,255,0.10);
            border: 1px solid rgba(255,255,255,0.12); font-size: 12px; }
    .ok { color: #7dffb2; }
    .bad { color: #ff8b8b; }
    code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; font-size: 12px; }
  </style>
</head>
<body>
  <div class="wrap">
    <div class="card">
      <h1>space_keyboard</h1>
      <p class="sub">Public UI. Protected actions require header auth: <code>X-API-Key</code>.</p>

      <div class="row">
        <div class="grow">
          <input id="key" type="password" placeholder="Enter API key..." />
        </div>
        <button class="secondary" onclick="saveKey()">Save</button>
        <button class="secondary" onclick="clearKey()">Clear</button>
      </div>

      <div class="grid">
        <button class="big" onclick="hit('/left')">⬅ Prev</button>
        <button class="big" onclick="hit('/right')">Next ➡</button>
      </div>

      <div class="status" id="status">
        <span class="pill">Status</span> Waiting…
      </div>

      <div class="status" style="opacity:.75; margin-top:10px">
        Curl example: <code>curl -H "X-API-Key: …" http://space_keyboard.local/left</code>
      </div>
    </div>
  </div>

<script>
  const KEY_NAME = "space_keyboard_api_key";
  const statusEl = document.getElementById("status");
  const keyEl = document.getElementById("key");

  function setStatus(msg, ok=null) {
    const cls = ok === null ? "" : (ok ? "ok" : "bad");
    statusEl.innerHTML = `<span class="pill">Status</span> <span class="${cls}">${msg}</span>`;
  }

  function loadKey() {
    const v = localStorage.getItem(KEY_NAME) || "";
    keyEl.value = v;
    if (v) setStatus("API key loaded from browser storage.", true);
  }

  function saveKey() {
    localStorage.setItem(KEY_NAME, keyEl.value || "");
    setStatus(keyEl.value ? "API key saved." : "Cleared (empty key).", !!keyEl.value);
  }

  function clearKey() {
    localStorage.removeItem(KEY_NAME);
    keyEl.value = "";
    setStatus("API key cleared.", true);
  }

  async function hit(path) {
    const apiKey = keyEl.value || localStorage.getItem(KEY_NAME) || "";
    if (!apiKey) {
      setStatus("No API key set. Enter it first.", false);
      return;
    }

    setStatus(`Sending ${path}...`);
    try {
      const res = await fetch(path, { headers: { "X-API-Key": apiKey } });
      const text = await res.text();

      if (!res.ok) {
        setStatus(`Error ${res.status}: ${text}`, false);
        return;
      }

      setStatus(`OK: ${path} (response: ${text})`, true);
    } catch (e) {
      setStatus(`Fetch failed: ${e}`, false);
    }
  }

  loadKey();
</script>
</body>
</html>
)HTML";

bool checkAuth() {
  if (!server.hasHeader(API_KEY_HEADER)) return false;
  return server.header(API_KEY_HEADER) == API_KEY_VALUE;
}

void sendUnauthorized() {
  server.sendHeader("WWW-Authenticate", "API key required");
  server.send(401, "text/plain", "Unauthorized: missing or invalid X-API-Key");
}

void tapKey(uint8_t key) {
  Keyboard.press(key);
  delay(15);
  Keyboard.release(key);
  delay(15);
}

// ====== ROUTES ======
void handleRoot() {
  // Public UI (no auth required)
  server.send(200, "text/html; charset=utf-8", FPSTR(INDEX_HTML));
}

void handleLeft() {
  if (!checkAuth()) return sendUnauthorized();
  tapKey(KEY_PAGE_UP);
  server.send(200, "application/json", "{\"ok\":true,\"sent\":\"pageup\"}");
}

void handleRight() {
  if (!checkAuth()) return sendUnauthorized();
  tapKey(KEY_PAGE_DOWN);
  server.send(200, "application/json", "{\"ok\":true,\"sent\":\"pagedown\"}");
}

void handleNotFound() {
  // Public 404 (no auth required)
  server.send(404, "text/plain", "Not found");
}

void setup() {
  Serial.begin(115200);
  delay(200);
  Serial.println("BOOT ESP32-S3 space_keyboard");

  // USB HID keyboard
  USB.begin();
  Keyboard.begin();

  // WiFi
  WiFi.mode(WIFI_STA);
  WiFi.setHostname(HOSTNAME);
  WiFi.begin(WIFI_SSID, WIFI_PASS);
  while (WiFi.status() != WL_CONNECTED) delay(200);

  // mDNS
  if (MDNS.begin(HOSTNAME)) {
    MDNS.addService("http", "tcp", 80);
  }

  // Collect headers so server.header() can read X-API-Key
  const char* headerKeys[] = { API_KEY_HEADER };
  server.collectHeaders(headerKeys, 1);

  // Routes
  server.on("/", HTTP_GET, handleRoot);
  server.on("/left", HTTP_GET, handleLeft);
  server.on("/right", HTTP_GET, handleRight);
  server.onNotFound(handleNotFound);

  server.begin();

  Serial.print("READY: http://");
  Serial.print(HOSTNAME);
  Serial.println(".local/");
}

void loop() {
  server.handleClient();
  delay(1);
}
