//! Minimum contrast (iTerm2's setting): text too close to its background
//! is moved toward black or white, just far enough to be readable.

fn channel(v: u32) -> f64 {
    let c = v as f64 / 255.0;
    if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// WCAG relative luminance of 0xRRGGBB.
pub fn luminance(c: u32) -> f64 {
    0.2126 * channel(c >> 16 & 0xff) + 0.7152 * channel(c >> 8 & 0xff) + 0.0722 * channel(c & 0xff)
}

/// WCAG contrast ratio, 1 (none) to 21 (black on white).
pub fn ratio(a: u32, b: u32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

fn mix(a: u32, b: u32, t: f64) -> u32 {
    let ch = |s: u32| {
        let x = (a >> s & 0xff) as f64;
        let y = (b >> s & 0xff) as f64;
        ((x + (y - x) * t).round() as u32).min(255)
    };
    ch(16) << 16 | ch(8) << 8 | ch(0)
}

/// The setting (0 = off, 1 = always black or white) as a target ratio.
pub fn target(minimum_contrast: f64) -> f64 {
    1.0 + minimum_contrast.clamp(0.0, 1.0) * 20.0
}

/// `fg`, or the nearest color toward black or white that reaches `target`
/// against `bg` (whichever direction gets there; the stronger one if neither).
pub fn ensure(fg: u32, bg: u32, target: f64) -> u32 {
    if target <= 1.0 || ratio(fg, bg) >= target {
        return fg;
    }
    let toward = |end: u32| -> Option<u32> {
        if ratio(end, bg) < target {
            return None;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..16 {
            let mid = (lo + hi) / 2.0;
            if ratio(mix(fg, end, mid), bg) >= target { hi = mid } else { lo = mid }
        }
        Some(mix(fg, end, hi))
    };
    // Prefer the direction away from the background.
    let first = if luminance(bg) > 0.5 { 0x000000 } else { 0xffffff };
    let second = first ^ 0xffffff;
    toward(first)
        .or_else(|| toward(second))
        .unwrap_or_else(|| if ratio(first, bg) >= ratio(second, bg) { first } else { second })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_ratios() {
        assert!((ratio(0x000000, 0xffffff) - 21.0).abs() < 0.01);
        assert!((ratio(0x777777, 0x777777) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn readable_text_is_untouched() {
        assert_eq!(ensure(0x657b83, 0xfdf6e3, target(0.15)), 0x657b83);
        assert_eq!(ensure(0xeee8d5, 0xfdf6e3, target(0.0)), 0xeee8d5);
    }

    #[test]
    fn faint_text_is_pushed_just_far_enough() {
        // Solarized Light: "white" (eee8d5) on base3 (fdf6e3).
        let t = target(0.15);
        let out = ensure(0xeee8d5, 0xfdf6e3, t);
        assert!(ratio(out, 0xfdf6e3) >= t);
        assert!(ratio(out, 0xfdf6e3) < t + 0.3, "only as far as needed");
        assert!(luminance(out) < luminance(0xeee8d5), "darker on a light background");
        // On a dark background it lightens.
        let out = ensure(0x2a2d32, 0x1b1d20, t);
        assert!(luminance(out) > luminance(0x2a2d32));
    }

    #[test]
    fn full_setting_gives_black_or_white() {
        assert_eq!(ensure(0x808080, 0xfdf6e3, target(1.0)), 0x000000);
    }
}
