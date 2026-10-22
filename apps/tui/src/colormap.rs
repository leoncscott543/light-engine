//! Full-RGB perceptual colormaps for heatmaps, sampled by linear interpolation
//! between evenly spaced stops.

use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Colormap {
    Turbo,
    Inferno,
    Viridis,
    /// Theme yellows, pale butter to deep bronze.
    Butter,
}

const TURBO: &[u32] = &[
    0x30123b, 0x4145ab, 0x4675ed, 0x39a2fc, 0x1bcfd4, 0x24eca6, 0x61fc6c, 0xa4fc3b, 0xd1e834,
    0xf6be39, 0xfe822a, 0xe2490d, 0xb01e04, 0x7a0403,
];
const INFERNO: &[u32] = &[
    0x000004, 0x1b0c41, 0x4a0c6b, 0x781c6d, 0xa52c60, 0xcf4446, 0xed6925, 0xfb9b06, 0xf7d13d,
    0xfcffa4,
];
const VIRIDIS: &[u32] = &[
    0x440154, 0x482878, 0x3e4989, 0x31688e, 0x26828e, 0x1f9e89, 0x35b779, 0x6ece58, 0xb5de2b,
    0xfde725,
];
const BUTTER: &[u32] = &[
    0xfbf5dc, 0xf7ecbf, 0xf6d84c, 0xf5c400, 0xe2a900, 0xbf8800, 0x8a5a00, 0x3f3300,
];

impl Colormap {
    pub const ALL: [Self; 4] = [Self::Turbo, Self::Inferno, Self::Viridis, Self::Butter];

    pub fn name(self) -> &'static str {
        match self {
            Self::Turbo => "turbo",
            Self::Inferno => "inferno",
            Self::Viridis => "viridis",
            Self::Butter => "butter",
        }
    }

    fn stops(self) -> &'static [u32] {
        match self {
            Self::Turbo => TURBO,
            Self::Inferno => INFERNO,
            Self::Viridis => VIRIDIS,
            Self::Butter => BUTTER,
        }
    }

    pub fn rgb(self, t: f64) -> (u8, u8, u8) {
        let stops = self.stops();
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        let pos = t * (stops.len() - 1) as f64;
        let i = (pos.floor() as usize).min(stops.len() - 2);
        let f = pos - i as f64;
        let channel = |c: u32, shift: u32| f64::from((c >> shift) & 0xff);
        let mix = |shift| {
            let a = channel(stops[i], shift);
            let b = channel(stops[i + 1], shift);
            (a + (b - a) * f).round() as u8
        };
        (mix(16), mix(8), mix(0))
    }

    /// Color for a normalized value `t` in `0..=1`.
    pub fn color(self, t: f64) -> Color {
        let (r, g, b) = self.rgb(t);
        Color::Rgb(r, g, b)
    }

    /// Black or white, whichever reads better on top of `color(t)`.
    pub fn contrast(self, t: f64) -> Color {
        let (r, g, b) = self.rgb(t);
        let luma = 0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b);
        if luma > 140.0 {
            Color::Rgb(0, 0, 0)
        } else {
            Color::Rgb(255, 255, 255)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_match_stops_and_clamp() {
        for map in Colormap::ALL {
            let stops = map.stops();
            let rgb = |c: u32| ((c >> 16) as u8, (c >> 8) as u8, c as u8);
            assert_eq!(map.rgb(0.0), rgb(stops[0]));
            assert_eq!(map.rgb(1.0), rgb(stops[stops.len() - 1]));
            assert_eq!(map.rgb(-3.0), map.rgb(0.0));
            assert_eq!(map.rgb(f64::NAN), map.rgb(0.0));
            assert_eq!(map.rgb(7.0), map.rgb(1.0));
        }
    }

    #[test]
    fn interpolates_between_stops() {
        let (r, _, _) = Colormap::Inferno.rgb(0.5 / 9.0);
        assert!(r > 0x00 && r < 0x1b);
    }
}
