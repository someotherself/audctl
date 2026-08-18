use anyhow::bail;
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::cli::{FavoriteOpts, StoreOpts};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    favorite_folders: Vec<FavoriteFolder>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FavoriteFolder {
    name: String,
    path: PathBuf,
}

fn load_config() -> Result<UserConfig, confy::ConfyError> {
    confy::load("audctl", None)
}

fn save_config(config: &UserConfig) -> Result<(), confy::ConfyError> {
    confy::store("audctl", None, config)
}

pub fn run_favorite_ops(opts: FavoriteOpts) -> anyhow::Result<()> {
    let path = opts.path.canonicalize().unwrap();
    let name = match opts.name {
        Some(name) => name,
        None => path.file_name().unwrap().to_str().unwrap().to_owned(),
    };

    let mut cfg = load_config()?;

    if let Some(folder) = cfg.favorite_folders.iter_mut().find(|f| f.name == name) {
        if opts.force {
            *folder = FavoriteFolder { name, path };
        } else {
            bail!("Name already exists");
        }
    } else {
        cfg.favorite_folders.push(FavoriteFolder { name, path });
    }

    save_config(&cfg)?;

    Ok(())
}

pub fn run_load_ops(name: &str) -> anyhow::Result<Option<PathBuf>> {
    let cfg = load_config()?;

    let folder = cfg
        .favorite_folders
        .iter()
        .find(|f| f.name == name)
        .map(|f| f.path.clone());

    Ok(folder)
}

pub fn run_store_ops(opts: StoreOpts) -> anyhow::Result<()> {
    let mut cfg = load_config()?;
    if opts.list {
        if cfg.favorite_folders.is_empty() {
            eprintln!("Store is empty");
            return Ok(());
        }

        let mut stdout = std::io::stderr();

        let width = cfg
            .favorite_folders
            .iter()
            .map(|folder| folder.name.len())
            .max()
            .unwrap_or(0);

        crossterm::execute!(
            stdout,
            SetForegroundColor(Color::DarkGrey),
            Print(format!("{:>2}  ", "ID")),
            Print(format!("{:<width$}", "Name", width = width)),
            Print("  Path\n"),
            ResetColor,
        )?;

        for (i, folder) in cfg.favorite_folders.iter().enumerate() {
            crossterm::execute!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("{:>2}  ", i + 1)),
                SetForegroundColor(Color::Cyan),
                Print(format!("{:<width$}", folder.name, width = width)),
                SetForegroundColor(Color::DarkGrey),
                Print("  "),
                SetForegroundColor(Color::White),
                Print(folder.path.display()),
                ResetColor,
                Print('\n'),
            )?;
        }
        return Ok(());
    }

    if opts.clear {
        cfg.favorite_folders.clear();
        save_config(&cfg)?;
        eprintln!("Store cleared");
        return Ok(());
    }

    if let Some(name) = opts.remove
        && let Some(idx) = cfg.favorite_folders.iter().position(|f| f.name == name)
    {
        cfg.favorite_folders.swap_remove(idx);
        save_config(&cfg)?;
    }

    Ok(())
}
