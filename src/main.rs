use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, atomic::AtomicBool},
};

use anyhow::{Context, bail};
use clap::Parser;
use ignore::{WalkBuilder, types::TypesBuilder};
use rand::seq::SliceRandom;

use crate::{
    cli::{Command, Opts},
    run_capture::run_capture,
    run_playback::{list_decides, run_playback},
    store::{run_favorite_ops, run_load_ops, run_store_ops},
};

mod cli;
mod run_capture;
mod run_playback;
mod store;
mod term;

fn main() -> anyhow::Result<()> {
    let mut opts = Opts::parse();

    let flags = RunningFlags::default();
    let flags_clone = flags.clone();

    match opts.command {
        Some(Command::Record(opt)) => {
            run_capture(&opt, flags)?;
        }
        Some(Command::List(opt)) => {
            list_decides(opt.typ)?;
        }
        Some(Command::Fav(opts)) => {
            run_favorite_ops(opts)?;
        }
        Some(Command::Store(opts)) => {
            run_store_ops(opts)?;
        }
        None => {
            let paths = match opts.playback.load.take() {
                Some(name) => match run_load_ops(&name)? {
                    Some(path) => {
                        vec![path]
                    }
                    None => {
                        bail!("Name not found in store")
                    }
                },
                None => opts.playback.paths.to_vec(),
            };
            let paths = collect_audio_files(&paths, &opts)?;

            if paths.is_empty() {
                bail!("no supported audio files found");
            }

            run_playback(paths, &opts.playback, flags_clone)?;
        }
    }
    Ok(())
}

#[derive(Clone)]
pub(crate) struct RunningFlags {
    is_paused: Rc<AtomicBool>,
    is_producing: Arc<AtomicBool>,
}

impl Default for RunningFlags {
    fn default() -> Self {
        Self {
            is_paused: Rc::new(AtomicBool::new(false)),
            is_producing: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn collect_audio_files(paths: &[PathBuf], opts: &Opts) -> anyhow::Result<Vec<PathBuf>> {
    let paths = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.to_vec()
    };

    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            if is_supported_audio_file(&path) {
                files.push(path);
            } else {
                eprintln!("unsupported audio file: {}", path.display());
            }

            continue;
        }

        if !path.is_dir() {
            bail!("path does not exist: {}", path.display());
        }

        collect_from_directory(&path, &mut files, opts.playback.recursive)?;
    }

    if opts.playback.shuffle {
        files.shuffle(&mut rand::rng());
    } else {
        // Walking a directory does not guarantee a useful playlist order.
        files.sort();
    }

    Ok(files)
}

fn collect_from_directory(
    directory: &Path,
    files: &mut Vec<PathBuf>,
    recursive: bool,
) -> anyhow::Result<()> {
    let mut types = TypesBuilder::new();

    types.add("audio", "*.mp3")?;
    types.add("audio", "*.flac")?;
    types.add("audio", "*.wav")?;
    types.select("audio");

    let types = types.build()?;

    let mut walker_builder = WalkBuilder::new(directory);
    if !recursive {
        walker_builder.max_depth(Some(1));
    }
    let walker = walker_builder.types(types).build();

    for result in walker {
        let entry =
            result.with_context(|| format!("failed while searching {}", directory.display()))?;

        if entry.file_type().is_some_and(|kind| kind.is_file()) {
            files.push(entry.into_path());
        }
    }

    Ok(())
}

fn is_supported_audio_file(path: &Path) -> bool {
    const EXTENSIONS: &[&str] = &[
        "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "aiff", "aif",
    ];

    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

// Recording over an existing file will leave the recording in a bad state
pub(crate) fn available_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let path = if path.is_dir() {
        path.join("audctl_recording.wav")
    } else {
        path.into()
    };

    let extension = path.extension().unwrap_or_default().to_string_lossy();
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();

    for i in 1.. {
        let file_name = if extension.is_empty() {
            format!("{}_{i}", stem)
        } else {
            format!("{}_{i}.wav", stem)
        };

        let candidate = path.with_file_name(file_name);

        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!()
}
