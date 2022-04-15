/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::controller::Controller;
use crate::discovery::Discovery;
use crate::protocol::decoder::{Config, Response, Scene, Value};
use crate::scenes::scene::LumenCacheScene;
use crate::zones::device::{BuiltLumenCacheDevice, LumenCacheDevice};
use as_any::Downcast;
use async_trait::async_trait;
use gateway_addon_rust::Device;
use gateway_addon_rust::{adapter, Adapter, AdapterStructure};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[adapter]
pub struct LumenCacheAdapter {
    id: String,
    title: String,
    controller: Controller,
    discovery: Discovery,
    devices: HashMap<u8, Arc<Mutex<Box<dyn Device>>>>,
    scenes: HashMap<u8, Arc<Mutex<Box<dyn Device>>>>,
}

impl LumenCacheAdapter {
    pub fn new(id: String, title: String, config: crate::Config, controller: Controller) -> Self {
        LumenCacheAdapter {
            id,
            title,
            controller: controller.clone(),
            discovery: Discovery::new(config, controller),
            devices: HashMap::new(),
            scenes: HashMap::new(),
        }
    }
}

impl AdapterStructure for LumenCacheAdapter {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn name(&self) -> String {
        self.title.clone()
    }
}

impl BuiltLumenCacheAdapter {
    pub async fn on_message(&mut self, result: Response) {
        match result {
            Response::Value(Value { id, value }) => {
                self.on_value_update(id, value).await;
            }
            Response::Config(config) => {
                self.on_config_update(config).await;
            }
            Response::Scene(scene) => {
                self.on_scene_update(scene).await;
            }
            _ => {}
        }
    }

    pub async fn on_value_update(&mut self, id: u8, value: u8) {
        match self.devices.get(&id) {
            Some(device) => {
                device
                    .lock()
                    .await
                    .downcast_mut::<BuiltLumenCacheDevice>()
                    .unwrap()
                    .set_value(value)
                    .await
                    .unwrap();
            }
            None => {
                log::debug!("No device with the id {} found", id);
            }
        }
    }

    pub async fn on_config_update(&mut self, config: Config) {
        let id = config.id;

        if id == 0 {
            return;
        };

        #[allow(clippy::map_entry)]
        if !self.devices.contains_key(&id) {
            log::debug!("Creating device {}", id);
            let controller = self.controller.clone();

            let device = self
                .adapter_handle
                .add_device(LumenCacheDevice::new(config, controller.clone()))
                .await
                .unwrap();

            device
                .lock()
                .await
                .downcast_ref::<BuiltLumenCacheDevice>()
                .unwrap()
                .request_initial_values();

            self.devices.insert(id, device);

            let mut controller = self.controller.clone();

            tokio::spawn(async move {
                controller.request_scenes(id).await;
            });
        }
    }

    pub async fn on_scene_update(&mut self, scene: Scene) {
        if scene.level < 0 || scene.duration < 0 {
            return;
        }

        let id = scene.scene;

        #[allow(clippy::map_entry)]
        if !self.scenes.contains_key(&id) {
            log::debug!("Creating scene {}", id);
            let controller = self.controller.clone();

            let device = self
                .adapter_handle
                .add_device(LumenCacheScene::new(controller.clone(), id))
                .await
                .unwrap();

            self.scenes.insert(id, device);
        }
    }
}

#[async_trait]
impl Adapter for BuiltLumenCacheAdapter {
    async fn on_start_pairing(&mut self, _timeout: Duration) -> Result<(), String> {
        self.discovery.start().await;
        Ok(())
    }
}
