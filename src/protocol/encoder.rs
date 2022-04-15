/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::protocol::codec::LumenCacheCodec;
use anyhow::{Error, Result};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;

#[derive(Clone, Debug)]
pub enum Commands {
    SetValue(SetValueCommand),
    GetValue(GetValueCommand),
    SetScene(SetSceneCommand),
    ClearScene(ClearSceneCommand),
    ClearScenes(ClearScenesCommand),
    GetScenes(GetScenesCommand),
    ActivateScene(ActivateSceneCommand),
    DeactivateScene(DeactivateSceneCommand),
    GetConfig(GetConfigCommand),
    Hail,
    AssignId(AssignIdCommand),
}

#[derive(Clone, Debug)]
pub struct SetValueCommand {
    pub id: u8,
    pub value: u8,
}

#[derive(Clone, Debug)]
pub struct GetValueCommand {
    pub id: u8,
}

#[derive(Clone, Debug)]
pub struct SetSceneCommand {
    pub id: u8,
    pub scene: u8,
    pub ramp_duration: u8,
    pub level: u8,
}

#[derive(Clone, Debug)]
pub struct ClearSceneCommand {
    pub id: u8,
    pub scene: u8,
}

#[derive(Clone, Debug)]
pub struct ClearScenesCommand {
    pub id: u8,
}

#[derive(Clone, Debug)]
pub struct GetScenesCommand {
    pub id: u8,
}

#[derive(Clone, Debug)]
pub struct ActivateSceneCommand {
    pub id: u8,
}

#[derive(Clone, Debug)]
pub struct DeactivateSceneCommand {
    pub id: u8,
}

#[derive(Clone, Debug)]
pub struct GetConfigCommand {
    pub id: u8,
}

#[derive(Clone, Debug)]
pub struct AssignIdCommand {
    pub id: u8,
    pub serial_number: String,
}

impl Encoder<Commands> for LumenCacheCodec {
    type Error = Error;

    fn encode(&mut self, item: Commands, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            Commands::SetValue(SetValueCommand { id, value }) => {
                dst.put(format!("[{},{}]", id, value).as_bytes())
            }
            Commands::GetValue(GetValueCommand { id }) => {
                dst.put(format!("[{},256]", id).as_bytes())
            }
            Commands::SetScene(SetSceneCommand {
                id,
                scene,
                ramp_duration,
                level,
            }) => dst
                .put(format!("[{},1{:02}{:03}{:03}]", id, scene, ramp_duration, level).as_bytes()),
            Commands::ClearScene(ClearSceneCommand { id, scene }) => {
                dst.put(format!("[{},7{:02}]", id, scene).as_bytes())
            }
            Commands::ClearScenes(ClearScenesCommand { id }) => {
                dst.put(format!("[{},700]", id).as_bytes())
            }
            Commands::GetScenes(GetScenesCommand { id }) => {
                dst.put(format!("[{},10000]", id).as_bytes())
            }
            Commands::ActivateScene(ActivateSceneCommand { id }) => {
                dst.put(format!("[254,{}]", 600 + id as u32).as_bytes())
            }
            Commands::DeactivateScene(DeactivateSceneCommand { id }) => {
                dst.put(format!("[254,{}]", 900 + id as u32).as_bytes())
            }
            Commands::GetConfig(GetConfigCommand { id }) => {
                dst.put(format!("[{},258]", id).as_bytes())
            }
            Commands::Hail => dst.put(format!("[{},{}]", 253, 0).as_bytes()),
            Commands::AssignId(AssignIdCommand { id, serial_number }) => {
                dst.put(format!("[{},{}]", 100_000 + id as u32, serial_number).as_bytes())
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_value() {
        let mut encoder = LumenCacheCodec;
        let mut buf = BytesMut::new();
        encoder
            .encode(
                Commands::SetValue(SetValueCommand { id: 13, value: 37 }),
                &mut buf,
            )
            .unwrap();
        assert_eq!(buf.to_vec(), &b"[13,37]"[..]);
    }

    #[test]
    fn test_get_value() {
        let mut encoder = LumenCacheCodec;
        let mut buf = BytesMut::new();
        encoder
            .encode(Commands::GetValue(GetValueCommand { id: 13 }), &mut buf)
            .unwrap();
        assert_eq!(buf.to_vec(), &b"[13,256]"[..]);
    }

    #[test]
    fn test_get_config() {
        let mut encoder = LumenCacheCodec;
        let mut buf = BytesMut::new();
        encoder
            .encode(Commands::GetConfig(GetConfigCommand { id: 13 }), &mut buf)
            .unwrap();
        assert_eq!(buf.to_vec(), &b"[13,258]"[..]);
    }
}
