/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::controller::Controller;
use crate::scenes::activate::ActivateAction;
use crate::scenes::deactivate::DeactivateAction;
use gateway_addon_rust::device::{device, Device, DeviceStructure};
use gateway_addon_rust::{Actions, DeviceDescription};

#[device]
pub struct LumenCacheScene {
    controller: Controller,
    id: u8,
}

impl LumenCacheScene {
    pub fn new(controller: Controller, id: u8) -> Self {
        LumenCacheScene { controller, id }
    }
}

impl DeviceStructure for LumenCacheScene {
    fn id(&self) -> String {
        format!("lumencache-scene-{}", self.id)
    }

    fn description(&self) -> DeviceDescription {
        DeviceDescription::default().title(format!("Scene {}", self.id))
    }

    fn actions(&self) -> Actions {
        vec![
            Box::new(ActivateAction::new(self.id, self.controller.clone())),
            Box::new(DeactivateAction::new(self.id, self.controller.clone())),
        ]
    }
}

impl Device for BuiltLumenCacheScene {}
