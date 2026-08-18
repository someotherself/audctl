use std::{
    io::Write,
    path::Path,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use anyhow::Error;
use auditorium::device::{CaptureDevice, PlaybackDevice};
use crossterm::{
    cursor::MoveToColumn,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};

use crate::{RunningFlags, run_playback::LoadedTrack};

pub(crate) enum PlayerCommand {
    TogglePause,
    Next,
    Previous,
    SeekForward,
    SeekBackward,
    Quit,
}

fn command_from_key(key: KeyEvent) -> Option<PlayerCommand> {
    if !key.is_press() {
        return None;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char(' '), _) => Some(PlayerCommand::TogglePause),
        (KeyCode::Char('n'), _) => Some(PlayerCommand::Next),
        (KeyCode::Char('p'), _) => Some(PlayerCommand::Previous),
        (KeyCode::Right, _) => Some(PlayerCommand::SeekForward),
        (KeyCode::Left, _) => Some(PlayerCommand::SeekBackward),
        (KeyCode::Char('q'), _) => Some(PlayerCommand::Quit),
        (KeyCode::Char('c'), modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
            Some(PlayerCommand::TogglePause)
        }
        _ => None,
    }
}

pub(crate) fn capture_control_loop(
    flags: RunningFlags,
    device: &CaptureDevice,
    path: &Path,
) -> anyhow::Result<()> {
    let path = path.canonicalize().unwrap_or(path.into());
    let mut next_update = Instant::now() + Duration::from_secs(1);
    let mut elapsed = Duration::ZERO;
    let mut started_at = Instant::now();

    loop {
        let timeout = next_update.saturating_duration_since(Instant::now());

        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
            && let Some(command) = command_from_key(key)
        {
            match command {
                PlayerCommand::TogglePause => {
                    let old = flags.is_paused.fetch_xor(true, Ordering::Relaxed);

                    if old {
                        // Was paused, now resuming.
                        started_at = Instant::now();
                        device.resume_recording()?;
                    } else {
                        // Was recording, now pausing.
                        elapsed += started_at.elapsed();
                        device.pause_recording()?;
                    }
                }

                PlayerCommand::Quit => break,

                _ => {}
            }
        }

        if Instant::now() >= next_update {
            let current = if flags.is_paused.load(Ordering::Relaxed) {
                elapsed
            } else {
                elapsed + started_at.elapsed()
            };

            print_recording_progress(current, &path);

            next_update += Duration::from_secs(1);
        }
    }

    Ok(())
}

pub(crate) fn playback_control_loop(
    flags: RunningFlags,
    loaded: &LoadedTrack,
    device: &PlaybackDevice,
) -> anyhow::Result<PlayerCommand> {
    let mut timer: usize = 0;
    let len_sec = loaded.audio.length_seconds()?;

    (|| -> anyhow::Result<PlayerCommand, Error> {
        let res = loop {
            if timer.is_multiple_of(1000) {
                let cur = loaded.audio.cursor_seconds()?.round();
                print_playback_progress(&loaded.path, cur, len_sec as u64);
            }
            timer += 10;

            if event::poll(Duration::from_millis(10))?
                && let Event::Key(key) = event::read()?
                && let Some(command) = command_from_key(key)
            {
                match command {
                    PlayerCommand::TogglePause => {
                        let old = flags.is_paused.fetch_xor(true, Ordering::Relaxed);
                        if old {
                            device.resume_playback()?;
                        } else {
                            device.pause_playback()?;
                        }
                        continue;
                    }
                    PlayerCommand::Next => {
                        break PlayerCommand::Next;
                    }
                    PlayerCommand::Previous => {
                        break PlayerCommand::Previous;
                    }
                    PlayerCommand::SeekForward => {
                        loaded
                            .audio
                            .seek_to_pcm_frame(loaded.audio.cursor_pcm()? + 44_100 * 5)?;
                        continue;
                    }
                    PlayerCommand::SeekBackward => {
                        let cur = loaded.audio.cursor_pcm()?;
                        loaded
                            .audio
                            .seek_to_pcm_frame(cur.saturating_sub(44_100 * 5))?;
                        continue;
                    }
                    PlayerCommand::Quit => {
                        break PlayerCommand::Quit;
                    }
                }
            }

            if flags.is_producing.load(Ordering::Relaxed) || flags.is_paused.load(Ordering::Relaxed)
            {
                continue;
            }

            break PlayerCommand::Next;
        };
        timer = 0;
        Ok(res)
    })()
}

pub(crate) fn print_line(args: std::fmt::Arguments<'_>) {
    let mut stderr = std::io::stderr();
    crossterm::execute!(stderr, crossterm::cursor::MoveToColumn(0)).unwrap();
    eprintln!("{args}");
}

pub(crate) fn move_cursor_to_start() {
    let mut stderr = std::io::stderr();
    crossterm::execute!(
        stderr,
        Clear(ClearType::CurrentLine),
        crossterm::cursor::MoveToColumn(0)
    )
    .unwrap();
}

fn print_playback_progress(path: &Path, cur: f32, total: u64) {
    let mut stderr = std::io::stderr();

    let cur = cur.min(total as f32).max(0.0).round() as u64;

    let cur = format_duration(cur);
    let total = format_duration(total);

    let name = path.file_name().unwrap_or(path.as_os_str());

    crossterm::execute!(stderr, MoveToColumn(0), Clear(ClearType::CurrentLine),).unwrap();

    write!(stderr, "{cur} / {total} | {}", name.display()).unwrap();

    stderr.flush().unwrap();
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn print_recording_progress(elapsed: Duration, path: &Path) {
    let total_seconds = elapsed.as_secs();

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        eprint!(
            "\r{hours:02}:{minutes:02}:{seconds:02} - RECORDING - {}",
            path.display()
        );
    } else {
        eprint!(
            "\r{minutes:02}:{seconds:02} - RECORDING - {}",
            path.display()
        );
    }
}

pub(crate) struct TermGuard;

impl TermGuard {
    pub(crate) fn enter() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        move_cursor_to_start();
        let _ = disable_raw_mode();
    }
}
