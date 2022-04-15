/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::protocol::codec::LumenCacheCodec;
use anyhow::{anyhow, Error, Result};
use bytes::{Buf, BytesMut};
use std::fmt::Debug;
use tokio_util::codec::Decoder;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Response {
    Value(Value),
    SerialNumber(SerialNumber),
    Scene(Scene),
    Config(Config),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Value {
    pub id: u8,
    pub value: u8,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SerialNumber {
    pub id: u8,
    pub serial_number: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Scene {
    pub id: u8,
    pub scene: u8,
    pub level: i16,
    pub duration: i16,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Config {
    pub id: u8,
    pub hardware_type: u8,
    pub hardware_version: u8,
    pub firmware_version: String,
    pub hardware_serial_number: String,
    pub mode: u8,
    pub dimming_curve: u8,
    pub pwm_frequency: u8,
    pub minimum_output_pwm: u8,
    pub maximum_output_pwm: u8,
    pub resume_level: u8,
    pub ramp_duration: u8,
    pub motion_sensor_enable: u8,
    pub mode_6_alternate_actions: u8,
    pub inverted_output: u8,
}

const BEGIN_RESPONSE: u8 = b'(';
const END_RESPONSE: u8 = b')';
const BEGIN_CONFIG: u8 = b'{';
const END_CONFIG: u8 = b'}';

impl Decoder for LumenCacheCodec {
    type Item = Response;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(LumenCacheCodec::parse(buf))
    }
}

impl LumenCacheCodec {
    pub fn parse(buf: &mut BytesMut) -> Option<Response> {
        let begin = buf
            .iter()
            .position(|b| *b == BEGIN_RESPONSE || *b == BEGIN_CONFIG)?;

        match buf[begin] {
            BEGIN_RESPONSE => {
                let end = buf.iter().position(|b| *b == END_RESPONSE)?;
                let response = LumenCacheCodec::extract(buf, begin, end)?;

                match LumenCacheCodec::parse_value(&response) {
                    Ok(value) => Some(value),
                    Err(err) => {
                        log::error!("Failed to parse {}: {}", response, err);
                        None
                    }
                }
            }
            BEGIN_CONFIG => {
                let end = buf.iter().position(|b| *b == END_CONFIG)?;
                let response = LumenCacheCodec::extract(buf, begin, end)?;
                let parts: Vec<&str> = response.split(',').map(|s| s.trim()).collect();

                match parts.len() {
                    2 => match LumenCacheCodec::parse_serial(parts) {
                        Ok(value) => Some(value),
                        Err(err) => {
                            log::error!("Failed to parse {}: {}", response, err);
                            None
                        }
                    },
                    4 => match LumenCacheCodec::parse_scene(parts) {
                        Ok(value) => Some(value),
                        Err(err) => {
                            log::error!("Failed to parse {}: {}", response, err);
                            None
                        }
                    },
                    15 => match LumenCacheCodec::parse_config(parts) {
                        Ok(value) => Some(value),
                        Err(err) => {
                            log::error!("Failed to parse {}: {}", response, err);
                            None
                        }
                    },
                    nr => {
                        log::warn!("Unexpected number of parts ({}) in config response", nr);
                        None
                    }
                }
            }
            _ => None,
        }
    }

    pub fn parse_value(response: &str) -> Result<Response> {
        let parts: Vec<&str> = response.split(',').map(|s| s.trim()).collect();

        if parts.len() != 2 {
            return Err(anyhow!(
                "Expected {:?} to have 2 parts but has {}",
                parts,
                parts.len()
            ));
        }

        Ok(Response::Value(Value {
            id: parts[0].parse()?,
            value: parts[1].parse()?,
        }))
    }

    pub fn parse_serial(parts: Vec<&str>) -> Result<Response> {
        assert_length(&parts, 2)?;

        Ok(Response::SerialNumber(SerialNumber {
            id: parts[0].parse()?,
            serial_number: parts[1].to_owned(),
        }))
    }

    pub fn parse_scene(parts: Vec<&str>) -> Result<Response> {
        assert_length(&parts, 4)?;

        Ok(Response::Scene(Scene {
            id: parts[0].parse()?,
            scene: parts[1].parse()?,
            level: parts[2].parse()?,
            duration: parts[3].parse()?,
        }))
    }

    pub fn parse_config(parts: Vec<&str>) -> Result<Response> {
        assert_length(&parts, 15)?;

        Ok(Response::Config(Config {
            id: parts[0].parse()?,
            hardware_type: parts[1].parse()?,
            hardware_version: parts[2].parse()?,
            firmware_version: parts[3].to_owned(),
            hardware_serial_number: parts[4].to_owned(),
            mode: parts[5].parse()?,
            dimming_curve: parts[6].parse()?,
            pwm_frequency: parts[7].parse()?,
            minimum_output_pwm: parts[8].parse()?,
            maximum_output_pwm: parts[9].parse()?,
            resume_level: parts[10].parse()?,
            ramp_duration: parts[11].parse()?,
            motion_sensor_enable: parts[12].parse()?,
            mode_6_alternate_actions: parts[13].parse()?,
            inverted_output: parts[14].parse()?,
        }))
    }

    pub fn extract(buf: &mut BytesMut, begin: usize, end: usize) -> Option<String> {
        let mut bytes = buf.split_to(end + 1);
        bytes.advance(begin + 1);
        bytes.truncate(bytes.len() - 1);

        match String::from_utf8(bytes.to_vec()) {
            Ok(response) => Some(response),
            Err(err) => {
                log::error!("Could not decode response {:?}: {}", bytes, err);
                None
            }
        }
    }
}

fn assert_length<T: Debug>(parts: &[T], length: usize) -> Result<()> {
    if parts.len() != length {
        return Err(anyhow!(
            "Expected {:?} to have {} parts but has {}",
            parts,
            length,
            parts.len()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_begin() {
        let mut buf = BytesMut::from(&b"foo"[..]);
        assert_eq!(LumenCacheCodec::parse(&mut buf), None);
        assert_eq!(buf.to_vec(), &b"foo"[..]);
    }

    #[test]
    fn test_no_end() {
        let mut buf = BytesMut::from(&b"(foo"[..]);
        assert_eq!(LumenCacheCodec::parse(&mut buf), None);
        assert_eq!(buf.to_vec(), &b"(foo"[..]);
    }

    #[test]
    fn test_chars_before_begin() {
        let mut buf = BytesMut::from(&b"bar(13,37)baz"[..]);
        assert_eq!(
            LumenCacheCodec::parse(&mut buf),
            Some(Response::Value(Value { id: 13, value: 37 }))
        );
        assert_eq!(buf.to_vec(), &b"baz"[..]);
    }

    #[test]
    fn test_multiple_delimiters() {
        let mut buf = BytesMut::from(&b"(13,,37)"[..]);
        assert_eq!(LumenCacheCodec::parse(&mut buf), None);
        assert_eq!(buf.to_vec(), &b""[..]);
    }

    #[test]
    fn test_spaces() {
        let mut buf = BytesMut::from(&b" ( 13 , 37 ) "[..]);
        assert_eq!(
            LumenCacheCodec::parse(&mut buf),
            Some(Response::Value(Value { id: 13, value: 37 }))
        );
        assert_eq!(buf.to_vec(), &b" "[..]);
    }

    #[test]
    fn test_exact_match() {
        let mut buf = BytesMut::from(&b"(13,37)"[..]);
        assert_eq!(
            LumenCacheCodec::parse(&mut buf),
            Some(Response::Value(Value { id: 13, value: 37 }))
        );
        assert_eq!(buf.to_vec(), &b""[..]);
    }

    #[test]
    fn test_multiple() {
        let mut buf = BytesMut::from(&b"(13,37)(42,24)"[..]);
        assert_eq!(
            LumenCacheCodec::parse(&mut buf),
            Some(Response::Value(Value { id: 13, value: 37 }))
        );
        assert_eq!(buf.to_vec(), &b"(42,24)"[..]);
    }

    #[test]
    fn test_config() {
        let mut buf =
            BytesMut::from(&b"{1,7,3,123456.12,0123456789ABCDEF0123,6,1,2,30,220,255,0,1,0,0}"[..]);
        assert_eq!(
            LumenCacheCodec::parse(&mut buf),
            Some(Response::Config(Config {
                id: 1,
                hardware_type: 7,
                hardware_version: 3,
                firmware_version: String::from("123456.12"),
                hardware_serial_number: String::from("0123456789ABCDEF0123"),
                mode: 6,
                dimming_curve: 1,
                pwm_frequency: 2,
                minimum_output_pwm: 30,
                maximum_output_pwm: 220,
                resume_level: 255,
                ramp_duration: 0,
                motion_sensor_enable: 1,
                mode_6_alternate_actions: 0,
                inverted_output: 0
            }))
        );
        assert_eq!(buf.to_vec(), &b""[..]);
    }
}
