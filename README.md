# A simple cli app for audio playback and recording

## Supported targets

`audctl` supports the following targets:
- aarch64-apple-darwin
- aarch64-pc-windows-msvc
- aarch64-unknown-linux-gnu
- x86_64-pc-windows-gnu
- x86_64-pc-windows-msvc
- x86_64-unknown-linux-gnu

## Supported audio formats for playback

- AAC
- ALAC
- FLAC
- MP3
- Opus
- Vorbis
- WAV / PCM
- ADPCM
- WavPack

## Supported audio formats for recording

- WAV

## Usage

### Controls during playback
| Key      | Action                   |
| -------- | ------------------------ |
| `Space`  | Pause / resume playback  |
| `Ctrl+C` | Pause / resume playback  |
| `←`      | Seek backward            |
| `→`      | Seek forward             |
| `n`      | Skip to the next track   |
| `p`      | Go to the previous track |
| `q`      | Quit playback            |

### Controls during recording
| Key      | Action                   |
| -------- | ------------------------ |
| `Space`  | Pause / resume recording |
| `Ctrl+C` | Pause / resume recording |
| `q`      | Stop and quit            |

### Playback
```bash
# Play everything in the current directory (no recursion)
audctl

# Play audio files
audctl song.mp3

# Play everything in a directory
audctl ~/Music

# Search subdirectories as well
audctl ~/Music --recursive

# Shuffle playback
audctl ~/Music --shuffle

# Repeat the entire playlist
audctl ~/Music --repeat all

# Repeat the current track
audctl song.mp3 --repeat one
```

### Selecting a playback device
```bash
# List playback devices
audctl list play

# Play using a specific device
audctl --device 2 ~/Music
```

### Recording device and relecting a recording device
```bash
# Record using the default capture device
audctl record recording.wav

# If no file name is provided, "audctl_recording.wav" will be used
audctl record

# List capture devices
audctl list capt

# Record from a specific capture device
audctl record recording.wav --device 1
```

### Manage favorites (files or folders)
```bash
# Add a directory to favorites
audctl fav ~/Music "Music"

# Add a specific file
audctl fav ~/Music/album

# Replace an existing favorite
audctl fav ~/Music --force "Music"

# List favorites
audctl store --list

# Remove a favorite
audctl store --remove "Music"

# Remove all favorites
audctl store --clear
```

### Play from favorites
```bash
# Play a stored entry (file or directory)
audctl --load "Music"
```
