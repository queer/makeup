use std::os::fd::BorrowedFd;
use std::os::unix::prelude::AsRawFd;
use std::time::Duration;

use async_recursion::async_recursion;
use eyre::{eyre, Result};
use nix::poll::{poll, PollFd, PollFlags};
use nix::sys::select::FdSet;
use nix::sys::signal::Signal;
use nix::sys::signalfd::SigSet;
use nix::sys::termios;
use nix::sys::termios::InputFlags;
use nix::sys::time::TimeSpec;
use nix::unistd::isatty;
use tokio::fs::File;

#[derive(Debug, Clone)] // TODO: Are clone bounds safe here?
pub struct ConsoleState<'a>(#[doc(hidden)] BorrowedFd<'a>);

pub async fn init() -> Result<ConsoleState<'static>> {
    // Safety: It's impossible for these to not be valid fds
    Ok(ConsoleState(unsafe {
        BorrowedFd::borrow_raw(if isatty(libc::STDIN_FILENO)? {
            std::io::stdin().as_raw_fd()
        } else {
            File::open("/dev/tty").await?.as_raw_fd()
        })
    }))
}

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
pub async fn next_keypress(state: &ConsoleState<'static>) -> Result<Option<Keypress>> {
    let original_termios = termios::tcgetattr(state.0)?;
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
    termios::tcsetattr(state.0, termios::SetArg::TCSADRAIN, &termios)?;

    let out = read_next_key(&state.0).await;

    termios::tcsetattr(state.0, termios::SetArg::TCSADRAIN, &original_termios)?;

    out
}

#[async_recursion]
async fn read_next_key(fd: &BorrowedFd<'_>) -> Result<Option<Keypress>> {
    match read_char(fd)? {
        Some('\x1b') => match read_char(fd)? {
            Some('[') => match read_char(fd)? {
                Some('A') => Ok(Some(Keypress::Up)),
                Some('B') => Ok(Some(Keypress::Down)),
                Some('C') => Ok(Some(Keypress::Right)),
                Some('D') => Ok(Some(Keypress::Left)),
                Some('H') => Ok(Some(Keypress::Home)),
                Some('F') => Ok(Some(Keypress::End)),
                Some('Z') => Ok(Some(Keypress::ShiftTab)),
                Some(byte3) => match read_char(fd)? {
                    Some('~') => match read_char(fd)? {
                        Some('1') => Ok(Some(Keypress::Home)),
                        Some('2') => Ok(Some(Keypress::Insert)),
                        Some('3') => Ok(Some(Keypress::Delete)),
                        Some('4') => Ok(Some(Keypress::End)),
                        Some('5') => Ok(Some(Keypress::PageUp)),
                        Some('6') => Ok(Some(Keypress::PageDown)),
                        Some('7') => Ok(Some(Keypress::Home)),
                        Some('8') => Ok(Some(Keypress::End)),
                        Some(byte5) => Ok(Some(Keypress::UnknownSequence(vec![
                            '\x1b', '[', byte3, '~', byte5,
                        ]))),
                        None => Ok(Some(Keypress::UnknownSequence(vec![
                            '\x1b', '[', byte3, '~',
                        ]))),
                    },
                    Some(byte4) => Ok(Some(Keypress::UnknownSequence(vec![
                        '\x1b', '[', byte3, byte4,
                    ]))),
                    None => Ok(Some(Keypress::UnknownSequence(vec!['\x1b', '[', byte3]))),
                },
                None => Ok(Some(Keypress::Escape)),
            },
            Some(byte) => Ok(Some(Keypress::UnknownSequence(vec!['\x1b', byte]))),
            None => Ok(Some(Keypress::Escape)),
        },
        Some('\r') | Some('\n') => Ok(Some(Keypress::Return)),
        Some('\t') => Ok(Some(Keypress::Tab)),
        Some('\x7f') => Ok(Some(Keypress::Backspace)),
        Some('\x01') => Ok(Some(Keypress::Home)),
        // ^C
        Some('\x03') => Err(ConsoleError::Interrupted.into()),
        Some('\x05') => Ok(Some(Keypress::End)),
        Some('\x08') => Ok(Some(Keypress::Backspace)),
        Some(byte) => {
            if (byte as u8) & 224u8 == 192u8 {
                let bytes = vec![byte as u8, read_byte(fd)?.unwrap()];
                Ok(Some(Keypress::Char(char_from_utf8(&bytes)?)))
            } else if (byte as u8) & 240u8 == 224u8 {
                let bytes: Vec<u8> =
                    vec![byte as u8, read_byte(fd)?.unwrap(), read_byte(fd)?.unwrap()];
                Ok(Some(Keypress::Char(char_from_utf8(&bytes)?)))
            } else if (byte as u8) & 248u8 == 240u8 {
                let bytes: Vec<u8> = vec![
                    byte as u8,
                    read_byte(fd)?.unwrap(),
                    read_byte(fd)?.unwrap(),
                    read_byte(fd)?.unwrap(),
                ];
                Ok(Some(Keypress::Char(char_from_utf8(&bytes)?)))
            } else {
                Ok(Some(Keypress::Char(byte)))
            }
        }
        None => {
            // there is no subsequent byte ready to be read, block and wait for input
            let pollfd = PollFd::new(&fd, PollFlags::POLLIN);
            let ret = poll(&mut [pollfd], 0)?;

            if ret < 0 {
                let last_error = std::io::Error::last_os_error();
                if last_error.kind() == std::io::ErrorKind::Interrupted {
                    // User probably hit ^C, oops
                    return Err(ConsoleError::Interrupted.into());
                } else {
                    return Err(ConsoleError::Io(last_error).into());
                }
            }

            Ok(None)
        }
    }
}

fn read_byte(fd: &BorrowedFd<'_>) -> Result<Option<u8>> {
    let mut buf = [0u8; 1];
    let mut read_fds = FdSet::new();
    read_fds.insert(fd);

    let mut signals = SigSet::empty();
    signals.add(Signal::SIGINT);
    signals.add(Signal::SIGTERM);
    signals.add(Signal::SIGKILL);

    match nix::sys::select::pselect(
        fd.as_raw_fd() + 1,
        Some(&mut read_fds),
        Some(&mut FdSet::new()),
        Some(&mut FdSet::new()),
        Some(&TimeSpec::new(
            0,
            Duration::from_millis(50).as_nanos() as i64,
        )),
        Some(&signals),
    ) {
        Ok(0) => Ok(None),
        Ok(_) => match nix::unistd::read(fd.as_raw_fd(), &mut buf) {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(buf[0])),
            Err(err) => Err(err.into()),
        },
        Err(err) => Err(err.into()),
    }
}

fn read_char(fd: &BorrowedFd<'_>) -> Result<Option<char>> {
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
