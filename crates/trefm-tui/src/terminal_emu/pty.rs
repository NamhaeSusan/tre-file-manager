use std::io::Read;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tokio::sync::mpsc;

use super::TerminalMessage;

/// Spawn a shell process in a PTY. Returns (master, reader).
pub fn spawn_shell(
    cwd: &std::path::Path,
    cols: u16,
    rows: u16,
) -> anyhow::Result<(Box<dyn MasterPty + Send>, Box<dyn Read + Send>)> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| anyhow::anyhow!("Failed to open PTY: {e}"))?;

    let shell = detect_shell();
    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(cwd);
    // Set TERM for proper escape sequence support
    cmd.env("TERM", "xterm-256color");

    pair.slave
        .spawn_command(cmd)
        .map_err(|e| anyhow::anyhow!("Failed to spawn shell: {e}"))?;

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| anyhow::anyhow!("Failed to clone PTY reader: {e}"))?;

    Ok((pair.master, reader))
}

/// Detect the user's shell. Tries $SHELL, falls back to /bin/sh.
fn detect_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

/// Spawns a blocking thread to read from PTY and send output via channel.
pub fn spawn_pty_reader(
    mut reader: Box<dyn Read + Send>,
    tx: mpsc::UnboundedSender<TerminalMessage>,
) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    let _ = tx.send(TerminalMessage::Exited(()));
                    break;
                }
                Ok(n) => {
                    if tx.send(TerminalMessage::Output(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.send(TerminalMessage::Exited(()));
                    break;
                }
            }
        }
    });
}

/// Converts a crossterm KeyEvent to the byte sequence expected by the PTY.
pub fn key_to_bytes(key: KeyEvent) -> Vec<u8> {
    // Handle Ctrl+key combinations first
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            let c_lower = c.to_ascii_lowercase();
            if c_lower.is_ascii_lowercase() {
                return vec![c_lower as u8 - b'a' + 1];
            }
        }
    }

    match key.code {
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            s.as_bytes().to_vec()
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        _ => vec![],
    }
}
