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

pub struct SetSceneAction {
    id: u8,
    controller: Controller,
}

impl SetSceneAction {
    pub fn new(id: u8, controller: Controller) -> Self {
        SetSceneAction { id, controller }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneInput {
    scene: u8,
    ramp_duration: f32,
    level_percent: u8,
}

impl Input for SceneInput {
    fn input() -> Option<serde_json::Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "scene": {
                    "type": "integer",
                    "title": "Scene number",
                    "minimum": 1,
                    "maximum": 64,
                },
                "ramp_duration": {
                    "type": "number",
                    "title": "Ramp duration",
                    "unit": "s",
                    "minimum": 0,
                    "maximum": 10,
                    "multipleOf": 0.1
                },
                "level_percent": {
                    "type": "integer",
                    "title": "Level",
                    "unit": "percent",
                    "minimum": 0,
                    "maximum": 100,
                },
            }
        }))
    }

    fn deserialize(value: serde_json::Value) -> Result<Self, WebthingsError> {
        serde_json::from_value(value).map_err(WebthingsError::Serialization)
    }
}

#[async_trait]
impl Action for SetSceneAction {
    type Input = SceneInput;

    fn name(&self) -> String {
        "set-scene".to_owned()
    }

    fn description(&self) -> ActionDescription<Self::Input> {
        ActionDescription::default().title("Set scene")
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
            let SceneInput {
                scene,
                ramp_duration,
                level_percent,
            } = action_handle.input;

            let ramp_duration = (ramp_duration * 10_f32).round() as u8;
            let level = (level_percent as f64 / 100_f64 * 255_f64).round() as u8;

            controller.set_scene(id, scene, ramp_duration, level).await;
            action_handle.finish().await.unwrap();
        });

        Ok(())
    }
}
