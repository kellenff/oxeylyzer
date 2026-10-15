//! Minimal truecolor helper that replaces the the removed `ansi-rgb` personal-fork dependency (FR-015).
//!
//! Only the foreground truecolor path is used in this crate (`display.rs::heatmap_heat`),
//! so only that path is implemented. The emitted SGR bytes are identical to
//! the old fork's `rgb(r, g, b)` + `Colorable::fg`: prelude `\x1B[38;2;<r>;<g>;<b>m`, text,
//! epilogue `\x1B[39m`.

use std::fmt;

/// A 24-bit foreground color, the equivalent of the removed fork's `rgb(r, g, b)` (`RGB8`).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Construct an RGB color, e.g. `rgb(215, 120, 120)`.
pub const fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    Rgb { r, g, b }
}

/// Extension trait mirroring the removed fork's `Colorable`, limited to the foreground case.
pub trait Colorable {
    /// Wrap `self` in a truecolor foreground SGR sequence, reset with `\x1B[39m`.
    fn fg(self, color: Rgb) -> Colored<Self>
    where
        Self: Sized;
}

impl<T> Colorable for T {
    fn fg(self, color: Rgb) -> Colored<Self>
    where
        Self: Sized,
    {
        Colored { item: self, color }
    }
}

/// `item` rendered between the color prelude and epilogue; the analogue of
/// the removed fork's `WithForeground`.
pub struct Colored<T> {
    item: T,
    color: Rgb,
}

impl<T: fmt::Display> fmt::Display for Colored<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\x1B[38;2;{};{};{}m{}\x1B[39m",
            self.color.r, self.color.g, self.color.b, self.item
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte sequences recorded from the removed fork dependency, so the
    /// swap is byte-identical per contracts/cli-surface.md.
    #[test]
    fn fg_matches_removed_fork_bytes() {
        assert_eq!(
            format!("{}", "a".to_string().fg(rgb(215, 0, 0))),
            "\x1B[38;2;215;0;0ma\x1B[39m"
        );
        assert_eq!(
            format!("{}", "z".to_string().fg(rgb(215, 215, 215))),
            "\x1B[38;2;215;215;215mz\x1B[39m"
        );
        assert_eq!(
            format!("{}", "".to_string().fg(rgb(0, 13, 200))),
            "\x1B[38;2;0;13;200m\x1B[39m"
        );
    }

    #[test]
    fn fg_works_on_displayable_non_strings() {
        assert_eq!(
            format!("{}", 42.fg(rgb(1, 2, 3))),
            "\x1B[38;2;1;2;3m42\x1B[39m"
        );
    }
}
