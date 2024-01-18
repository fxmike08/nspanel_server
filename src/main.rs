extern crate chrono;
extern crate indexmap;
extern crate lazy_static;
extern crate rumqttc;
extern crate serde_json;
extern crate slog;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fs, thread};

use slog::{debug, error, info, o, warn, Drain, Logger};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::config::schema::{Config, Connectivity, Device};
use crate::homeassitant::hass::start_hass;
use crate::mqttc::MqttC;
use crate::utils::redact;
use crate::watcher::notify::FolderWatcher;

mod command;
mod config;
mod homeassitant;
mod mqttc;
mod pages;
mod utils;
mod watcher;

fn get_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    Logger::root(
        drain,
        o!("component" => "nspanel_server", "version" => "0.1"),
    )
}

#[tokio::main]
async fn main() {
    let logger = get_logger();

    let (files, path, config) = get_config(&logger);

    let folder_watcher = FolderWatcher::from_folder(logger.clone(), path, files);

    futures::executor::block_on(async move {
        let shutdown = Arc::new(AtomicBool::new(false));
        let (mqqt2hass_sender, mqqt2hass_receiver) = mpsc::channel::<(String, String)>(100);
        let (hass2mqtt_sender, hass2mqtt_receiver) = mpsc::channel::<(String, String)>(100);
        let mqqt2hass_receiver = Arc::new(Mutex::new(mqqt2hass_receiver));
        let hass2mqtt_receiver = Arc::new(Mutex::new(hass2mqtt_receiver));

        info!(logger, "Starting Mqtt Client thread.");
        let mut mqtt_handle = start_mqtt(
            MqttC::new(config.clone(), logger.clone()),
            shutdown.clone(),
            (hass2mqtt_sender.clone(), mqqt2hass_receiver.clone()),
        );
        let mut hass_handle = start_hass(
            config.clone(),
            logger.clone(),
            shutdown.clone(),
            (mqqt2hass_sender.clone(), hass2mqtt_receiver.clone()),
        );

        let logger_clonned = logger.clone();
        if let Err(e) = folder_watcher
            .watch(move || {
                let shutdown_cloned = shutdown.clone();
                let logger = &logger_clonned;
                info!(logger, "Configuration file has changed ! Restarting.");
                shutdown_cloned.store(true, Ordering::SeqCst);
                // Waiting for Mqtt to gracefully shutdown.
                while !mqtt_handle.is_finished() {
                    debug!(logger, "Waiting for Mqtt Client thread to stop.");
                    thread::sleep(Duration::from_millis(1000));
                }
                // Waiting for Hass to gracefully shutdown.
                while !hass_handle.is_finished() {
                    debug!(logger, "Waiting for HASS thread to stop.");
                    thread::sleep(Duration::from_millis(1000));
                }

                shutdown_cloned.store(false, Ordering::SeqCst);
                let (_, _, config) = get_config(&logger);
                info!(logger, "Starting Mqtt Client thread.");
                mqtt_handle = start_mqtt(
                    MqttC::new(config.clone(), logger.clone()),
                    shutdown.clone(),
                    (hass2mqtt_sender.clone(), mqqt2hass_receiver.clone()),
                );
                info!(logger, "Starting HASS Client thread.");
                hass_handle = start_hass(
                    config.clone(),
                    logger.clone(),
                    shutdown.clone(),
                    (mqqt2hass_sender.clone(), hass2mqtt_receiver.clone()),
                );
            })
            .await
        {
            error!(
                logger,
                "Unable to start watching on specified folder. Reason: {:?}", e
            );
        }
    });
}

fn start_mqtt(
    mut mqtt_client: MqttC,
    shutdown: Arc<AtomicBool>,
    channel: (
        Sender<(String, String)>,
        Arc<Mutex<Receiver<(String, String)>>>,
    ),
) -> JoinHandle<()> {
    let sender_to_hass = channel.0;
    let receiver_from_hass = channel.1;
    tokio::spawn(async move {
        mqtt_client
            .subscribe(shutdown, (sender_to_hass, receiver_from_hass))
            .await;
    })
}

fn get_config(logger: &Logger) -> (Vec<String>, &Path, Config) {
    let files = vec![
        env::var("config").unwrap_or("config.yaml".into()),
        env::var("connectivity").unwrap_or("connectivity.yaml".into()),
        env::var("icons").unwrap_or("icons.yaml".into()),
    ];

    let path = Path::new("./config/");

    if !path.exists() {
        warn!(logger, "File path does not exist");
    }
    let config = fs::read_to_string(path.join(&files[0])).expect("Unable to read config file!");
    let devices: BTreeMap<String, Device> =
        serde_yaml::from_str::<BTreeMap<String, Device>>(&config).unwrap();

    info!(logger, "Deserialize yaml: {:?}", devices);

    let connection_config =
        fs::read_to_string(path.join(&files[1])).expect("Unable to read connectivity file!");
    let connectivity: Connectivity =
        serde_yaml::from_str::<Connectivity>(&connection_config).unwrap();

    let icons_config =
        fs::read_to_string(path.join(&files[2])).expect("Unable to read connectivity file!");
    let icons: BTreeMap<String, char> =
        serde_yaml::from_str::<BTreeMap<String, char>>(&icons_config).unwrap();

    // Redact sensitive data
    info!(
        logger,
        "Deserialize yaml: {:?}",
        redact(
            format!("{:?}", connectivity).as_str(),
            r##"token:\s\"(.*?)\"|password:\s\"(.*?)\""##
        )
    );

    let config = Config {
        connectivity,
        devices,
        icons,
    };
    (files, path, config)
}