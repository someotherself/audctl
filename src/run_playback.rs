use std::path::PathBuf;

use auditorium::{
    context::{ContextBuilder, ContextOps, EnumerateControl},
    device::PlaybackDevice,
    device_id::DeviceId,
    device_type::DeviceType,
    host::Host,
    sample_rate::SampleRate,
    sources::audio::Audio,
};

use crate::{
    RunningFlags,
    cli::{DeviceTypes, PlaybackOpts, Repeat},
    term::{PlayerCommand, TermGuard, playback_control_loop, print_line},
};

pub(crate) struct SelectedDevice {
    pub(crate) id: DeviceId,
    pub(crate) name: String,
}

pub(crate) fn list_decides(device_type: DeviceTypes) -> anyhow::Result<()> {
    let ctx = ContextBuilder::new().build()?;
    let is_play = device_type == DeviceTypes::Play;
    if is_play {
        eprintln!("Playback devices found:")
    } else {
        eprintln!("Capture devices found:")
    }
    let mut idx = 1;
    ctx.enumerate_devices(|dev, info| {
        if is_play && dev == DeviceType::Playback {
            eprintln!("{idx}. {}", info.name());
            idx += 1;
        }
        if !is_play && dev == DeviceType::Capture {
            eprintln!("{idx}. {}", info.name());
            idx += 1;
        }
        EnumerateControl::Continue
    })?;
    Ok(())
}
pub(crate) fn find_device(
    dev_pos: usize,
    device_type: DeviceTypes,
) -> anyhow::Result<Option<SelectedDevice>> {
    let ctx = ContextBuilder::new().build()?;
    let wanted_type = match device_type {
        DeviceTypes::Play => DeviceType::Playback,
        DeviceTypes::Capt => DeviceType::Capture,
    };

    let mut index = 1;
    let mut selected = None;

    ctx.enumerate_devices(|device_type, info| {
        if device_type != wanted_type {
            return EnumerateControl::Continue;
        }

        if index == dev_pos {
            selected = Some(SelectedDevice {
                id: info.id().clone(),
                name: info.name().to_owned(),
            });

            return EnumerateControl::Stop;
        }

        index += 1;
        EnumerateControl::Continue
    })?;

    Ok(selected)
}

pub(crate) struct BuiltPlayDevice {
    pub(crate) device: PlaybackDevice,
    pub(crate) name: Option<String>,
    pub(crate) used_default: bool,
}

impl BuiltPlayDevice {
    pub(crate) fn build(
        host: &Host,
        device_pos: Option<usize>,
        flags: RunningFlags,
    ) -> anyhow::Result<BuiltPlayDevice> {
        if let Some(device_pos) = device_pos
            && let Some(selected) = find_device(device_pos, DeviceTypes::Play)?
        {
            let device = host
                .build_playback_device()?
                .clipping(false)?
                .producing_flag(flags.is_producing.clone())?
                .device_id(&selected.id)?
                .build()?;

            return Ok(BuiltPlayDevice {
                device,
                name: Some(selected.name),
                used_default: false,
            });
        }

        let device = host
            .build_playback_device()?
            .producing_flag(flags.is_producing.clone())?
            .sample_rate(SampleRate::Sr44100)?
            .channels(2)?
            .clipping(false)?
            .build()?;
        Ok(BuiltPlayDevice {
            device,
            name: None,
            used_default: device_pos.is_some(),
        })
    }
}

pub(crate) struct LoadedTrack {
    pub(crate) path: PathBuf,
    pub(crate) audio: Audio,
}

impl LoadedTrack {
    fn new(path: PathBuf, device: &PlaybackDevice) -> anyhow::Result<Self> {
        let audio = device.new_audio(&path)?;
        Ok(Self { path, audio })
    }
}

pub(crate) struct Playlist {
    pub(crate) sounds: Vec<PathBuf>,
    pub(crate) cur_pos: usize,
    pub(crate) current: Option<LoadedTrack>,
    pub(crate) next: Option<LoadedTrack>,
    pub(crate) repeat: Repeat,
}

impl Playlist {
    pub(crate) fn new(
        paths: Vec<PathBuf>,
        repeat: Repeat,
        _shuffle: bool,
        device: &PlaybackDevice,
    ) -> anyhow::Result<Self> {
        let next = LoadedTrack::new(paths[0].clone(), device).ok();
        Ok(Self {
            sounds: paths,
            cur_pos: 0,
            current: None,
            next,
            repeat,
        })
    }

    pub(crate) fn move_next(
        &mut self,
        device: &PlaybackDevice,
        backwards: bool,
    ) -> anyhow::Result<&Option<LoadedTrack>> {
        if backwards {
            let prev = self.cur_pos.saturating_sub(2);
            self.current = LoadedTrack::new(self.sounds[prev].clone(), device).ok();
            if prev == self.sounds.len() - 1 {
                self.next = None;
            } else {
                self.cur_pos = prev + 1;
                self.next = LoadedTrack::new(self.sounds[self.cur_pos].clone(), device).ok();
            }
        } else {
            match self.repeat {
                Repeat::Off => {
                    let next = self.next.take();
                    self.current = next;
                    if self.cur_pos == self.sounds.len() - 1 {
                        self.next = None;
                    } else {
                        self.cur_pos += 1;
                        self.next =
                            LoadedTrack::new(self.sounds[self.cur_pos].clone(), device).ok();
                    }
                }
                Repeat::One => {
                    // TODO: Set looping on data source instead
                    // Don't increment cur_pos
                    let next = self.next.take();
                    self.current = next;
                    self.next = LoadedTrack::new(self.sounds[self.cur_pos].clone(), device).ok();
                }
                Repeat::All => {
                    let next = self.next.take();
                    self.current = next;
                    if self.cur_pos == self.sounds.len() - 1 {
                        self.cur_pos = 0;
                    } else {
                        self.cur_pos += 1;
                    }
                    self.next = LoadedTrack::new(self.sounds[self.cur_pos].clone(), device).ok();
                }
            }
        }
        Ok(&self.current)
    }
}

pub(crate) fn run_playback(
    paths: Vec<PathBuf>,
    opts: &PlaybackOpts,
    flags: RunningFlags,
) -> anyhow::Result<()> {
    let host = Host::spawn()?;
    let selected = BuiltPlayDevice::build(&host, opts.pos, flags.clone())?;
    let mut playlist = Playlist::new(paths, opts.repeat, false, &selected.device)?;

    print_track_device_info(&selected, opts);

    let _raw_mode = TermGuard::enter()?;

    selected.device.start_device()?;

    let mut backwards = false;
    while let Some(loaded) = playlist.move_next(&selected.device, backwards)? {
        backwards = false;
        loaded.audio.start_audio()?;


        match playback_control_loop(flags.clone(), loaded, &selected.device)? {
            PlayerCommand::Next => continue,
            PlayerCommand::Previous => {
                backwards = true;
                continue;
            }
            PlayerCommand::Quit => break,
            _ => unreachable!(),
        }
    }

    host.shutdown()?;
    Ok(())
}

fn print_track_device_info(device: &BuiltPlayDevice, opts: &PlaybackOpts) {
    if device.used_default {
        eprintln!(
            "Playback device {} was not found; using the default device.",
            opts.pos.expect("used_default requires a requested device"),
        );
    }

    if let Some(device_name) = &device.name {
        print_line(format_args!("Playing on device {device_name}"));
    }
}
