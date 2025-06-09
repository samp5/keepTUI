use std::{fs::{create_dir, OpenOptions}, io::Write, path::PathBuf};

use crate::config::{self, Config, RuntimeOptions};
use anyhow::{Context, Result as AResult};
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult}
};

use super::{note::NoteCollection, tag::TagCollection, App};

pub struct AppData();

impl AppData {
    pub fn read(config: &Config, runtime_opts: &RuntimeOptions) ->AResult<(NoteCollection, TagCollection)> {
        let data_path = config.data_path();

        if !data_path.exists() {
            if runtime_opts.local && !runtime_opts.local_create {
                return Err(IOError::new(IOErrorKind::Other, "Not creating data directory in current directory, run again with `-L` or `--local_force` to create").into());
            } else {
                create_dir(data_path).context(format!("failed to create path {:#?}", data_path))?
            }
        }


        Ok((
            AppData::read_file::<NoteCollection>(config.data_path().join("notes"))?,
            AppData::read_file(config.data_path().join("tags"))?
        ))
    }
    pub fn write(app: &App) -> IOResult<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(app.config.data_path().join("notes"))?;

        let serialized = serde_json::to_string(&app.notes)?;
        file.write_all(serialized.as_bytes())?;

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(app.config.data_path().join("tags"))?;

        let serialized = serde_json::to_string(&app.tags)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }


    fn read_file<T: Default + DeserializeOwned >(path: PathBuf) -> AResult<T> {
        let tag_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true) // for creation requirement
            .open(&path)
            .context(format!("Could not open {:#?}", path))?;

        if tag_file
            .metadata()
            .context(format!("Could not open {:#?}", path))?
            .len()
            == 0
        {
            Ok(T::default())
        } else {
            Ok(serde_json::from_reader(tag_file).context(format!(
                "serde_json failed to read 'tags' file in {:#?}", path
            ))?)
        }
    }

}


