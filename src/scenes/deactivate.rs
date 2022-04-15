/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::controller::Controller;
use async_trait::async_trait;
use gateway_addon_rust::action::NoInput;
use gateway_addon_rust::{Action, ActionDescription, ActionHandle};

pub struct DeactivateAction {
    id: u8,
    controller: Controller,
}

impl DeactivateAction {
    pub fn new(id: u8, controller: Controller) -> Self {
        DeactivateAction { id, controller }
    }
}

#[async_trait]
impl Action for DeactivateAction {
    type Input = NoInput;

    fn name(&self) -> String {
        "deactivate".to_owned()
    }

    fn description(&self) -> ActionDescription<Self::Input> {
        ActionDescription::default().title("Deactivate")
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
            controller.deactivate_scene(id).await;
            action_handle.finish().await.unwrap();
        });

        Ok(())
    }
}
