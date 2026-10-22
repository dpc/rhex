use super::rgb::Rgb;

/// Presentation saturation: lanterns and forest lights emit five light units.
pub const FULL_LIGHT: u32 = 5;

/// RGB endpoints for visible lighting, with a separate remembered indexed color.
#[derive(Copy, Clone)]
pub struct Shade {
    /// Full-light base color.
    lit: Rgb,
    /// Readable zero-light color; backgrounds may start at black.
    dark: Rgb,
    /// Memory never samples live lighting.
    remembered: u8,
    /// Optional zero-light contrast override for inverse dungeon colors.
    unlit: Option<u8>,
    /// Optional terminal-native ramp when RGB interpolation loses visible steps.
    indexed: Option<[u8; FULL_LIGHT as usize + 1]>,
}

impl Shade {
    /// Define a continuous visible ramp and an independent memory style.
    pub const fn new(lit: Rgb, dark: Rgb, remembered: u8) -> Self {
        Self {
            lit,
            dark,
            remembered,
            unlit: None,
            indexed: None,
        }
    }

    /// Use measured terminal colors for each light unit, retaining memory styling.
    pub const fn with_indexed(mut self, indexed: [u8; FULL_LIGHT as usize + 1]) -> Self {
        self.indexed = Some(indexed);
        self
    }

    /// Keep zero-light contrast when a visible ramp must start across its foreground.
    pub const fn with_unlit(mut self, unlit: u8) -> Self {
        self.unlit = Some(unlit);
        self
    }

    /// Convert actual apparent light to a terminal color only at the drawing boundary.
    pub fn index(self, visible: bool, light: u32) -> u8 {
        if !visible {
            return self.remembered;
        }
        if let Some(indexed) = self.indexed {
            return indexed[light.min(FULL_LIGHT) as usize];
        }
        if light == 0
            && let Some(unlit) = self.unlit
        {
            return unlit;
        }
        let amount = (light.min(FULL_LIGHT) * 255 / FULL_LIGHT) as u8;
        self.dark.interpolate(self.lit, amount).to_u8()
    }
}
