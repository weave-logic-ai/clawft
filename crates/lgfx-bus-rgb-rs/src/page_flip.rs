// SPDX-License-Identifier: MIT OR Apache-2.0 OR BSD-3-Clause
//
// Pure page-flip state machine for double-buffered RGB-DPI present.
// Extracted so the swap/index invariants can be unit-tested without
// the ESP32-S3 HAL or a physical panel (WEFT-595 / BUG-1).

//! Pure double-buffer page-flip model.
//!
//! The live driver ([`crate::isr`] + [`crate::bus`]) keeps this state
//! in atomics and GDMA restart-descriptor addresses. This module is
//! the same state machine as plain values so host-side unit tests can
//! prove:
//!
//! 1. `offscreen_index` is always `scanning_fb ^ 1` after init.
//! 2. A present request + VSYNC swaps scanning ↔ offscreen exactly once.
//! 3. Spurious VSYNCs without a pending present do not flip.
//! 4. Two pending presents without a VSYNC between them collapse to one
//!    swap (one-shot `present_pending`).
//!
//! ## Dirty-rect coherence (BUG-1 companion)
//!
//! After a successful swap, the new **offscreen** buffer still holds
//! frame N−1 (or older). Dirty-rect renderers that only clear/redraw
//! damage rects will then present a hybrid of N−1 + N damage — which
//! looks like tearing and "stuck" layout (gutter coords on the wire
//! never appear on-panel). Callers MUST either:
//!
//! - full-clear / full-redraw every frame, or
//! - [`BusRgb::copy_scanning_to_offscreen`](crate::BusRgb::copy_scanning_to_offscreen)
//!   before applying partial damage (see `DpiSurface::begin_frame`).

/// Indices of the two framebuffers.
pub const FB_A: u8 = 0;
pub const FB_B: u8 = 1;

/// Host-testable page-flip state (mirrors ISR atomics + restart addrs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageFlipState {
    /// Framebuffer the GDMA is scanning (`0` or `1`).
    pub scanning_fb: u8,
    /// Set by `request_present`, cleared on the next `on_vsync` swap.
    pub present_pending: bool,
    /// Restart-descriptor address of the currently-scanning ring.
    pub scanning_restart: u32,
    /// Restart-descriptor address of the offscreen ring.
    pub offscreen_restart: u32,
}

impl PageFlipState {
    /// FB-A starts scanning; FB-B is offscreen (matches `BusRgb::new`).
    pub fn new(restart_a: u32, restart_b: u32) -> Self {
        Self {
            scanning_fb: FB_A,
            present_pending: false,
            scanning_restart: restart_a,
            offscreen_restart: restart_b,
        }
    }

    /// Index of the buffer the CPU is safe to write (`scanning ^ 1`).
    #[inline]
    pub fn offscreen_index(&self) -> u8 {
        self.scanning_fb ^ 1
    }

    /// Schedule a swap on the next VSYNC (one-shot).
    #[inline]
    pub fn request_present(&mut self) {
        self.present_pending = true;
    }

    /// VSYNC ISR page-flip branch. Returns `true` if a swap occurred.
    ///
    /// Order matches `lcd_vsync_isr`: swap restart-desc addresses, then
    /// toggle `scanning_fb`, then the caller re-arms GDMA from
    /// `scanning_restart` (already updated).
    pub fn on_vsync(&mut self) -> bool {
        if !self.present_pending {
            return false;
        }
        self.present_pending = false;
        core::mem::swap(&mut self.scanning_restart, &mut self.offscreen_restart);
        self.scanning_fb ^= 1;
        true
    }

    /// Simulate the synchronous `present()` wait: request + one VSYNC.
    /// Returns the post-swap offscreen index (where the next draw lands).
    pub fn present_sync(&mut self) -> u8 {
        self.request_present();
        let _ = self.on_vsync();
        self.offscreen_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RA: u32 = 0x3FC0_0000;
    const RB: u32 = 0x3FC0_1000;

    #[test]
    fn init_scans_a_draws_to_b() {
        let s = PageFlipState::new(RA, RB);
        assert_eq!(s.scanning_fb, FB_A);
        assert_eq!(s.offscreen_index(), FB_B);
        assert_eq!(s.scanning_restart, RA);
        assert_eq!(s.offscreen_restart, RB);
        assert!(!s.present_pending);
    }

    #[test]
    fn vsync_without_present_is_noop() {
        let mut s = PageFlipState::new(RA, RB);
        assert!(!s.on_vsync());
        assert_eq!(s.scanning_fb, FB_A);
        assert_eq!(s.scanning_restart, RA);
        assert_eq!(s.offscreen_restart, RB);
    }

    #[test]
    fn present_then_vsync_swaps_once() {
        let mut s = PageFlipState::new(RA, RB);
        s.request_present();
        assert!(s.on_vsync());
        // Just-drawn FB-B is now scanning; next draw lands on FB-A.
        assert_eq!(s.scanning_fb, FB_B);
        assert_eq!(s.offscreen_index(), FB_A);
        assert_eq!(s.scanning_restart, RB);
        assert_eq!(s.offscreen_restart, RA);
        assert!(!s.present_pending);
    }

    #[test]
    fn double_present_request_collapses_to_one_swap() {
        let mut s = PageFlipState::new(RA, RB);
        s.request_present();
        s.request_present(); // still one-shot
        assert!(s.on_vsync());
        assert!(!s.present_pending);
        assert!(!s.on_vsync()); // second VSYNC does not double-swap
        assert_eq!(s.scanning_fb, FB_B);
    }

    #[test]
    fn alternating_presents_keep_offscreen_xor_scanning() {
        let mut s = PageFlipState::new(RA, RB);
        for i in 0..16 {
            let before_scan = s.scanning_fb;
            let next_off = s.present_sync();
            assert_eq!(next_off, before_scan, "iter {i}: offscreen must be prior scanning");
            assert_eq!(s.offscreen_index(), s.scanning_fb ^ 1);
            assert_ne!(s.scanning_restart, s.offscreen_restart);
        }
    }

    #[test]
    fn wrong_buffer_invariant_would_fail_if_toggle_skipped() {
        // Documents the BUG-1 failure mode: if scanning_fb is not
        // toggled when restart addrs swap, offscreen_index() still
        // points at the buffer the GDMA is about to scan → CPU and
        // GDMA fight over the same FB (tearing + "updates never land").
        let mut broken = PageFlipState::new(RA, RB);
        broken.request_present();
        // Manual broken path: swap addrs but forget scanning_fb.
        core::mem::swap(
            &mut broken.scanning_restart,
            &mut broken.offscreen_restart,
        );
        broken.present_pending = false;
        // scanning_fb still 0, but scanning_restart is now RB.
        assert_eq!(broken.scanning_fb, FB_A);
        assert_eq!(broken.offscreen_index(), FB_B);
        assert_eq!(broken.scanning_restart, RB);
        // CPU believes offscreen is B while GDMA re-arms at RB (also B).
        assert_eq!(
            broken.offscreen_index(),
            FB_B,
            "CPU write target"
        );
        // Correct path would have scanning_fb == FB_B so offscreen is A.
        let mut good = PageFlipState::new(RA, RB);
        good.request_present();
        assert!(good.on_vsync());
        assert_eq!(good.scanning_fb, FB_B);
        assert_eq!(good.offscreen_index(), FB_A);
        assert_ne!(
            good.offscreen_index(),
            broken.offscreen_index(),
            "correct swap must diverge from toggle-skipped path"
        );
    }
}
