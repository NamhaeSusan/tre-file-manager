pub mod pty;
pub mod screen;
pub mod widget;

use std::io::Write;

use crossterm::event::KeyEvent;
use tokio::sync::mpsc;

/// Messages from the PTY reader thread to the main loop.
pub enum TerminalMessage {
    /// Raw bytes from PTY stdout.
    Output(Vec<u8>),
    /// Shell process exited.
    Exited(()),
}

/// Combines PTY handle and virtual screen.
pub struct TerminalEmulator {
    pub pty_master: Box<dyn portable_pty::MasterPty + Send>,
    pub screen: screen::ScreenState,
    pub last_size: (u16, u16),
    writer: Box<dyn Write + Send>,
}

impl TerminalEmulator {
    /// Spawns a shell and starts the PTY reader thread.
    pub fn spawn(
        cwd: &std::path::Path,
        cols: u16,
        rows: u16,
        tx: mpsc::UnboundedSender<TerminalMessage>,
    ) -> anyhow::Result<Self> {
        let (master, reader) = pty::spawn_shell(cwd, cols, rows)?;
        pty::spawn_pty_reader(reader, tx);
        let writer = master.take_writer().map_err(|e| anyhow::anyhow!("{e}"))?;
        let screen = screen::ScreenState::new(rows, cols);
        Ok(Self {
            pty_master: master,
            screen,
            last_size: (cols, rows),
            writer,
        })
    }

    /// Writes a key event to the PTY.
    pub fn write_key(&mut self, key: KeyEvent) {
        let bytes = pty::key_to_bytes(key);
        if !bytes.is_empty() {
            let _ = self.writer.write_all(&bytes);
            let _ = self.writer.flush();
        }
    }

    /// Writes raw bytes to the PTY (e.g. for cd sync).
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    /// Resize the PTY and virtual screen.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if (cols, rows) != self.last_size && cols > 0 && rows > 0 {
            let _ = self.pty_master.resize(portable_pty::PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
            self.screen.resize(rows, cols);
            self.last_size = (cols, rows);
        }
    }
}
