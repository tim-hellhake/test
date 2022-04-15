/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::protocol::codec::LumenCacheCodec;
use crate::protocol::encoder::Commands;
use anyhow::{Error, Result};
use async_trait::async_trait;
use futures::stream::SplitSink;
use futures::SinkExt;
use tokio::net::TcpStream;
use tokio_serial::SerialStream;
use tokio_util::codec::Framed;

#[async_trait]
pub trait Transport: Send {
    async fn send(&mut self, command: Commands) -> Result<(), Error>;
}

pub struct SerialTransport {
    sink: SplitSink<Framed<SerialStream, LumenCacheCodec>, Commands>,
}

impl SerialTransport {
    pub fn new(sink: SplitSink<Framed<SerialStream, LumenCacheCodec>, Commands>) -> Self {
        SerialTransport { sink }
    }
}

#[async_trait]
impl Transport for SerialTransport {
    async fn send(&mut self, command: Commands) -> Result<(), Error> {
        log::trace!("Sending {:?}", command);
        self.sink.send(command).await
    }
}

pub struct TcpTransport {
    sink: SplitSink<Framed<TcpStream, LumenCacheCodec>, Commands>,
}

impl TcpTransport {
    pub fn new(sink: SplitSink<Framed<TcpStream, LumenCacheCodec>, Commands>) -> Self {
        TcpTransport { sink }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn send(&mut self, command: Commands) -> Result<(), Error> {
        log::trace!("Sending {:?}", command);
        self.sink.send(command).await
    }
}
