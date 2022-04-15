/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::controller::Controller;
use async_trait::async_trait;
use gateway_addon_rust::action::NoInput;
use gateway_addon_rust::{Action, ActionDescription, ActionHandle};

pub struct ActivateAction {
    id: u8,
    controller: Controller,
}

impl ActivateAction {
    pub fn new(id: u8, controller: Controller) -> Self {
        ActivateAction { id, controller }
    }
}

#[async_trait]
impl Action for ActivateAction {
    type Input = NoInput;

    fn name(&self) -> String {
        "activate".to_owned()
    }

    fn description(&self) -> ActionDescription<Self::Input> {
        ActionDescription::default().title("Activate")
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
            controller.activate_scene(id).await;
            action_handle.finish().await.unwrap();
        });

        Ok(())
    }
}
