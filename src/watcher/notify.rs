use std::path::Path;

use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};
use notify::EventKind::{Create, Modify};
use notify::{Config, Error, Event, RecommendedWatcher, RecursiveMode, Watcher};
use slog::{error, info, trace, warn, Logger};

pub struct FolderWatcher {
    logger: Logger,
    path: Box<Path>,
    files: Vec<String>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl FolderWatcher {
    ///
    /// `Path` must be a folder
    pub fn from_folder(logger: Logger, path: &Path, files: Vec<String>) -> Self {
        let mut new_path = path;
        if path.is_file() {
            warn!(
                logger,
                "Expected path to be a folder, instead `{:?}` is a file! ", path
            );
            new_path = path.parent().expect("Unable to get file parent directory");
        }
        info!(
            logger,
            "NotifyWatcher is register to watch `{:?}` folder, for files :{:?}",
            new_path.to_path_buf().canonicalize(),
            files
        );
        Self {
            logger,
            path: Box::from(new_path),
            files,
        }
    }
    pub fn from_file(logger: Logger, path: &Path) -> Self {
        let mut new_path = path;
        if path.is_file() {
            new_path = path.parent().expect("Unable to get file parent directory");
        }
        let file = path.file_name().unwrap().to_str().unwrap();
        info!(
            logger,
            "NotifyWatcher is register to watch `{:?}` folder, for files :{:?}",
            new_path.to_path_buf().canonicalize().unwrap(),
            file
        );
        Self {
            logger,
            path: Box::from(new_path),
            files: vec![String::from(file)],
        }
    }

    fn async_watcher(
        &self,
    ) -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
        let (mut tx, rx) = channel(1);

        // Automatically select the best implementation for your platform.
        // Can also access each implementation directly e.g. INotifyWatcher.
        let watcher = RecommendedWatcher::new(
            move |res| {
                futures::executor::block_on(async {
                    tx.send(res).await.unwrap();
                })
            },
            Config::default(),
        )?;

        Ok((watcher, rx))
    }

    pub async fn watch(&self, mut callback: impl FnMut()) -> Result<()> {
        let (mut watcher, mut rx) = self.async_watcher()?;

        let files = self.files.clone();
        let logger = self.logger.clone();

        // Add a path to be watched. All files and directories at that path
        // will be monitored for changes.
        watcher.watch(self.path.as_ref(), RecursiveMode::NonRecursive)?;

        while let Some(res) = rx.next().await {
            match res {
                Ok(event) => {
                    trace!(logger, "Folder event"; "event" => format!("{:?}",event.clone()));
                    match event.kind {
                        Modify(_) | Create(_) => {
                            let file_path = event.paths[0].as_path();
                            let file = file_path
                                .file_name()
                                .unwrap()
                                .to_os_string()
                                .into_string()
                                .unwrap();
                            if files.contains(&file) {
                                info!(
                                    logger,
                                    "File {} was changed, calling callback method!", file
                                );
                                (callback)();
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => error!(
                    logger,
                    "Unable to watch folder for changes!"; "error" => format!("{:?}", e)
                ),
            }
        }

        Ok(())
    }
}
