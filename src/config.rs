/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub serial_adapters: Vec<SerialAdapter>,
    #[serde(default)]
    pub tcp_adapters: Vec<TcpAdapter>,
    #[serde(default)]
    pub expert_settings: ExpertSettings,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SerialAdapter {
    #[serde(default = "uuid")]
    pub id: String,
    pub title: String,
    pub port: String,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TcpAdapter {
    #[serde(default = "uuid")]
    pub id: String,
    pub title: String,
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExpertSettings {
    pub max_id: u8,
    pub tx_delay_ms: u64,
    pub response_timeout_ms: u64,
}

impl Default for ExpertSettings {
    fn default() -> Self {
        ExpertSettings {
            max_id: 240,
            tx_delay_ms: 200,
            response_timeout_ms: 500,
        }
    }
}

fn uuid() -> String {
    Uuid::new_v4().to_string()
}
