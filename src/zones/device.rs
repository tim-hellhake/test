/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::controller::Controller;
use crate::protocol::decoder::Config;
use crate::request::RequestResponse;
use crate::zones::brightness::BrightnessProperty;
use crate::zones::clear_scene::ClearSceneAction;
use crate::zones::clear_scenes::ClearScenesAction;
use crate::zones::on_off::OnOffProperty;
use crate::zones::set_scene::SetSceneAction;
use gateway_addon_rust::error::WebthingsError;
use gateway_addon_rust::{
    device,
    device::{AtType as DeviceType, Device},
    Actions, DeviceDescription, DeviceStructure, Properties,
};
use serde_json::json;

#[device]
pub struct LumenCacheDevice {
    config: Config,
    controller: Controller,
}

impl LumenCacheDevice {
    pub fn new(config: Config, controller: Controller) -> Self {
        LumenCacheDevice { config, controller }
    }
}

impl DeviceStructure for LumenCacheDevice {
    fn id(&self) -> String {
        format!("lumencache-dm-{}", self.config.hardware_serial_number)
    }

    fn description(&self) -> DeviceDescription {
        DeviceDescription::default()
            .at_type(DeviceType::Light)
            .title(format!("Light {}", self.config.id))
    }

    fn properties(&self) -> Properties {
        vec![
            Box::new(OnOffProperty::new(self.controller.clone(), self.config.id)),
            Box::new(BrightnessProperty::new(
                self.controller.clone(),
                self.config.id,
            )),
        ]
    }

    fn actions(&self) -> Actions {
        vec![
            Box::new(SetSceneAction::new(self.config.id, self.controller.clone())),
            Box::new(ClearSceneAction::new(
                self.config.id,
                self.controller.clone(),
            )),
            Box::new(ClearScenesAction::new(
                self.config.id,
                self.controller.clone(),
            )),
        ]
    }
}

impl BuiltLumenCacheDevice {
    pub async fn set_value(&mut self, value: u8) -> Result<(), WebthingsError> {
        self.device_handle
            .get_property("on")
            .unwrap()
            .lock()
            .await
            .property_handle_mut()
            .set_value(Some(json!(value > 0)))
            .await?;

        self.device_handle
            .get_property("brightness")
            .unwrap()
            .lock()
            .await
            .property_handle_mut()
            .set_value(Some(
                json!((value as f64 / 255_f64 * 100_f64).round() as u8),
            ))
            .await
    }

    pub fn request_initial_values(&self) {
        let id = self.config.id;
        let mut controller = self.controller.clone();

        tokio::spawn(async move {
            let receiver = controller.request_current_value(id).await;

            match receiver.await {
                Ok(RequestResponse::Response(_)) => {}
                Ok(RequestResponse::Timeout) => {
                    log::debug!("Failed to request initial value: timeout");
                }
                Err(err) => {
                    log::debug!("Failed to request initial value: {}", err);
                }
            }
        });
    }
}

impl Device for BuiltLumenCacheDevice {}
