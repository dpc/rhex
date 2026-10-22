use super::rgb::Rgb;

/// Presentation saturation: lanterns and forest lights emit five light units.
pub const FULL_LIGHT: u32 = 5;

/// A precomputed terminal-palette ramp with independent remembered styling.
#[derive(Copy, Clone)]
pub struct Shade {
    /// Visible colors from zero light through lantern strength.
    indexed: [u8; FULL_LIGHT as usize + 1],
    /// Memory never samples live lighting.
    remembered: u8,
}

impl Shade {
    /// Select visible shades from RGB endpoints, preserving useful existing ramps.
    ///
    /// Collapsed ramps use colors sampled along the RGB segment, ordered by rendered
    /// luma. If fewer than six exist, neutral grays within the endpoint luma range
    /// supply fallback steps. Endpoints never move. Constrained ranges repeat colors
    /// rather than inventing brighter/darker endpoints; equal-luma endpoints retain
    /// their original interpolation. Identical endpoints give a fixed color.
    ///
    /// Work is bounded and const-evaluated for palette declarations. Drawing only
    /// indexes the resulting six colors; it never searches the terminal palette.
    pub const fn new(lit: Rgb, dark: Rgb, remembered: u8) -> Self {
        Self {
            indexed: ramp(dark, lit),
            remembered,
        }
    }

    /// Keep zero-light contrast when a visible ramp must start across its foreground.
    pub const fn with_unlit(mut self, unlit: u8) -> Self {
        self.indexed[0] = unlit;
        self
    }

    /// Convert actual apparent light to a terminal color only at the drawing boundary.
    pub fn index(self, visible: bool, light: u32) -> u8 {
        if visible {
            self.indexed[light.min(FULL_LIGHT) as usize]
        } else {
            self.remembered
        }
    }
}

/// Preserve good interpolation; repair collapsed steps without terrain-specific rules.
const fn ramp(dark: Rgb, lit: Rgb) -> [u8; FULL_LIGHT as usize + 1] {
    let mut result = [0; FULL_LIGHT as usize + 1];
    let mut step = 0;
    while step < result.len() {
        result[step] = dark
            .interpolate(lit, (step as u32 * 255 / FULL_LIGHT) as u8)
            .to_u8();
        step += 1;
    }
    let first = Rgb::from_index(result[0]).luma();
    let last = Rgb::from_index(result[FULL_LIGHT as usize]).luma();
    if first == last {
        return result;
    }
    let ascending = first < last;
    let mut distinct = true;
    step = 1;
    while step < result.len() {
        let previous = Rgb::from_index(result[step - 1]).luma();
        let next = Rgb::from_index(result[step]).luma();
        if (ascending && previous >= next) || (!ascending && previous <= next) {
            distinct = false;
        }
        step += 1;
    }
    if distinct {
        return result;
    }

    // Sample all 8-bit interpolation positions once. A boolean set deduplicates
    // quantized colors; configurable ANSI entries never enter the candidate pool.
    let mut available = [false; 256];
    let mut amount = 0;
    while amount <= 255 {
        available[dark.interpolate(lit, amount as u8).to_u8() as usize] = true;
        amount += 1;
    }
    let mut candidates = [0; 240];
    let mut count = collect(&available, first, last, &mut candidates);
    if count < result.len() - 2 {
        // Grays are a fallback, not a replacement for a sufficiently rich hue path.
        let mut index = 232;
        while index <= 255 {
            available[index] = true;
            index += 1;
        }
        count = collect(&available, first, last, &mut candidates);
    }

    if count < result.len() - 2 {
        // There are genuinely too few shades in this luma interval. Spread all
        // available colors across the light bands, keeping both endpoints exact.
        let mut all = [result[0]; FULL_LIGHT as usize + 1];
        let mut i = 0;
        while i < count {
            all[i + 1] = candidates[i];
            i += 1;
        }
        all[count + 1] = result[FULL_LIGHT as usize];
        step = 1;
        while step < FULL_LIGHT as usize {
            result[step] =
                all[(step * (count + 1) + FULL_LIGHT as usize / 2) / FULL_LIGHT as usize];
            step += 1;
        }
        return result;
    }

    // Minimize total error across the four interior bands, not greedily per band:
    // an early saturated shade must not force later bands back onto washed-out
    // grays. Prefix minima keep this bounded to four linear candidate scans.
    // Equal costs retain the earlier candidate in endpoint order.
    let mut costs = [0u32; 240];
    let mut parents = [[0usize; 240]; FULL_LIGHT as usize - 1];
    step = 1;
    while step < FULL_LIGHT as usize {
        let target = dark.interpolate(lit, (step as u32 * 255 / FULL_LIGHT) as u8);
        let mut next = [u32::MAX; 240];
        let mut prefix = 0;
        let mut candidate = step - 1;
        while candidate < count {
            if step > 1 {
                if costs[candidate - 1] < costs[prefix] {
                    prefix = candidate - 1;
                }
                parents[step - 1][candidate] = prefix;
            }
            next[candidate] =
                costs[prefix] + target.distance(Rgb::from_index(candidates[candidate]));
            candidate += 1;
        }
        costs = next;
        step += 1;
    }
    let mut best = FULL_LIGHT as usize - 2;
    let mut candidate = best + 1;
    while candidate < count {
        if costs[candidate] < costs[best] {
            best = candidate;
        }
        candidate += 1;
    }
    step = FULL_LIGHT as usize - 1;
    while step > 0 {
        result[step] = candidates[best];
        best = parents[step - 1][best];
        step -= 1;
    }
    result
}

/// Collect strictly interior colors in endpoint order, with unique rendered luma.
const fn collect(available: &[bool; 256], first: u32, last: u32, out: &mut [u8; 240]) -> usize {
    let ascending = first < last;
    let mut count = 0;
    let mut index = 16;
    while index <= 255 {
        let luma = Rgb::from_index(index as u8).luma();
        let interior = if ascending {
            first < luma && luma < last
        } else {
            last < luma && luma < first
        };
        if available[index] && interior {
            let mut position = 0;
            while position < count {
                let previous = Rgb::from_index(out[position]).luma();
                if (ascending && previous >= luma) || (!ascending && previous <= luma) {
                    break;
                }
                position += 1;
            }
            if position == count || Rgb::from_index(out[position]).luma() != luma {
                let mut tail = count;
                while tail > position {
                    out[tail] = out[tail - 1];
                    tail -= 1;
                }
                out[position] = index as u8;
                count += 1;
            }
        }
        index += 1;
    }
    count
}
