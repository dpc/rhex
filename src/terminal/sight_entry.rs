use ratatui::style::Style;

/// One distinct, currently visible map symbol and its name.
pub(super) struct SightEntry {
    /// Ordinary map glyph, without transient targeting or animation overlays.
    pub glyph: char,
    /// Fully lit map colors, independent of live lighting and transient overlays.
    pub style: Style,
    /// Player-facing name of the topmost visible occupant or notable terrain.
    pub name: String,
}
