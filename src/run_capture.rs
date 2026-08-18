use std::path::Path;

use auditorium::{device::CaptureDevice, host::Host};

use crate::{
    RunningFlags, available_path,
    cli::{DeviceTypes, RecordOpts},
    run_playback::find_device,
    term::{TermGuard, capture_control_loop},
};

#[allow(unused)]
pub(crate) struct BuiltCaptDevice {
    pub(crate) device: CaptureDevice,
    pub(crate) name: Option<String>,
    pub(crate) used_default: bool,
}

fn build_device(
    host: &Host,
    device_pos: Option<usize>,
    path: &Path,
) -> anyhow::Result<BuiltCaptDevice> {
    if let Some(device_pos) = device_pos
        && let Some(selected) = find_device(device_pos, DeviceTypes::Capt)?
    {
        let device = host
            .build_capture_device()?
            .device_id(&selected.id)?
            .build(path)?;

        return Ok(BuiltCaptDevice {
            device,
            name: Some(selected.name),
            used_default: false,
        });
    }

    let device = host.build_capture_device()?.build(path)?;
    Ok(BuiltCaptDevice {
        device,
        name: None,
        used_default: device_pos.is_some(),
    })
}

pub(crate) fn run_capture(opts: &RecordOpts, flags: RunningFlags) -> anyhow::Result<()> {
    let host = Host::spawn()?;
    let path = opts
        .output
        .as_deref()
        .map(available_path)
        .unwrap_or_else(|| available_path(Path::new("audctl_recording.wav")));

    let selected = build_device(&host, opts.device, &path)?;

    let _raw_mode = TermGuard::enter()?;

    selected.device.start_device()?;

    capture_control_loop(flags, &selected.device, &path)?;

    host.shutdown()?;

    Ok(())
}
