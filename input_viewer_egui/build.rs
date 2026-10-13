// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Embeds the app icon in the Windows executable, so Explorer, the taskbar
//! pin and the shortcuts Velopack creates show it. `vpk --icon` only sets the
//! installer's icon.

fn main() {
    println!("cargo:rerun-if-changed=packaging/icon/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("packaging/icon/icon.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the icon: {e}");
        }
    }
}
