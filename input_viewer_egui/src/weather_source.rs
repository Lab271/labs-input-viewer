// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Live weather readings for the Weather screensaver (#101, ADR 0027), from
//! `weather-source.js`.
//!
//! The poller is deliberately **not** part of the saver. A saver is built and
//! dropped on every activation; a timer it owned would either leak into every
//! later saver or die with it, and a saver that fetched for itself would make
//! the no-signal screen wait on HTTP. So a thread polls here for the app's
//! lifetime and keeps the last good reading; the saver only reads it, and
//! declines the random rotation while there is none (`is_available`).
//!
//! Open-Meteo: no key, no account, nothing secret to store. The configured
//! coordinates go to a third party on every poll, so they are rounded to two
//! decimals (~1 km) first, and the feature is off unless `weatherEnabled`.
//!
//! The app's instance is a module singleton (`install` / `current_reading`),
//! as in Electron: the saver is built by the registry, which knows nothing
//! about weather (open question D11). The poller itself is an ordinary value
//! with its clock and fetch injected, which is what the tests drive.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::filelog::{log_info, log_warn};

/// Open-Meteo's current-conditions endpoint. No key, no account.
const ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";

const FIELDS: &[&str] = &[
    "temperature_2m",
    "precipitation",
    "wind_speed_10m",
    "wind_direction_10m",
    "cloud_cover",
    "weather_code",
    "is_day",
];

/// Open-Meteo refreshes about every 15 minutes; faster is waste, and rude to a
/// free service.
pub const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Backoff on repeated failure: doubles from one minute to one hour and stops
/// growing. One counter, so nothing accumulates over weeks offline.
const BACKOFF_MIN: Duration = Duration::from_secs(60);
const BACKOFF_MAX: Duration = Duration::from_secs(60 * 60);

/// Well inside the poll interval, so a connection that opens and never
/// answers cannot hold the poll slot.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Older than this, the readout shows the age, so stale data is not mistaken
/// for calm weather.
pub const STALE_AFTER: Duration = Duration::from_secs(30 * 60);

/// ...and past this the reading is dropped: a wall confidently showing
/// yesterday's rain is worse than a different screensaver.
pub const EXPIRE_AFTER: Duration = Duration::from_secs(12 * 60 * 60);

/// One observation, normalised. `age` and `stale` are filled in when it is
/// read out of the cache.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    /// Observation time as reported, for diagnostics only. Age comes from
    /// our own clock, so a wall with a skewed clock is still consistent.
    pub observed_at: Option<String>,
    pub temperature_c: f64,
    /// Rate, converted from the API's mm per `interval`.
    pub precipitation_mm_h: f64,
    pub wind_speed_kmh: f64,
    pub wind_direction_deg: f64,
    pub cloud_cover_pct: f64,
    pub weather_code: f64,
    pub is_day: bool,
    pub age: Duration,
    pub stale: bool,
    /// The saver's FALLBACK, not an observation; the readout says NO DATA.
    pub synthetic: bool,
}

/// Read at each poll, so toggling the setting needs no restart.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Config {
    pub enabled: bool,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Diagnostics. A fixed shape however long an outage lasts.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub running: bool,
    pub has_reading: bool,
    pub age: Option<Duration>,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub next_delay: Duration,
}

/// Round a coordinate before it leaves the process: two decimals is ~1 km,
/// finer than any weather model's grid. `Math.round`, so halves go up.
pub fn coarsen(n: f64) -> f64 {
    (n * 100.0 + 0.5).floor() / 100.0
}

fn request_url(latitude: f64, longitude: f64) -> String {
    format!("{ENDPOINT}?latitude={}&longitude={}&current={}", coarsen(latitude), coarsen(longitude), FIELDS.join("%2C"))
}

/// Validate and normalise the `current` block of a response. None for
/// anything unexpected: "the JSON was not what we expected" and "the network
/// is down" both mean "no reading".
pub fn parse_reading(json: &Value) -> Option<Reading> {
    let c = json.get("current")?.as_object()?;
    let num = |k: &str| c.get(k).and_then(Value::as_f64).filter(|v| v.is_finite());
    // A partial response is a failed one: these three drive the whole scene,
    // and a reading without one would render something confidently wrong.
    let temperature_c = num("temperature_2m")?;
    let wind = num("wind_speed_10m")?;
    let cloud = num("cloud_cover")?;
    let interval = num("interval").filter(|&v| v != 0.0).unwrap_or(900.0);
    let is_day = c.get("is_day").is_some_and(|v| v.as_f64() == Some(1.0) || v.as_bool() == Some(true));
    Some(Reading {
        observed_at: c.get("time").and_then(Value::as_str).map(str::to_owned),
        temperature_c,
        precipitation_mm_h: num("precipitation").map_or(0.0, |mm| mm * 3600.0 / interval),
        wind_speed_kmh: wind.max(0.0),
        wind_direction_deg: num("wind_direction_10m").unwrap_or(0.0),
        cloud_cover_pct: cloud.clamp(0.0, 100.0),
        weather_code: num("weather_code").unwrap_or(0.0),
        is_day,
        age: Duration::ZERO,
        stale: false,
        synthetic: false,
    })
}

fn next_delay(consecutive_failures: u32) -> Duration {
    if consecutive_failures == 0 {
        return POLL_INTERVAL;
    }
    let grown = BACKOFF_MIN.saturating_mul(1 << (consecutive_failures - 1).min(16));
    grown.min(BACKOFF_MAX)
}

pub type Fetch = Box<dyn Fn(&str) -> Result<Value, String> + Send + Sync>;
pub type Clock = Box<dyn Fn() -> Duration + Send + Sync>;
pub type GetConfig = Box<dyn Fn() -> Config + Send + Sync>;

#[derive(Default)]
struct Cache {
    reading: Option<Reading>,
    /// Our clock when it arrived.
    fetched_at: Duration,
    consecutive_failures: u32,
    last_error: Option<String>,
    in_flight: bool,
}

struct Inner {
    get_config: GetConfig,
    fetch: Fetch,
    now: Clock,
    cache: Mutex<Cache>,
    /// Polling threads alive, for the timer-hygiene tests.
    threads: AtomicUsize,
}

impl Inner {
    fn poll_once(&self) {
        let cfg = (self.get_config)();
        // Not configured is not a failure: no backoff, no error. Checked
        // again next tick in case settings changed.
        let (true, Some(lat), Some(lon)) = (cfg.enabled, cfg.latitude, cfg.longitude) else {
            self.cache.lock().unwrap().last_error = None;
            return;
        };
        {
            let mut c = self.cache.lock().unwrap();
            // No overlap: a slow request and a manual poll must not both run.
            if c.in_flight {
                return;
            }
            c.in_flight = true;
        }
        let result = (self.fetch)(&request_url(lat, lon))
            .and_then(|json| parse_reading(&json).ok_or_else(|| "unexpected response shape".to_owned()));
        let mut c = self.cache.lock().unwrap();
        c.in_flight = false;
        match result {
            Ok(reading) => {
                if c.reading.is_none() || c.consecutive_failures > 0 {
                    log_info!("[Weather] reading: {:.1}C, code {}, cloud {}%", reading.temperature_c, reading.weather_code, reading.cloud_cover_pct);
                }
                c.reading = Some(reading);
                c.fetched_at = (self.now)();
                c.consecutive_failures = 0;
                c.last_error = None;
            }
            Err(e) => {
                // Swallowed on purpose: the cached reading (if any) keeps
                // being shown, and the saver drops out once it expires.
                c.consecutive_failures += 1;
                log_warn!("[Weather] poll failed ({}x): {e}", c.consecutive_failures);
                c.last_error = Some(e);
            }
        }
    }

    fn reading(&self) -> Option<Reading> {
        let c = self.cache.lock().unwrap();
        let age = (self.now)().saturating_sub(c.fetched_at);
        let r = c.reading.as_ref().filter(|_| age <= EXPIRE_AFTER)?;
        Some(Reading { age, stale: age > STALE_AFTER, ..r.clone() })
    }
}

/// A poller. Nothing happens until `start`, and a poll without coordinates
/// or with the feature off makes no request.
pub struct Source {
    inner: Arc<Inner>,
    stop: Option<mpsc::Sender<()>>,
}

impl Source {
    pub fn new(get_config: GetConfig, fetch: Fetch, now: Clock) -> Self {
        let inner = Inner { get_config, fetch, now, cache: Mutex::default(), threads: AtomicUsize::new(0) };
        Self { inner: Arc::new(inner), stop: None }
    }

    /// Begin polling: once straight away, then on the interval or the
    /// backoff. Safe to call twice.
    pub fn start(&mut self) {
        if self.stop.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel::<()>();
        self.stop = Some(tx);
        let inner = self.inner.clone();
        inner.threads.fetch_add(1, Ordering::SeqCst);
        std::thread::Builder::new().name("weather".into()).spawn(move || {
            loop {
                inner.poll_once();
                let delay = next_delay(inner.cache.lock().unwrap().consecutive_failures);
                if !matches!(rx.recv_timeout(delay), Err(RecvTimeoutError::Timeout)) {
                    break;
                }
            }
            inner.threads.fetch_sub(1, Ordering::SeqCst);
        }).expect("spawning the weather thread");
    }

    /// Stop polling. The thread ends as soon as it is not mid-request; the
    /// cached reading is kept.
    pub fn stop(&mut self) {
        self.stop = None;
    }
}

/// What Electron's `getStatus` and `pollNow` exposed for a settings "test"
/// button. There is no Settings pane for weather in either app yet, so only
/// the tests use them.
#[cfg(test)]
impl Source {
    /// Poll now, on the calling thread.
    pub fn poll_now(&self) {
        self.inner.poll_once();
    }

    /// The reading with its age, or None when there is none or it expired.
    pub fn reading(&self) -> Option<Reading> {
        self.inner.reading()
    }

    pub fn status(&self) -> Status {
        let c = self.inner.cache.lock().unwrap();
        Status {
            running: self.stop.is_some(),
            has_reading: c.reading.is_some(),
            age: c.reading.as_ref().map(|_| (self.inner.now)().saturating_sub(c.fetched_at)),
            consecutive_failures: c.consecutive_failures,
            last_error: c.last_error.clone(),
            next_delay: next_delay(c.consecutive_failures),
        }
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A blocking GET with the request timeout. An HTTP error status is a
/// failure like any other.
fn http_fetch() -> Fetch {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    Box::new(move |url| {
        let mut res = agent.get(url).call().map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("HTTP {}", res.status().as_u16()));
        }
        res.body_mut().read_json::<Value>().map_err(|e| e.to_string())
    })
}

/// Monotonic, so a clock change on the wall cannot make a reading look new
/// or ancient.
fn monotonic() -> Clock {
    let origin = Instant::now();
    Box::new(move || origin.elapsed())
}

static SHARED: Mutex<Option<Arc<Inner>>> = Mutex::new(None);
static INJECTED: Mutex<Option<Reading>> = Mutex::new(None);

/// Start the app's poller and make it the one `current_reading` reads.
/// Dropping the returned value stops it.
pub fn install(get_config: GetConfig) -> Source {
    let mut source = Source::new(get_config, http_fetch(), monotonic());
    source.start();
    *SHARED.lock().unwrap() = Some(source.inner.clone());
    source
}

/// The reading the saver should animate, or None -- feature off, no
/// coordinates, never reached the network, or expired, all alike.
pub fn current_reading() -> Option<Reading> {
    if let Some(r) = INJECTED.lock().unwrap().clone() {
        return Some(r);
    }
    SHARED.lock().unwrap().as_ref().and_then(|s| s.reading())
}

/// Pin a reading without the network (`--weather-state`), so rain, snow, fog
/// and night can be reviewed without waiting for them. None hands control
/// back to the poller.
pub fn inject(reading: Option<Reading>) {
    *INJECTED.lock().unwrap() = reading;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicU64;

    fn current() -> Value {
        json!({
            "time": "2026-08-11T11:45",
            "interval": 900,
            "temperature_2m": 21.7,
            "precipitation": 0.5,
            "wind_speed_10m": 15.1,
            "wind_direction_10m": 58,
            "cloud_cover": 48,
            "weather_code": 61,
            "is_day": 1
        })
    }

    fn ok() -> Result<Value, String> {
        Ok(json!({ "current": current() }))
    }

    fn config() -> Config {
        Config { enabled: true, latitude: Some(52.3676), longitude: Some(4.9041) }
    }

    /// A clock the test moves by hand, and a fetch that records its URLs.
    struct Harness {
        clock: Arc<AtomicU64>,
        urls: Arc<Mutex<Vec<String>>>,
    }

    impl Harness {
        fn advance(&self, d: Duration) {
            self.clock.fetch_add(d.as_millis() as u64, Ordering::SeqCst);
        }
        fn calls(&self) -> usize {
            self.urls.lock().unwrap().len()
        }
    }

    fn build_with(cfg: Config, f: impl Fn() -> Result<Value, String> + Send + Sync + 'static) -> (Source, Harness) {
        let clock = Arc::new(AtomicU64::new(1_700_000_000_000));
        let urls = Arc::new(Mutex::new(Vec::new()));
        let (c, u) = (clock.clone(), urls.clone());
        let src = Source::new(
            Box::new(move || cfg),
            Box::new(move |url| {
                u.lock().unwrap().push(url.to_owned());
                f()
            }),
            Box::new(move || Duration::from_millis(c.load(Ordering::SeqCst))),
        );
        (src, Harness { clock, urls })
    }

    fn build(f: impl Fn() -> Result<Value, String> + Send + Sync + 'static) -> (Source, Harness) {
        build_with(config(), f)
    }

    fn parse_current(c: Value) -> Option<Reading> {
        parse_reading(&json!({ "current": c }))
    }

    fn wait_until(mut cond: impl FnMut() -> bool) {
        for _ in 0..500 {
            if cond() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("timed out");
    }

    // ---------------------------------------------------------- parseReading

    #[test]
    fn converts_precipitation_from_mm_per_interval_to_mm_h() {
        // 0.5 mm per 900 s is 2 mm/h.
        assert!((parse_current(current()).unwrap().precipitation_mm_h - 2.0).abs() < 1e-9);
    }

    #[test]
    fn defaults_a_missing_interval_to_900s() {
        let mut c = current();
        c.as_object_mut().unwrap().remove("interval");
        let r = parse_current(c).unwrap();
        assert!((r.precipitation_mm_h - 2.0).abs() < 1e-9);
    }

    #[test]
    fn treats_a_partial_response_as_no_reading() {
        for missing in ["temperature_2m", "wind_speed_10m", "cloud_cover"] {
            let mut c = current();
            c.as_object_mut().unwrap().remove(missing);
            assert_eq!(parse_current(c), None, "{missing}");
        }
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        let r = parse_current(json!({ "temperature_2m": 3, "wind_speed_10m": 0, "cloud_cover": 100 })).unwrap();
        assert_eq!(r.precipitation_mm_h, 0.0);
        assert_eq!(r.wind_direction_deg, 0.0);
        assert!(!r.is_day);
    }

    #[test]
    fn rejects_junk_instead_of_panicking() {
        for junk in [
            Value::Null,
            json!({}),
            json!({ "current": null }),
            json!({ "current": "sunny" }),
            json!({ "current": { "temperature_2m": "NaN", "wind_speed_10m": 1, "cloud_cover": 1 } }),
        ] {
            assert_eq!(parse_reading(&junk), None, "{junk}");
        }
    }

    #[test]
    fn clamps_cloud_cover_into_range() {
        let with = |v: i32| {
            let mut c = current();
            c["cloud_cover"] = json!(v);
            parse_current(c).unwrap().cloud_cover_pct
        };
        assert_eq!(with(140), 100.0);
        assert_eq!(with(-5), 0.0);
    }

    #[test]
    fn coarsens_coordinates_to_about_a_km() {
        assert_eq!(coarsen(52.36757), 52.37);
        assert_eq!(coarsen(4.90414), 4.9);
        assert_eq!(coarsen(-0.126), -0.13);
    }

    // ------------------------------------------------------------ happy path

    #[test]
    fn caches_a_reading_and_reports_it() {
        let (src, _) = build(ok);
        src.poll_now();
        let r = src.reading().unwrap();
        assert_eq!(r.temperature_c, 21.7);
        assert!(!r.stale);
        assert_eq!(src.status().consecutive_failures, 0);
    }

    #[test]
    fn sends_only_coarsened_coordinates() {
        let (src, h) = build(ok);
        src.poll_now();
        let url = h.urls.lock().unwrap()[0].clone();
        assert!(url.contains("latitude=52.37"), "{url}");
        assert!(url.contains("longitude=4.9"), "{url}");
        assert!(!url.contains("52.3676"), "{url}");
        assert!(url.contains("current=temperature_2m%2Cprecipitation%2C"), "{url}");
    }

    #[test]
    fn polls_on_start_then_waits_the_interval_once_healthy() {
        let (mut src, h) = build(ok);
        src.start();
        wait_until(|| h.calls() == 1);
        assert_eq!(src.status().next_delay, POLL_INTERVAL);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(h.calls(), 1);
        src.stop();
    }

    // ---------------------------------------------------------- not configured

    #[test]
    fn not_configured_makes_no_request_and_records_no_failure() {
        for cfg in [
            Config { enabled: false, ..config() },
            Config { latitude: None, ..config() },
            Config { longitude: None, ..config() },
        ] {
            let (src, h) = build_with(cfg, ok);
            src.poll_now();
            assert_eq!(h.calls(), 0);
            let s = src.status();
            assert_eq!((s.consecutive_failures, s.last_error), (0, None));
            assert_eq!(src.reading(), None);
        }
    }

    // ----------------------------------------------------------- failure paths

    #[test]
    fn every_failure_is_counted_and_keeps_no_reading() {
        type Case = (&'static str, fn() -> Result<Value, String>);
        let cases: [Case; 7] = [
            ("no route to host", || Err("Failed to fetch".into())),
            ("connection timeout", || Err("timeout: global".into())),
            ("HTTP 429", || Err("HTTP 429".into())),
            ("HTTP 500", || Err("HTTP 500".into())),
            ("malformed JSON", || Err("expected value at line 1".into())),
            ("partial JSON", || Ok(json!({ "current": { "temperature_2m": 9 } }))),
            ("empty body", || Ok(Value::Null)),
        ];
        for (name, f) in cases {
            let (src, _) = build(f);
            src.poll_now();
            let s = src.status();
            assert_eq!(s.consecutive_failures, 1, "{name}");
            assert!(s.last_error.is_some(), "{name}");
            assert_eq!(src.reading(), None, "{name}");
        }
    }

    #[test]
    fn keeps_the_last_good_reading_through_a_network_blip() {
        let up = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let u = up.clone();
        let (src, h) = build(move || if u.load(Ordering::SeqCst) { ok() } else { Err("down".into()) });
        src.poll_now();
        up.store(false, Ordering::SeqCst);
        h.advance(Duration::from_secs(60));
        src.poll_now();
        assert_eq!(src.reading().unwrap().temperature_c, 21.7);
        assert_eq!(src.status().consecutive_failures, 1);
    }

    #[test]
    fn backs_off_geometrically_and_caps_at_an_hour() {
        let (src, _) = build(|| Err("down".into()));
        let delays: Vec<Duration> = (0..8).map(|_| {
            src.poll_now();
            src.status().next_delay
        }).collect();
        assert_eq!(&delays[..3], [60, 120, 240].map(Duration::from_secs));
        assert!(delays.windows(2).all(|w| w[1] >= w[0]));
        assert_eq!(delays.iter().max(), Some(&BACKOFF_MAX));
    }

    #[test]
    fn recovers_after_hours_offline_and_resets_the_backoff() {
        let up = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let u = up.clone();
        let (src, h) = build(move || if u.load(Ordering::SeqCst) { ok() } else { Err("down".into()) });
        for _ in 0..20 {
            h.advance(Duration::from_secs(3600));
            src.poll_now();
        }
        assert_eq!(src.status().consecutive_failures, 20);
        up.store(true, Ordering::SeqCst);
        src.poll_now();
        assert_eq!(src.status().consecutive_failures, 0);
        assert_eq!(src.status().next_delay, POLL_INTERVAL);
        assert!(src.reading().is_some());
    }

    #[test]
    fn a_long_outage_is_one_counter() {
        let (src, _) = build(|| Err("down".into()));
        for _ in 0..500 {
            src.poll_now();
        }
        assert_eq!(src.status().consecutive_failures, 500);
        assert_eq!(src.status().next_delay, BACKOFF_MAX);
    }

    // ---------------------------------------------------- staleness and expiry

    #[test]
    fn flags_a_stale_reading_but_keeps_using_it() {
        let (src, h) = build(ok);
        src.poll_now();
        h.advance(STALE_AFTER + Duration::from_secs(1));
        let r = src.reading().unwrap();
        assert!(r.stale);
        assert!(r.age > STALE_AFTER);
    }

    #[test]
    fn stops_offering_a_reading_once_it_has_expired() {
        let (src, h) = build(ok);
        src.poll_now();
        h.advance(EXPIRE_AFTER + Duration::from_secs(1));
        assert_eq!(src.reading(), None);
    }

    // ----------------------------------------------------------- thread hygiene

    #[test]
    fn leaves_no_thread_behind_after_stop() {
        let (mut src, h) = build(ok);
        src.start();
        wait_until(|| h.calls() == 1);
        src.stop();
        wait_until(|| src.inner.threads.load(Ordering::SeqCst) == 0);
        assert!(!src.status().running);
    }

    #[test]
    fn repeated_start_stop_does_not_pile_up_threads() {
        let (mut src, _) = build(ok);
        for _ in 0..10 {
            src.start();
            src.stop();
        }
        wait_until(|| src.inner.threads.load(Ordering::SeqCst) == 0);
    }

    #[test]
    fn start_twice_does_not_double_the_polling() {
        let (mut src, h) = build(ok);
        src.start();
        src.start();
        wait_until(|| h.calls() >= 1);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(h.calls(), 1);
        assert_eq!(src.inner.threads.load(Ordering::SeqCst), 1);
        src.stop();
    }

    #[test]
    fn does_not_run_two_requests_at_once() {
        let concurrent = Arc::new(AtomicUsize::new(0));
        let max = Arc::new(AtomicUsize::new(0));
        let (c, m) = (concurrent.clone(), max.clone());
        let (src, _) = build(move || {
            let now = c.fetch_add(1, Ordering::SeqCst) + 1;
            m.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(50));
            c.fetch_sub(1, Ordering::SeqCst);
            ok()
        });
        let src = Arc::new(src);
        let handles: Vec<_> = (0..2).map(|_| {
            let s = src.clone();
            std::thread::spawn(move || s.poll_now())
        }).collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(max.load(Ordering::SeqCst), 1);
    }
}
