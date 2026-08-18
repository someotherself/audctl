use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "audctl")]
#[command(about = "A simple CLI audio player and recorder")]
pub(crate) struct Opts {
    #[command(flatten)]
    pub(crate) playback: PlaybackOpts,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct PlaybackOpts {
    /// Repeat mode for playback.
    #[arg(long, short = 'r', value_enum, default_value_t = Repeat::Off)]
    pub(crate) repeat: Repeat,

    /// Recursively search directories for audio files.
    #[arg(long, short = 'R')]
    pub(crate) recursive: bool,

    /// Shuffle the playback order.
    #[arg(long, short = 'S')]
    pub(crate) shuffle: bool,

    /// Files or directories to play/search.
    /// If omitted, the current working directory is used.
    #[arg(value_name = "PATH")]
    pub(crate) paths: Vec<PathBuf>,

    /// Number id of the output device. Run list first
    #[arg(long, short = 'd')]
    pub(crate) pos: Option<usize>,

    /// Load audio from store by name
    #[arg(long, short = 'l')]
    pub(crate) load: Option<String>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Record audio to a .wav file
    Record(RecordOpts),
    /// Enumerate playback or capture devices
    List(ListOpts),
    /// Add a folder or file to favorites
    Fav(FavoriteOpts),
    /// Manage favorites
    Store(StoreOpts),
}

#[derive(Debug, clap::Args)]
pub(crate) struct RecordOpts {
    /// Output file to record into.
    #[arg(value_name = "OUTPUT")]
    pub(crate) output: Option<PathBuf>,

    /// Id for the output device (enumerate first)
    #[arg(long, short = 'd')]
    pub(crate) device: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub(crate) enum Repeat {
    #[default]
    Off,
    All,
    One,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ListOpts {
    pub(crate) typ: DeviceTypes,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub(crate) enum DeviceTypes {
    /// Enumerate playback devices
    Play,
    /// Enumerate capture devices
    Capt,
}

#[derive(Debug, clap::Args)]
pub(crate) struct FavoriteOpts {
    /// The path to this entry
    pub(crate) path: PathBuf,
    /// Name to identify this entry
    pub(crate) name: Option<String>,
    /// Overrides an existing entry with this name
    #[arg(long, short = 'f', default_value_t = false)]
    pub(crate) force: bool,
}

#[derive(Debug, clap::Args)]
#[group(required = true, multiple = false)]
pub(crate) struct StoreOpts {
    /// List all entries in the store
    #[arg(long, short = 'l')]
    pub(crate) list: bool,
    /// Remove an entry from favorites by name
    #[arg(long, short = 'r')]
    pub(crate) remove: Option<String>,
    /// Delete all favorites
    #[arg(long, short = 'c')]
    pub(crate) clear: bool,
}
