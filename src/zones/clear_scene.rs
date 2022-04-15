/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::controller::Controller;
use async_trait::async_trait;
use gateway_addon_rust::action::Input;
use gateway_addon_rust::error::WebthingsError;
use gateway_addon_rust::{Action, ActionDescription, ActionHandle};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct ClearSceneAction {
    id: u8,
    controller: Controller,
}

impl ClearSceneAction {
    pub fn new(id: u8, controller: Controller) -> Self {
        ClearSceneAction { id, controller }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClearSceneInput {
    scene: u8,
}

impl Input for ClearSceneInput {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "scene": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 64,
                },
            }
        }))
    }

    fn deserialize(value: serde_json::Value) -> Result<Self, WebthingsError> {
        serde_json::from_value(value).map_err(WebthingsError::Serialization)
    }
}

#[async_trait]
impl Action for ClearSceneAction {
    type Input = ClearSceneInput;

    fn name(&self) -> String {
        "clear-scene".to_owned()
    }

    fn description(&self) -> ActionDescription<Self::Input> {
        ActionDescription::default().title("Clear scene")
    }

    async fn perform(
        &mut self,
        mut action_handle: ActionHandle<Self::Input>,
    ) -> Result<(), String> {
        action_handle.start().await.unwrap();

        log::debug!(
            "Performing {} action with {:?}",
            self.name(),
            action_handle.input
        );

        let id = self.id;
        let mut controller = self.controller.clone();

        tokio::spawn(async move {
            let ClearSceneInput { scene } = action_handle.input;

            controller.clear_scene(id, scene).await;
            action_handle.finish().await.unwrap();
        });

        Ok(())
    }
}
