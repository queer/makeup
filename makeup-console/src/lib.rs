use std::os::unix::prelude::AsRawFd;

use async_recursion::async_recursion;
use eyre::{eyre, Result};
use nix::poll::{poll, PollFd, PollFlags};
use nix::sys::termios;
use nix::sys::termios::InputFlags;
use nix::unistd::isatty;
use tokio::fs::File;

/// - Check if stdin is a terminal (libc::isatty == 1)
///   - If not, open /dev/tty
/// - Put the terminal in raw input mode
/// - Enable TCSADRAIN
/// - Read a byte
///   - If \x1b, csi, so read next byte
///     - If next byte is [, start reading control sequence
///       - Match next byte
///         - A => up
///         - B => down
///         - C => right
///         - D => left
///         - H => home
///         - F => end
///         - Z => shift-tab
///         - _ =>
///           - Match next byte
///             - ~ =>
///               - Match next byte
///                 - 1 => home
///                 - 2 => insert
///                 - 3 => delete
///                 - 4 => end
///                 - 5 => page up
///                 - 6 => page down
///                 - 7 => home
///                 - 8 => end
///                 - Else, the escape sequence was unknown
///             - Else, the escape sequence was unknown
///     - Else, if next byte is not [, bail out on unknown control sequence
///     - Else, if there was no next byte, input was <ESC>
///   - Else, if byte & 224u8 == 192u8, Unicode 2-byte
///   - Else, if byte & 240u8 == 224u8, Unicode 3-byte
///   - Else, if byte & 248u8 == 240u8, Unicode 4-byte
///   - Else:
///     - If byte == \r || byte == \n, <RETURN>
///     - If byte == \t, <TAB>
///     - If byte == \x7f, <BACKSPACE>
///     - If byte == \x1b, <ESC>
///     - If byte == \x01, <HOME>
///     - If byte == \x05, <END>
///     - If byte == \x08, <BACKSPACE>
///     - Else, char = byte
///   - Else, if no byte to read:
///     - If stdin is a terminal, return None
/// - Disable TCSADRAIN
pub async fn next_keypress() -> Result<Keypress> {
    let fd = if isatty(libc::STDIN_FILENO)? {
        libc::STDIN_FILENO
    } else {
        File::open("/dev/tty").await?.as_raw_fd()
    };

    let original_termios = termios::tcgetattr(fd)?;
    let mut termios = original_termios.clone();

    // Note: This is ONLY what termios::cfmakeraw does to input
    termios.input_flags &= !(InputFlags::IGNBRK
        | InputFlags::BRKINT
        | InputFlags::PARMRK
        | InputFlags::ISTRIP
        | InputFlags::INLCR
        | InputFlags::IGNCR
        | InputFlags::ICRNL
        | InputFlags::IXON);
    termios.local_flags &= !(termios::LocalFlags::ECHO
        | termios::LocalFlags::ECHONL
        | termios::LocalFlags::ICANON
        | termios::LocalFlags::ISIG
        | termios::LocalFlags::IEXTEN);
    termios::tcsetattr(fd, termios::SetArg::TCSADRAIN, &termios)?;

    let out = read_next_key(fd).await;

    termios::tcsetattr(fd, termios::SetArg::TCSADRAIN, &original_termios)?;

    out
}

#[async_recursion]
async fn read_next_key(fd: std::os::unix::io::RawFd) -> Result<Keypress> {
    match read_char(fd)? {
        Some('\x1b') => match read_char(fd)? {
            Some('[') => match read_char(fd)? {
                Some('A') => Ok(Keypress::Up),
                Some('B') => Ok(Keypress::Down),
                Some('C') => Ok(Keypress::Right),
                Some('D') => Ok(Keypress::Left),
                Some('H') => Ok(Keypress::Home),
                Some('F') => Ok(Keypress::End),
                Some('Z') => Ok(Keypress::ShiftTab),
                Some(byte3) => match read_char(fd)? {
                    Some('~') => match read_char(fd)? {
                        Some('1') => Ok(Keypress::Home),
                        Some('2') => Ok(Keypress::Insert),
                        Some('3') => Ok(Keypress::Delete),
                        Some('4') => Ok(Keypress::End),
                        Some('5') => Ok(Keypress::PageUp),
                        Some('6') => Ok(Keypress::PageDown),
                        Some('7') => Ok(Keypress::Home),
                        Some('8') => Ok(Keypress::End),
                        Some(byte5) => Ok(Keypress::UnknownSequence(vec![
                            '\x1b', '[', byte3, '~', byte5,
                        ])),
                        None => Ok(Keypress::UnknownSequence(vec!['\x1b', '[', byte3, '~'])),
                    },
                    Some(byte4) => Ok(Keypress::UnknownSequence(vec!['\x1b', '[', byte3, byte4])),
                    None => Ok(Keypress::UnknownSequence(vec!['\x1b', '[', byte3])),
                },
                None => Ok(Keypress::Escape),
            },
            Some(byte) => Ok(Keypress::UnknownSequence(vec!['\x1b', byte])),
            None => Ok(Keypress::Escape),
        },
        Some('\r') | Some('\n') => Ok(Keypress::Return),
        Some('\t') => Ok(Keypress::Tab),
        Some('\x7f') => Ok(Keypress::Backspace),
        Some('\x01') => Ok(Keypress::Home),
        // ^C
        Some('\x03') => Err(ConsoleError::Interrupted.into()),
        Some('\x05') => Ok(Keypress::End),
        Some('\x08') => Ok(Keypress::Backspace),
        Some(byte) => {
            if (byte as u8) & 224u8 == 192u8 {
                let bytes = vec![byte as u8, read_byte(fd)?.unwrap()];
                Ok(Keypress::Char(char_from_utf8(&bytes)?))
            } else if (byte as u8) & 240u8 == 224u8 {
                let bytes: Vec<u8> =
                    vec![byte as u8, read_byte(fd)?.unwrap(), read_byte(fd)?.unwrap()];
                Ok(Keypress::Char(char_from_utf8(&bytes)?))
            } else if (byte as u8) & 248u8 == 240u8 {
                let bytes: Vec<u8> = vec![
                    byte as u8,
                    read_byte(fd)?.unwrap(),
                    read_byte(fd)?.unwrap(),
                    read_byte(fd)?.unwrap(),
                ];
                Ok(Keypress::Char(char_from_utf8(&bytes)?))
            } else {
                Ok(Keypress::Char(byte))
            }
        }
        None => {
            // there is no subsequent byte ready to be read, block and wait for input

            let pollfd = PollFd::new(fd, PollFlags::POLLIN);
            // In THEORY the last error should already be set, since it just happened
            let ret = poll(&mut [pollfd], 0)?;

            // negative timeout means that it will block indefinitely
            if ret < 0 {
                let last_error = std::io::Error::last_os_error();
                if last_error.kind() == std::io::ErrorKind::Interrupted {
                    // User probably hit ^C, oops
                    return Err(ConsoleError::Interrupted.into());
                } else {
                    return Err(ConsoleError::Io(last_error).into());
                }
            }

            read_next_key(fd).await
        }
    }
}

fn read_byte(fd: std::os::unix::io::RawFd) -> Result<Option<u8>> {
    let mut buf = [0u8; 1];

    match nix::unistd::read(fd, &mut buf) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(buf[0])),
        Err(err) => Err(err.into()),
    }
}

fn read_char(fd: std::os::unix::io::RawFd) -> Result<Option<char>> {
    read_byte(fd).map(|byte| byte.map(|byte| byte as char))
}

fn char_from_utf8(buf: &[u8]) -> Result<char> {
    let str = std::str::from_utf8(buf)?;
    let ch = str.chars().next();
    match ch {
        Some(c) => Ok(c),
        None => Err(eyre!("invalid utf8 sequence: {:?}", buf)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keypress {
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
    ShiftTab,
    Insert,
    Delete,
    PageUp,
    PageDown,
    Return,
    Tab,
    Backspace,
    Escape,
    Char(char),
    UnknownSequence(Vec<char>),
}

#[derive(thiserror::Error, Debug)]
pub enum ConsoleError {
    #[error("Interrupted!")]
    Interrupted,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
