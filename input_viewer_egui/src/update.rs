// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Auto-update, the egui counterpart of electron-updater.
//!
//! Same flow as the Electron main process: one check a few seconds after
//! start, then "Download / Later", then "Restart now / Later". A postponed
//! update installs on the next quit, as `autoInstallOnAppQuit` does there.
//!
//! Packaging and updates are Velopack. Releases share the
//! `LAB271/labs-input-viewer` repo with the Electron app, which constrains
//! where the feed can be found:
//!
//! - egui releases are tagged `egui-v*` and are **never** marked "latest".
//!   electron-updater resolves `/releases/latest`; an egui release there would
//!   strand every Electron install looking for a `latest.yml` it lacks.
//! - So Velopack's own `GithubSource` is not used. It scans only the ten most
//!   recent releases, and the Electron app has shipped three in one day -- a
//!   busy fortnight would push the last egui release out of that window and
//!   the updater would report nothing, silently. `EguiReleases` filters by tag
//!   prefix instead and hands the chosen release to `HttpSource`.

use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use velopack::sources::{HttpSource, UpdateSource};
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, VelopackAsset, VelopackAssetFeed};
use crate::filelog::{log_error, log_info, log_warn};

pub const REPO: &str = "LAB271/labs-input-viewer";
pub const TAG_PREFIX: &str = "egui-v";
const FIRST_CHECK_AFTER: Duration = Duration::from_secs(3);

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

/// The newest published egui release, from a page of `/releases`.
/// GitHub returns them newest first.
pub fn newest_egui_tag(releases: &[(String, bool, bool)]) -> Option<&str> {
    releases
        .iter()
        .find(|(tag, draft, pre)| !draft && !pre && tag.starts_with(TAG_PREFIX))
        .map(|(tag, _, _)| tag.as_str())
}

/// Update source that resolves the newest `egui-v*` release, then reads its
/// Velopack feed over plain HTTP.
#[derive(Default)]
struct EguiReleases {
    resolved: Mutex<Option<HttpSource>>,
}

impl EguiReleases {
    fn resolve(&self) -> Result<HttpSource, velopack::Error> {
        let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=100");
        let list: Vec<Release> = ureq::get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "input-viewer-egui")
            .call()
            .and_then(|mut r| r.body_mut().read_json())
            .map_err(|e| velopack::Error::Other(format!("listing releases: {e}")))?;
        let list: Vec<_> = list.into_iter().map(|r| (r.tag_name, r.draft, r.prerelease)).collect();
        let tag = newest_egui_tag(&list)
            .ok_or_else(|| velopack::Error::Other(format!("no {TAG_PREFIX}* release in the last 100")))?;
        Ok(HttpSource::new(format!("https://github.com/{REPO}/releases/download/{tag}/")))
    }
}

impl UpdateSource for EguiReleases {
    fn get_release_feed(&self, channel: &str, app: &velopack::bundle::Manifest, staged: &str) -> Result<VelopackAssetFeed, velopack::Error> {
        let source = self.resolve()?;
        let feed = source.get_release_feed(channel, app, staged);
        *self.resolved.lock().unwrap() = Some(source);
        feed
    }

    fn download_release_entry(&self, asset: &VelopackAsset, local_file: &Path, progress: Option<Sender<i16>>) -> Result<(), velopack::Error> {
        // The feed of release N lists N's full package and its deltas, all
        // uploaded to release N, so the source the feed came from serves them.
        let source = match self.resolved.lock().unwrap().clone() {
            Some(s) => s,
            None => self.resolve()?,
        };
        source.download_release_entry(asset, local_file, progress)
    }
}

/// What the UI should show.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Idle,
    Available(String),
    Downloading(i16),
    Ready(String),
    Failed(String),
}

enum Command {
    Download,
    RestartNow,
}

/// Background updater. All Velopack calls block on the network, so they run on
/// a worker thread and report back through `status`.
pub struct Updater {
    status: Arc<Mutex<Status>>,
    commands: Sender<Command>,
    /// Set once an update is downloaded, so `on_exit` can apply it.
    pending: Arc<Mutex<Option<(UpdateManager, VelopackAsset)>>>,
}

impl Updater {
    /// None when the app was not installed by Velopack (`cargo run`, a copied
    /// binary): there is nothing to update in place, as with `isDev` in Electron.
    pub fn start(ctx: eframe::egui::Context) -> Option<Self> {
        let manager = match UpdateManager::new(EguiReleases::default(), None, None) {
            Ok(m) => m,
            Err(e) => {
                log_info!("[AutoUpdater] disabled: {e}");
                return None;
            }
        };
        log_info!("[AutoUpdater] installed version {}", manager.get_current_version_as_string());
        let status = Arc::new(Mutex::new(Status::Idle));
        let pending = Arc::new(Mutex::new(None));
        let (tx, rx) = mpsc::channel();
        let (s, p) = (status.clone(), pending.clone());
        std::thread::Builder::new()
            .name("updater".into())
            .spawn(move || run(manager, rx, s, p, ctx))
            .ok()?;
        Some(Self { status, commands: tx, pending })
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    pub fn dismiss(&self) {
        let mut s = self.status.lock().unwrap();
        // "Later" on a downloaded update keeps it pending for on_exit.
        if !matches!(*s, Status::Downloading(_)) {
            *s = Status::Idle;
        }
    }

    pub fn download(&self) {
        let _ = self.commands.send(Command::Download);
    }

    pub fn restart_now(&self) {
        let _ = self.commands.send(Command::RestartNow);
    }

    /// Install a postponed update as the app quits.
    pub fn on_exit(&self) {
        if let Some((manager, asset)) = self.pending.lock().unwrap().take() {
            log_info!("[AutoUpdater] installing {} on quit", asset.Version);
            if let Err(e) = manager.wait_exit_then_apply_updates(&asset, true, false, Vec::<String>::new()) {
                log_error!("[AutoUpdater] could not schedule install: {e}");
            }
        }
    }
}

fn run(
    manager: UpdateManager,
    rx: Receiver<Command>,
    status: Arc<Mutex<Status>>,
    pending: Arc<Mutex<Option<(UpdateManager, VelopackAsset)>>>,
    ctx: eframe::egui::Context,
) {
    let set = |s: Status| {
        *status.lock().unwrap() = s;
        ctx.request_repaint();
    };
    std::thread::sleep(FIRST_CHECK_AFTER);
    log_info!("[AutoUpdater] checking for update...");
    let info: Box<UpdateInfo> = match manager.check_for_updates() {
        Ok(UpdateCheck::UpdateAvailable(info)) => info,
        Ok(_) => {
            log_info!("[AutoUpdater] up to date");
            return;
        }
        Err(e) => {
            // Logged, not shown: an offline wall should not greet the room
            // with an error dialog every launch.
            log_warn!("[AutoUpdater] check failed: {e}");
            return;
        }
    };
    let version = info.TargetFullRelease.Version.clone();
    log_info!("[AutoUpdater] update available: {version}");
    set(Status::Available(version.clone()));

    while let Ok(cmd) = rx.recv() {
        match cmd {
            Command::Download => {
                let (ptx, prx) = mpsc::channel::<i16>();
                let progress_status = status.clone();
                let progress_ctx = ctx.clone();
                std::thread::spawn(move || {
                    for pct in prx {
                        *progress_status.lock().unwrap() = Status::Downloading(pct);
                        progress_ctx.request_repaint();
                    }
                });
                set(Status::Downloading(0));
                match manager.download_updates(&info, Some(ptx)) {
                    Ok(()) => {
                        log_info!("[AutoUpdater] downloaded {version}");
                        *pending.lock().unwrap() = Some((manager.clone(), info.TargetFullRelease.clone()));
                        set(Status::Ready(version.clone()));
                    }
                    Err(e) => {
                        log_error!("[AutoUpdater] download failed: {e}");
                        set(Status::Failed(e.to_string()));
                    }
                }
            }
            Command::RestartNow => {
                pending.lock().unwrap().take();
                if let Err(e) = manager.apply_updates_and_restart(&info.TargetFullRelease) {
                    log_error!("[AutoUpdater] restart failed: {e}");
                    set(Status::Failed(e.to_string()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(tag: &str, draft: bool, pre: bool) -> (String, bool, bool) {
        (tag.to_owned(), draft, pre)
    }

    #[test]
    fn skips_electron_drafts_and_prereleases() {
        let list = [
            r("v3.5.0", false, false),
            r("egui-v0.3.0", true, false),
            r("egui-v0.2.1", false, true),
            r("v3.4.1", false, false),
            r("egui-v0.2.0", false, false),
            r("egui-v0.1.0", false, false),
        ];
        assert_eq!(newest_egui_tag(&list), Some("egui-v0.2.0"));
    }

    #[test]
    fn none_when_only_electron_releases() {
        assert_eq!(newest_egui_tag(&[r("v3.4.1", false, false)]), None);
    }
}
