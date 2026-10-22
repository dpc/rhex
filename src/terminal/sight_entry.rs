use ratatui::style::Style;

/// One distinct, currently visible map symbol and its name.
pub(super) struct SightEntry {
    /// Ordinary map glyph, without transient targeting or animation overlays.
    pub glyph: char,
    /// Map lighting and colors at the nearest matching visible cell.
    pub style: Style,
    /// Player-facing name of the topmost visible occupant or notable terrain.
    pub name: String,
}
