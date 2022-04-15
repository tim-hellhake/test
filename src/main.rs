/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

mod adapter;
mod config;
mod controller;
mod discovery;
mod protocol;
mod request;
mod scenes;
mod throttle;
mod transport;
mod zones;

use crate::adapter::{BuiltLumenCacheAdapter, LumenCacheAdapter};
use crate::config::Config;
use crate::controller::Controller;
use crate::transport::{SerialTransport, TcpTransport, Transport};
use anyhow::{anyhow, Error, Result};
use as_any::Downcast;
use futures::prelude::stream::SplitStream;
use futures::prelude::*;
use gateway_addon_rust::error::WebthingsError;
use gateway_addon_rust::plugin::{connect, Plugin};
use log::LevelFilter;
use protocol::codec::LumenCacheCodec;
use simple_logger::SimpleLogger;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncRead;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();

    if let Err(err) = run().await {
        log::error!("Could not start adapter: {}", err);
    }

    log::info!("Exiting adapter");
}

async fn run() -> Result<(), Error> {
    let mut plugin = connect("lumencache-adapter").await?;
    log::debug!("Plugin registered");

    let database = plugin.get_config_database();
    let conf: Option<Config> = database.load_config().unwrap();

    if let Some(conf) = conf {
        log::debug!("Loaded config {:?}", conf);
        database.save_config(&conf).unwrap();

        for adapter_config in conf.clone().serial_adapters {
            let id = adapter_config.id.clone();

            let title = adapter_config.title.clone();

            log::debug!("Creating adapter '{}' ({})", title, id);

            let stream = tokio_serial::new(adapter_config.port, 38400)
                .open_native_async()
                .map_err(|err| anyhow!(err))?;
            let stream = Framed::new(stream, LumenCacheCodec);
            let (sink, stream) = stream.split();
            let transport = Arc::new(Mutex::new(SerialTransport::new(sink)));

            create_adapter(conf.clone(), &mut plugin, &id, &title, stream, transport).await?;
        }

        for adapter_config in conf.clone().tcp_adapters {
            let id = adapter_config.id.clone();

            let title = adapter_config.title.clone();

            log::debug!("Creating tcp adapter '{}' ({})", title, id);

            let stream =
                TcpStream::connect(format!("{}:{}", adapter_config.host, adapter_config.port))
                    .await
                    .unwrap();
            let stream = Framed::new(stream, LumenCacheCodec);
            let (sink, stream) = stream.split();
            let transport = Arc::new(Mutex::new(TcpTransport::new(sink)));

            create_adapter(conf.clone(), &mut plugin, &id, &title, stream, transport).await?;
        }
    }

    plugin.event_loop().await;

    Ok(())
}

async fn create_adapter<T>(
    config: Config,
    plugin: &mut Plugin,
    id: &str,
    title: &str,
    mut stream: SplitStream<Framed<T, LumenCacheCodec>>,
    transport: Arc<Mutex<dyn Transport>>,
) -> Result<(), WebthingsError>
where
    T: AsyncRead + Send + 'static,
{
    let controller = Controller::start(config.clone(), transport);
    let adapter = plugin
        .add_adapter(LumenCacheAdapter::new(
            id.to_owned(),
            title.to_owned(),
            config,
            controller.clone(),
        ))
        .await?;

    let adapter_clone = adapter.clone();

    tokio::spawn(async move {
        loop {
            match stream.next().await {
                Some(Ok(response)) => {
                    log::debug!("Received {:?}", response);

                    controller.check_response(&response).await;

                    adapter_clone
                        .lock()
                        .await
                        .downcast_mut::<BuiltLumenCacheAdapter>()
                        .unwrap()
                        .on_message(response)
                        .await;
                }
                Some(Err(err)) => {
                    panic!("Failed to get response: {}", err);
                }
                None => {
                    panic!("End of stream");
                }
            }
        }
    });

    if let Err(err) = adapter
        .lock()
        .await
        .on_start_pairing(Duration::from_secs(120))
        .await
    {
        log::error!("Failed to start pairing: {}", err);
    }

    Ok(())
}
