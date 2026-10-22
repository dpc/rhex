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
    pub fn interpolate(self, lit: Self, amount: u8) -> Self {
        let amount = u32::from(amount);
        let channel = |dark: u8, lit: u8| {
            ((u32::from(dark) * (255 - amount) + u32::from(lit) * amount + 127) / 255) as u8
        };
        Self::new(
            channel(self.red, lit.red),
            channel(self.green, lit.green),
            channel(self.blue, lit.blue),
        )
    }

    /// Quantize to xterm's cube, including the finer gray ramp for neutral RGB.
    ///
    /// Cube channels are 0, 95, 135, 175, 215, 255, not multiples of 51.
    /// Ties select the darker value. Avoid configurable ANSI colors 0–15.
    /// The palette is defined by xterm's `256colres.pl`.
    pub fn to_u8(self) -> u8 {
        if self.red == self.green && self.green == self.blue {
            return super::color::GRAY
                .into_iter()
                .chain([59, 102, 145, 188])
                .min_by_key(|&index| {
                    let level = Self::gray_value(index);
                    (self.red.abs_diff(level), level)
                })
                .unwrap();
        }
        let channel = |value: u8| {
            [0u8, 95, 135, 175, 215, 255]
                .into_iter()
                .enumerate()
                .min_by_key(|&(_, level)| value.abs_diff(level))
                .unwrap()
                .0 as u8
        };
        16 + 36 * channel(self.red) + 6 * channel(self.green) + channel(self.blue)
    }

    /// Decode the neutral palette endpoints and ramp.
    fn gray_value(index: u8) -> u8 {
        match index {
            16 => 0,
            231 => 255,
            59 => 95,
            102 => 135,
            145 => 175,
            188 => 215,
            _ => 8 + (index - 232) * 10,
        }
    }
}
