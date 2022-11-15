use eyre::Result;

pub mod prelude {
    pub use crate::{
        Ansi, Colour, CursorStyle, CursorVisibility, DisplayEraseMode, LineEraseMode, SgrParameter,
    };
}

/// Convert a string literal to an ANSI escape sequence.
/// See: <https://github.com/crossterm-rs/crossterm/blob/7e1279edc57a668e98211043710022b2bfa4b3a8/src/macros.rs#L1-L6>
#[macro_export]
macro_rules! ansi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}

/// ANSI escape sequences. Can be directly formatted into strings.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ansi {
    // Cursor manipulation
    /// Set the (x, y) cursor position.
    CursorPosition(u64, u64),
    /// Set the cursor style.
    CursorStyle(CursorStyle),
    /// Set the cursor visibility.
    CursorVisibility(CursorVisibility),
    /// Move the cursor up.
    CursorUp(u64),
    /// Move the cursor down.
    CursorDown(u64),
    /// Move the cursor left.
    CursorLeft(u64),
    /// Move the cursor right.
    CursorRight(u64),
    /// Move the cursor to the start of line `count` steps down.
    CursorNextLine(u64),
    /// Move the cursor to the start of line `count` steps up.
    CursorPreviousLine(u64),
    /// Move the cursor to the column `x`.
    CursorHorizontalAbsolute(u64),
    /// Save the current position of the cursor.
    SaveCursorPosition,
    /// Restore the position of the cursor.
    RestoreCursorPosition,

    // Text manipulation
    /// Erase part of the current display.
    EraseInDisplay(DisplayEraseMode),
    /// Erase part of the current line.
    EraseInLine(LineEraseMode),
    /// Scroll the display up.
    ScrollUp(u64),
    /// Scroll the display down.
    ScrollDown(u64),

    // Terminal manipulation
    /// Set the terminal size.
    /// This is not supported on Windows.
    TerminalSize(u64, u64),
    /// Set the terminal title.
    /// This is not supported on Windows.
    TerminalTitle(String),
    /// Set the terminal foreground colour.
    /// This is not supported on Windows.
    TerminalForegroundColour(Colour),
    /// Set the terminal background colour.
    /// This is not supported on Windows.
    TerminalBackgroundColour(Colour),
    /// Set attributes on the current terminal.
    /// This is not supported on Windows.
    /// See: <https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters>
    Sgr(Vec<SgrParameter>),
}

impl Ansi {
    /// Render this ANSI escape sequence into the given `Write`able.
    pub fn render(&self, f: &mut impl std::fmt::Write) -> Result<()> {
        match self {
            // Cursor
            Self::CursorPosition(x, y) => {
                write!(f, ansi!("{};{}H"), y + 1, x + 1)
            }
            Self::CursorStyle(style) => match style {
                CursorStyle::Block => {
                    write!(f, ansi!("2 q"))
                }
                CursorStyle::Bar => {
                    write!(f, ansi!("5 q"))
                }
                CursorStyle::HollowBlock => {
                    write!(f, ansi!("2 q"))
                }
            },
            Self::CursorVisibility(visibility) => match visibility {
                CursorVisibility::Visible => {
                    write!(f, ansi!("?25h"))
                }
                CursorVisibility::Invisible => {
                    write!(f, ansi!("?25l"))
                }
            },
            Self::CursorUp(count) => {
                write!(f, ansi!("{}A"), count)
            }
            Self::CursorDown(count) => {
                write!(f, ansi!("{}B"), count)
            }
            Self::CursorLeft(count) => {
                write!(f, ansi!("{}D"), count)
            }
            Self::CursorRight(count) => {
                write!(f, ansi!("{}C"), count)
            }
            Self::CursorNextLine(count) => {
                write!(f, ansi!("{}E"), count)
            }
            Self::CursorPreviousLine(count) => {
                write!(f, ansi!("{}F"), count)
            }
            Self::CursorHorizontalAbsolute(x) => {
                write!(f, ansi!("{}G"), x + 1)
            }
            Self::SaveCursorPosition => {
                write!(f, ansi!("s"))
            }
            Self::RestoreCursorPosition => {
                write!(f, ansi!("u"))
            }

            // Terminal
            Self::EraseInDisplay(mode) => match mode {
                DisplayEraseMode::All => {
                    write!(f, ansi!("2J"))
                }
                DisplayEraseMode::FromCursorToEnd => {
                    write!(f, ansi!("0J"))
                }
                DisplayEraseMode::FromCursorToStart => {
                    write!(f, ansi!("1J"))
                }
                DisplayEraseMode::ScrollbackBuffer => {
                    write!(f, ansi!("3J"))
                }
            },
            Self::EraseInLine(mode) => match mode {
                LineEraseMode::All => {
                    write!(f, ansi!("2K"))
                }
                LineEraseMode::FromCursorToEnd => {
                    write!(f, ansi!("0K"))
                }
                LineEraseMode::FromCursorToStart => {
                    write!(f, ansi!("1K"))
                }
            },
            Self::ScrollUp(count) => {
                write!(f, ansi!("{}S"), count)
            }
            Self::ScrollDown(count) => {
                write!(f, ansi!("{}T"), count)
            }
            Self::TerminalSize(width, height) => {
                write!(f, ansi!("8;{};{}t"), height, width)
            }
            Self::TerminalTitle(title) => {
                write!(f, "\x1B]0;{}\x07", title)
            }
            Self::TerminalForegroundColour(colour) => {
                write!(f, ansi!("38;5;{}"), colour.index())
            }
            Self::TerminalBackgroundColour(colour) => {
                write!(f, ansi!("48;5;{}"), colour.index())
            }
            Self::Sgr(attributes) => {
                let mut first = true;
                write!(f, ansi!(""))?;
                for attribute in attributes {
                    if first {
                        first = false;
                    } else {
                        write!(f, ";")?;
                    }
                    match attribute {
                        SgrParameter::Reset => {
                            write!(f, "0")
                        }
                        SgrParameter::Bold => {
                            write!(f, "1")
                        }
                        SgrParameter::Faint => {
                            write!(f, "2")
                        }
                        SgrParameter::Italic => {
                            write!(f, "3")
                        }
                        SgrParameter::Underline => {
                            write!(f, "4")
                        }
                        SgrParameter::Blink => {
                            write!(f, "5")
                        }
                        SgrParameter::RapidBlink => {
                            write!(f, "6")
                        }
                        SgrParameter::ReverseVideo => {
                            write!(f, "7")
                        }
                        SgrParameter::Conceal => {
                            write!(f, "8")
                        }
                        SgrParameter::CrossedOut => {
                            write!(f, "9")
                        }
                        SgrParameter::PrimaryFont => {
                            write!(f, "10")
                        }
                        SgrParameter::AlternativeFont(idx) => {
                            write!(f, "{}", 10 + idx)
                        }
                        SgrParameter::Fraktur => {
                            write!(f, "20")
                        }
                        SgrParameter::DoubleUnderline => {
                            write!(f, "21")
                        }
                        SgrParameter::NormalIntensity => {
                            write!(f, "22")
                        }
                        SgrParameter::NotItalicOrBlackletter => {
                            write!(f, "23")
                        }
                        SgrParameter::NotUnderlined => {
                            write!(f, "24")
                        }
                        SgrParameter::SteadyCursor => {
                            write!(f, "25")
                        }
                        SgrParameter::ProportionalSpacing => {
                            write!(f, "26")
                        }
                        SgrParameter::NotReversed => {
                            write!(f, "27")
                        }
                        SgrParameter::Reveal => {
                            write!(f, "28")
                        }
                        SgrParameter::NotCrossedOut => {
                            write!(f, "29")
                        }
                        SgrParameter::Framed => {
                            write!(f, "51")
                        }
                        SgrParameter::Encircled => {
                            write!(f, "52")
                        }
                        SgrParameter::Overlined => {
                            write!(f, "53")
                        }
                        SgrParameter::NotFramedOrEncircled => {
                            write!(f, "54")
                        }
                        SgrParameter::NotOverlined => {
                            write!(f, "55")
                        }
                        SgrParameter::IdeogramUnderlineOrRightSideLine => {
                            write!(f, "60")
                        }
                        SgrParameter::IdeogramDoubleUnderlineOrDoubleLineOnTheRightSide => {
                            write!(f, "61")
                        }
                        SgrParameter::IdeogramOverlineOrLeftSideLine => {
                            write!(f, "62")
                        }
                        SgrParameter::IdeogramDoubleOverlineOrDoubleLineOnTheLeftSide => {
                            write!(f, "63")
                        }
                        SgrParameter::IdeogramStressMarking => {
                            write!(f, "64")
                        }
                        SgrParameter::IdeogramAttributesOff => {
                            write!(f, "65")
                        }
                        SgrParameter::ForegroundColour(colour) => {
                            write!(f, "38;5;{}", colour.index())
                        }
                        SgrParameter::BackgroundColour(colour) => {
                            write!(f, "48;5;{}", colour.index())
                        }
                        SgrParameter::HexForegroundColour(hex) => {
                            let (r, g, b) = Self::rgb(hex);
                            write!(f, "38;2;{};{};{}", r, g, b)
                        }
                        SgrParameter::HexBackgroundColour(hex) => {
                            let (r, g, b) = Self::rgb(hex);
                            write!(f, "48;2;{};{};{}", r, g, b)
                        }
                        SgrParameter::DefaultForegroundColour => {
                            write!(f, "39")
                        }
                        SgrParameter::DefaultBackgroundColour => {
                            write!(f, "49")
                        }
                        SgrParameter::DisableProportionalSpacing => {
                            write!(f, "50")
                        }
                        SgrParameter::UnderlineColour(colour) => {
                            write!(f, "58;5;{}", colour.index())
                        }
                        SgrParameter::HexUnderlineColour(hex) => {
                            let (r, g, b) = Self::rgb(hex);
                            write!(f, "58;2;{};{};{}", r, g, b)
                        }
                        SgrParameter::DefaultUnderlineColour => {
                            write!(f, "59")
                        }
                        SgrParameter::Superscript => {
                            write!(f, "73")
                        }
                        SgrParameter::Subscript => {
                            write!(f, "74")
                        }
                        SgrParameter::NotSuperscriptOrSubscript => {
                            write!(f, "75")
                        }
                    }?;
                }
                write!(f, "m")
            }
        }
        .map_err(|e| e.into())
    }

    /// Convert a hex colour to RGB.
    fn rgb(hex: &u32) -> (u32, u32, u32) {
        let r = (hex >> 16) & 0xFF;
        let g = (hex >> 8) & 0xFF;
        let b = hex & 0xFF;
        (r, g, b)
    }
}

impl std::fmt::Display for Ansi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.render(f).map_err(|_| std::fmt::Error)
    }
}

/// Terminal cursor styles.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum CursorStyle {
    /// The cursor is a block.
    Block,

    /// The cursor is a bar.
    Bar,

    /// The cursor is a hollow block.
    HollowBlock,
}

/// Terminal cursor visibility.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum CursorVisibility {
    /// The cursor is visible.
    Visible,

    /// The cursor is invisible.
    Invisible,
}

/// Default 8-bit colour palette.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Colour {
    /// Black.
    Black,

    /// Red.
    Red,

    /// Green.
    Green,

    /// Yellow.
    Yellow,

    /// Blue.
    Blue,

    /// Magenta.
    Magenta,

    /// Cyan.
    Cyan,

    /// White.
    White,

    /// Bright black.
    BrightBlack,

    /// Bright red.
    BrightRed,

    /// Bright green.
    BrightGreen,

    /// Bright yellow.
    BrightYellow,

    /// Bright blue.
    BrightBlue,

    /// Bright magenta.
    BrightMagenta,

    /// Bright cyan.
    BrightCyan,

    /// Bright white.
    BrightWhite,
}

impl Colour {
    /// Index in the enum.
    pub fn index(&self) -> u64 {
        *self as u64
    }
}

/// Erase part or all of the current display.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisplayEraseMode {
    /// Erase from the cursor to the end of the display.
    FromCursorToEnd,

    /// Erase from the cursor to the start of the display.
    FromCursorToStart,

    /// Erase the entire display.
    All,

    /// Erase the scrollback buffer.
    ScrollbackBuffer,
}

/// Erase part or all of the current line. Does not move the cursor.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum LineEraseMode {
    /// Erase from the cursor to the end of the line.
    FromCursorToEnd,

    /// Erase from the cursor to the start of the line.
    FromCursorToStart,

    /// Erase the entire line.
    All,
}

/// See: <https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters>
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SgrParameter {
    /// Reset all attributes.
    Reset,

    /// Bold.
    Bold,

    /// Faint.
    Faint,

    /// Italic.
    Italic,

    /// Underline.
    Underline,

    /// Blink.
    Blink,

    /// Rapid blink.
    RapidBlink,

    /// Reverse video (note: Wikipedia notes inconsistent behaviour). Also
    /// known as "invert."
    ReverseVideo,

    /// Conceal / hide text (note: Wikipedia notes lack of wide support).
    Conceal,

    /// Crossed out. Not supported in Terminal.app.
    CrossedOut,

    /// Select the primary font.
    PrimaryFont,

    /// Select the alternative font at the given index (N-10).
    AlternativeFont(u64),

    /// Fraktur/Gothic mode (note: Wikipedia notes lack of wide support).
    Fraktur,

    /// Double underline. Note: On some systems, this may instead disable
    /// `Bold`.
    DoubleUnderline,

    /// Normal intensity.
    NormalIntensity,

    /// Not italic or blackletter.
    NotItalicOrBlackletter,

    /// Not underlined.
    NotUnderlined,

    /// Steady cursor (not blinking).
    SteadyCursor,

    /// Proportional spacing.
    /// Note: Wikipedia says:
    /// > ITU T.61 and T.416, not known to be used on terminals.
    ProportionalSpacing,

    /// Not reversed.
    /// Presumably undoes `ReverseVideo`, needs testing.
    NotReversed,

    /// Reveal concealed text.
    /// Presumably undoes `Conceal`, needs testing.
    Reveal,

    /// Not crossed out.
    NotCrossedOut,

    /// Set foreground colour to the given colour.
    ForegroundColour(Colour),

    /// Set background colour to the given colour.
    BackgroundColour(Colour),

    /// Set the foreground colour to the given hex colour.
    HexForegroundColour(u32),

    /// Set the background colour to the given hex colour.
    HexBackgroundColour(u32),

    /// Presumably resets to the default foreground colour, needs testing.
    DefaultForegroundColour,

    /// Presumably resets to the default background colour, needs testing.
    DefaultBackgroundColour,

    /// Disable proportional spacing.
    DisableProportionalSpacing,

    /// Set the framing (encircled) attribute.
    Framed,

    /// Set the encircled attribute.
    Encircled,

    /// Set the overlined attribute.
    /// Note: Not supported in Terminal.app.
    /// Note: On some systems, this may instead enable `Bold`.
    Overlined,

    /// Not framed or encircled.
    NotFramedOrEncircled,

    /// Not overlined.
    NotOverlined,

    /// Set the underline colour.
    /// Note: Not in standard, implemented in Kitty, VTE, mintty, iTerm2.
    UnderlineColour(Colour),

    /// Set the underline colour to the given hex colour.
    /// Note: Not in standard, implemented in Kitty, VTE, mintty, iTerm2.
    HexUnderlineColour(u32),

    /// Set the underline colour to the default.
    /// Note: Not in standard, implemented in Kitty, VTE, mintty, iTerm2.
    DefaultUnderlineColour,

    /// Ideogram underline or right side line.
    IdeogramUnderlineOrRightSideLine,

    /// Ideogram double underline or double line on the right side.
    IdeogramDoubleUnderlineOrDoubleLineOnTheRightSide,

    /// Ideogram overline or left side line.
    IdeogramOverlineOrLeftSideLine,

    /// Ideogram double overline or double line on the left side.
    IdeogramDoubleOverlineOrDoubleLineOnTheLeftSide,

    /// Ideogram stress marking.
    IdeogramStressMarking,

    /// Ideogram attributes off.
    /// Resets:
    /// - `IdeogramUnderlineOrRightSideLine`
    /// - `IdeogramDoubleUnderlineOrDoubleLineOnTheRightSide`
    /// - `IdeogramOverlineOrLeftSideLine`
    /// - `IdeogramDoubleOverlineOrDoubleLineOnTheLeftSide`
    /// - `IdeogramStressMarking`.
    IdeogramAttributesOff,

    /// Implemented only in mintty.
    Superscript,

    /// Implemented only in mintty.
    Subscript,

    /// Implemented only in mintty.
    NotSuperscriptOrSubscript,
}

#[cfg(test)]
mod tests {
    use eyre::Result;

    use super::{Ansi, DisplayEraseMode, SgrParameter};

    #[test]
    fn test_works_as_expected() -> Result<()> {
        let mut buffer = String::new();
        Ansi::CursorPosition(0, 0).render(&mut buffer)?;
        assert_eq!("\u{1b}[1;1H", buffer);
        buffer.clear();

        Ansi::CursorDown(1).render(&mut buffer)?;
        assert_eq!("\u{1b}[1B", buffer);
        buffer.clear();

        Ansi::CursorUp(1).render(&mut buffer)?;
        assert_eq!("\u{1b}[1A", buffer);
        buffer.clear();

        Ansi::CursorLeft(1).render(&mut buffer)?;
        assert_eq!("\u{1b}[1D", buffer);
        buffer.clear();

        Ansi::CursorRight(1).render(&mut buffer)?;
        assert_eq!("\u{1b}[1C", buffer);
        buffer.clear();

        Ansi::CursorNextLine(1).render(&mut buffer)?;
        assert_eq!("\u{1b}[1E", buffer);
        buffer.clear();

        Ansi::CursorPreviousLine(1).render(&mut buffer)?;
        assert_eq!("\u{1b}[1F", buffer);
        buffer.clear();

        Ansi::CursorHorizontalAbsolute(1).render(&mut buffer)?;
        assert_eq!("\u{1b}[2G", buffer);
        buffer.clear();

        Ansi::CursorPosition(1, 1).render(&mut buffer)?;
        assert_eq!("\u{1b}[2;2H", buffer);
        buffer.clear();

        Ansi::EraseInDisplay(DisplayEraseMode::All).render(&mut buffer)?;
        assert_eq!("\u{1b}[2J", buffer);
        buffer.clear();

        Ansi::Sgr(vec![SgrParameter::HexForegroundColour(0xDB325C)]).render(&mut buffer)?;
        assert_eq!("\u{1b}[38;2;219;50;92m", buffer);
        buffer.clear();

        Ansi::Sgr(vec![SgrParameter::HexBackgroundColour(0xDB325C)]).render(&mut buffer)?;
        assert_eq!("\u{1b}[48;2;219;50;92m", buffer);
        buffer.clear();

        Ok(())
    }
}
