/// An RGB color with full-range (0–255) channels, before terminal quantization.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Rgb {
    /// Red intensity.
    red: u8,
    /// Green intensity.
    green: u8,
    /// Blue intensity.
    blue: u8,
}

impl Rgb {
    /// Construct a color without restricting it to terminal cube coordinates.
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    /// Interpolate with rounding, from this color at zero to `lit` at 255.
    pub const fn interpolate(self, lit: Self, amount: u8) -> Self {
        const fn channel(dark: u8, lit: u8, amount: u8) -> u8 {
            ((dark as u32 * (255 - amount as u32) + lit as u32 * amount as u32 + 127) / 255) as u8
        }
        Self::new(
            channel(self.red, lit.red, amount),
            channel(self.green, lit.green, amount),
            channel(self.blue, lit.blue, amount),
        )
    }

    /// Quantize to xterm's cube, including the finer gray ramp for neutral RGB.
    ///
    /// Cube channels are 0, 95, 135, 175, 215, 255, not multiples of 51.
    /// Ties select the darker value. Avoid configurable ANSI colors 0–15.
    /// The palette is defined by xterm's `256colres.pl`.
    pub const fn to_u8(self) -> u8 {
        if self.red == self.green && self.green == self.blue {
            let mut best = 16;
            let mut index = 17u16;
            while index <= 255 {
                let rgb = Self::from_index(index as u8);
                if rgb.red == rgb.green && rgb.green == rgb.blue {
                    let old = Self::from_index(best).red;
                    if self.red.abs_diff(rgb.red) < self.red.abs_diff(old)
                        || (self.red.abs_diff(rgb.red) == self.red.abs_diff(old) && rgb.red < old)
                    {
                        best = index as u8;
                    }
                }
                index += 1;
            }
            return best;
        }
        const fn channel(value: u8) -> u8 {
            let levels = [0u8, 95, 135, 175, 215, 255];
            let mut best = 0;
            let mut index = 1;
            while index < levels.len() {
                if value.abs_diff(levels[index]) < value.abs_diff(levels[best]) {
                    best = index;
                }
                index += 1;
            }
            best as u8
        }
        16 + 36 * channel(self.red) + 6 * channel(self.green) + channel(self.blue)
    }

    /// Decode a fixed xterm color; configurable ANSI entries are not supported.
    pub(super) const fn from_index(index: u8) -> Self {
        assert!(index >= 16);
        match index {
            16..=231 => {
                let levels = [0, 95, 135, 175, 215, 255];
                let n = (index - 16) as usize;
                Self::new(levels[n / 36], levels[n / 6 % 6], levels[n % 6])
            }
            _ => {
                let level = 8 + (index - 232) * 10;
                Self::new(level, level, level)
            }
        }
    }

    /// Integer Rec.709 luma, used to order rendered shades (not physical light).
    pub(super) const fn luma(self) -> u32 {
        2126 * self.red as u32 + 7152 * self.green as u32 + 722 * self.blue as u32
    }

    /// Luma-weighted squared RGB distance for deterministic palette selection.
    pub(super) const fn distance(self, other: Self) -> u32 {
        let r = self.red.abs_diff(other.red) as u32;
        let g = self.green.abs_diff(other.green) as u32;
        let b = self.blue.abs_diff(other.blue) as u32;
        2126 * r * r + 7152 * g * g + 722 * b * b
    }
}
