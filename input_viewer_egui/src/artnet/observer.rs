// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The frame observer: gl-base.js `observeFrames`, the GL side of Art-Net.
//!
//! Two facts from Electron shape this, both measured on the wall:
//!
//! - **Registered only while Art-Net is enabled.** The observer was once
//!   registered unconditionally, "a no-op while disabled" -- true of the
//!   callback, false of the readback in front of it. Two split-flap boards
//!   then did 64 synchronous `readPixels` per frame on the wall's RTX PRO
//!   4000 and ran at 1.4 fps against 118.9. With no sink registered,
//!   [`observe`] is one atomic load.
//! - **At most once a second.** The only consumer sends at 1 Hz.
//!
//! What the port does differently: Electron read the 8x4 grid as 32 separate
//! synchronous `readPixels` from WebGL's RGB8 default framebuffer, because a
//! blit from it into an RGBA8 target is INVALID_OPERATION there. Desktop GL
//! has no such restriction, and the shader savers already finish in their
//! own RGBA8 target. So the frame is *blitted* down to 64x32 (NEAREST), read
//! with one `readPixels` into a pixel-pack buffer, and fenced; a later frame
//! collects the bytes once the fence has signalled, so the render thread
//! never waits for the GPU to finish the frame.
//!
//! One scaled blit, not 32 tile-sized ones, and that is measured: on an M3
//! Pro (Apple's GL over Metal) 32 8x8 blits cost 9-20 ms of render-thread
//! time per read -- each is its own pass -- against 40-200 us for one blit
//! of the whole 2560x1440 frame, and 0.1-0.3 ms for the whole issue
//! including the PBO read. The consequence: each 8x8 tile now samples its
//! whole grid cell on an 8x8 lattice of single pixels, where Electron read
//! an 8x8 block at the cell's centre. Same layout, same maths, wider
//! coverage; still point samples rather than an average, so a thin bright
//! filament keeps its saturation instead of being blurred into the dark
//! around it (averaging is what turns everything grey).
//!
//! The bytes are handed on in gl-base.js's exact layout -- tile by tile, `ty`
//! outer, each tile's rows bottom-up -- so the colour maths and its tests are
//! unchanged.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

use eframe::glow::{self, HasContext};

use super::colour::{GRID, SampleGrid};
use super::worker::Msg;
use crate::filelog::{log_info, log_warn};

/// gl-base.js `FRAME_OBSERVER_MIN_INTERVAL_MS`: no readback more often than
/// the consumer can use.
pub const MIN_INTERVAL: Duration = Duration::from_millis(1000);
/// A read whose fence has not signalled by now belongs to a saver that has
/// since stopped or been replaced; its bytes would describe the wrong
/// picture, so it is dropped rather than delivered late.
const STALE: Duration = Duration::from_millis(500);

const W: i32 = (GRID.tile * GRID.tiles_x) as i32;
const H: i32 = (GRID.tile * GRID.tiles_y) as i32;

/// Who gets the frames. In the app: the Art-Net worker's queue.
pub struct Registry {
    sink: Option<mpsc::Sender<Msg>>,
}

impl Registry {
    pub const fn new() -> Self {
        Self { sink: None }
    }

    /// renderer.js `syncArtnetFrameObserver`: register exactly one sink while
    /// enabled, none while disabled. Called every `logic` tick; idempotent,
    /// so it neither stacks sinks nor costs anything when nothing changed.
    /// Returns whether registration changed.
    pub fn sync(&mut self, enabled: bool, sink: impl FnOnce() -> Option<mpsc::Sender<Msg>>) -> bool {
        match (enabled, self.sink.is_some()) {
            (true, false) => {
                self.sink = sink();
                self.sink.is_some()
            }
            (false, true) => {
                self.sink = None;
                true
            }
            _ => false,
        }
    }

    pub fn count(&self) -> usize {
        usize::from(self.sink.is_some())
    }
}

static REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
/// Mirrors `REGISTRY.count() > 0`, so the per-frame check is lock-free.
static WANTED: AtomicBool = AtomicBool::new(false);

/// Register or drop the app's sink to follow the setting.
pub fn sync(enabled: bool, sink: impl FnOnce() -> Option<mpsc::Sender<Msg>>) {
    let mut reg = REGISTRY.lock().unwrap();
    if reg.sync(enabled, sink) {
        let on = reg.count() > 0;
        WANTED.store(on, Ordering::Release);
        if on {
            log_info!("[Art-Net] frame observer registered");
        } else {
            log_info!("[Art-Net] frame observer removed; readback stopped");
        }
    }
}

/// Whether anything is listening. Painter savers ask before adding the
/// paint callback at all.
pub fn wanted() -> bool {
    WANTED.load(Ordering::Acquire)
}

/// The 64x32 read (rows bottom-up, tiles side by side) rearranged into the
/// sample grid's tile-major layout.
pub fn tiles_from_image(img: &[u8], grid: SampleGrid) -> Vec<u8> {
    let row = grid.tile * grid.tiles_x * 4;
    let mut out = Vec::with_capacity(grid.bytes());
    for ty in 0..grid.tiles_y {
        for tx in 0..grid.tiles_x {
            for y in 0..grid.tile {
                let start = (ty * grid.tile + y) * row + tx * grid.tile * 4;
                out.extend_from_slice(&img[start..start + grid.tile * 4]);
            }
        }
    }
    out
}

struct Reader {
    fbo: glow::Framebuffer,
    pbo: glow::Buffer,
    pending: Option<(glow::Fence, Instant)>,
    last_issue: Option<Instant>,
    /// Measured costs, logged for the first reads and then now and then.
    reads: u64,
}

thread_local! {
    /// GL objects belong to the thread with the context: the render thread,
    /// the only one that runs paint callbacks. A fence is a raw pointer, so
    /// this is per thread rather than a static behind a Mutex.
    static READER: std::cell::RefCell<Option<Result<Reader, ()>>> = const { std::cell::RefCell::new(None) };
}

unsafe fn create(gl: &glow::Context) -> Result<Reader, String> {
    unsafe {
        let tex = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, W, H, 0, glow::RGBA, glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        gl.bind_texture(glow::TEXTURE_2D, None);
        let fbo = gl.create_framebuffer()?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        if status != glow::FRAMEBUFFER_COMPLETE {
            return Err(format!("sample target incomplete: 0x{status:x}"));
        }
        let pbo = gl.create_buffer()?;
        gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(pbo));
        gl.buffer_data_size(glow::PIXEL_PACK_BUFFER, W * H * 4, glow::STREAM_READ);
        gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
        Ok(Reader { fbo, pbo, pending: None, last_issue: None, reads: 0 })
    }
}

/// Sample the frame at `viewport` (pixels, GL origin bottom-left) of `src`
/// (None: the default framebuffer). Call once per rendered frame, from
/// inside a paint callback, after the saver has drawn. Leaves `restore`
/// bound as the framebuffer.
pub fn observe(gl: &glow::Context, src: Option<glow::Framebuffer>, viewport: [i32; 4], restore: Option<glow::Framebuffer>) {
    if !wanted() {
        return;
    }
    READER.with_borrow_mut(|slot| read(gl, slot, src, viewport, restore));
}

fn read(gl: &glow::Context, slot: &mut Option<Result<Reader, ()>>, src: Option<glow::Framebuffer>, viewport: [i32; 4], restore: Option<glow::Framebuffer>) {
    if slot.is_none() {
        *slot = Some(unsafe { create(gl) }.map_err(|e| log_warn!("[Art-Net] no frame readback: {e}")));
    }
    let Some(Ok(r)) = slot.as_mut() else { return };
    let now = Instant::now();
    unsafe {
        // Collect a read issued on an earlier frame, if the GPU is done.
        if let Some((fence, at)) = r.pending {
            if now.duration_since(at) > STALE {
                gl.delete_sync(fence);
                r.pending = None;
            } else if gl.get_sync_status(fence) == glow::SIGNALED {
                let t0 = Instant::now();
                let mut img = vec![0u8; (W * H * 4) as usize];
                gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(r.pbo));
                gl.get_buffer_sub_data(glow::PIXEL_PACK_BUFFER, 0, &mut img);
                gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
                gl.delete_sync(fence);
                r.pending = None;
                r.reads += 1;
                let collect = t0.elapsed();
                if r.reads <= 3 || r.reads % 600 == 0 {
                    log_info!("[Art-Net] readback #{}: collected {} ms after issue, {} us to copy out",
                        r.reads, now.duration_since(at).as_millis(), collect.as_micros());
                }
                let reg = REGISTRY.lock().unwrap();
                if let Some(tx) = &reg.sink {
                    let _ = tx.send(Msg::Frame(tiles_from_image(&img, GRID), at));
                }
            }
        }

        let [x, y, w, h] = viewport;
        let due = r.last_issue.is_none_or(|t| now.duration_since(t) >= MIN_INTERVAL);
        if r.pending.is_none() && due && w >= GRID.tile as i32 && h >= GRID.tile as i32 {
            let t0 = Instant::now();
            // egui leaves the scissor test on after a callback, and it clips
            // blits too.
            gl.disable(glow::SCISSOR_TEST);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, src);
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(r.fbo));
            gl.blit_framebuffer(x, y, x + w, y + h, 0, 0, W, H, glow::COLOR_BUFFER_BIT, glow::NEAREST);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(r.fbo));
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(r.pbo));
            gl.read_pixels(0, 0, W, H, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelPackData::BufferOffset(0));
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
            match gl.fence_sync(glow::SYNC_GPU_COMMANDS_COMPLETE, 0) {
                Ok(fence) => r.pending = Some((fence, now)),
                Err(e) => log_warn!("[Art-Net] fence failed: {e}"),
            }
            r.last_issue = Some(now);
            if r.reads < 3 || r.reads % 600 == 0 {
                log_info!("[Art-Net] readback issued in {} us ({w}x{h} frame)", t0.elapsed().as_micros());
            }
            gl.bind_framebuffer(glow::FRAMEBUFFER, restore);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- artnet-observer.test.js: registration follows the setting

    fn sink() -> Option<mpsc::Sender<Msg>> {
        Some(mpsc::channel().0)
    }

    #[test]
    fn registers_nothing_while_disabled() {
        let mut r = Registry::new();
        r.sync(false, sink);
        assert_eq!(r.count(), 0);
    }

    #[test]
    fn registers_exactly_one_when_enabled_and_removes_it_again() {
        let mut r = Registry::new();
        r.sync(true, sink);
        assert_eq!(r.count(), 1);
        r.sync(false, sink);
        assert_eq!(r.count(), 0);
    }

    #[test]
    fn does_not_stack_when_synced_repeatedly_while_enabled() {
        let mut r = Registry::new();
        r.sync(true, sink);
        let mut asked = 0;
        for _ in 0..10 {
            assert!(!r.sync(true, || { asked += 1; sink() }));
        }
        assert_eq!((r.count(), asked), (1, 0));
    }

    #[test]
    fn survives_being_toggled_repeatedly() {
        let mut r = Registry::new();
        for _ in 0..5 {
            r.sync(true, sink);
            assert_eq!(r.count(), 1);
            r.sync(false, sink);
            assert_eq!(r.count(), 0);
        }
    }

    #[test]
    fn rate_limited_to_no_more_often_than_the_consumer_can_use() {
        // Art-Net sends at 1 Hz; sampling faster is waste by definition.
        assert!(MIN_INTERVAL >= Duration::from_millis(super::super::client::SEND_INTERVAL_MS));
    }

    // ---- layout

    #[test]
    fn tiles_come_out_in_the_sample_grids_layout() {
        // Paint each tile of a 64x32 image with its own index in red.
        let row = W as usize * 4;
        let mut img = vec![0u8; (W * H * 4) as usize];
        for y in 0..H as usize {
            for x in 0..W as usize {
                img[y * row + x * 4] = ((y / 8) * 8 + x / 8) as u8;
            }
        }
        let tiles = tiles_from_image(&img, GRID);
        assert_eq!(tiles.len(), GRID.bytes());
        for (i, tile) in tiles.as_chunks::<{ GRID.bytes_per_tile() }>().0.iter().enumerate() {
            assert!(tile.as_chunks::<4>().0.iter().all(|p| p[0] == i as u8), "tile {i}");
        }
    }
}
