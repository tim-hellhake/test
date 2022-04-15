/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::controller::Controller;
use crate::request::RequestResponse;
use crate::Config;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Discovery {
    config: Config,
    controller: Controller,
    known_ids: Arc<Mutex<HashSet<u8>>>,
    in_progress: Arc<Mutex<Box<bool>>>,
}

fn next_id(known_ids: &mut HashSet<u8>) -> Option<u8> {
    for id in 5..=240 {
        if !known_ids.contains(&id) {
            known_ids.insert(id);
            return Some(id);
        }
    }

    None
}

impl Discovery {
    pub fn new(config: Config, controller: Controller) -> Self {
        Discovery {
            config,
            controller,
            known_ids: Arc::new(Mutex::new(HashSet::new())),
            in_progress: Arc::new(Mutex::new(Box::new(false))),
        }
    }

    pub async fn start(&mut self) {
        let mut in_progress = self.in_progress.lock().await;

        if !**in_progress {
            log::debug!("Starting discovery");

            **in_progress = true;
            self.known_ids.lock().await.clear();

            let mut controller = self.controller.clone();

            let in_progress = self.in_progress.clone();

            let max_id = self.config.expert_settings.max_id;

            tokio::spawn(async move {
                let mut known_ids = HashSet::new();

                for id in 5..=max_id {
                    let receiver = controller.request_config(id).await;

                    match receiver.await {
                        Ok(RequestResponse::Response(config)) => {
                            log::debug!("Received config for {}", id);
                            known_ids.insert(config.id);
                        }
                        Ok(RequestResponse::Timeout) => {
                            log::debug!("Received timeout for get config of id {}", id)
                        }
                        Err(err) => {
                            log::error!("Failed to request config: {}", err)
                        }
                    }
                }

                log::debug!("Discovered existing ids: {:?}", known_ids);

                loop {
                    let receiver = controller.hail().await;

                    match receiver.await {
                        Ok(RequestResponse::Response(hail_config)) => {
                            log::debug!(
                                "Received hail config for {}",
                                hail_config.hardware_serial_number
                            );

                            match next_id(&mut known_ids) {
                                Some(id) => {
                                    let receiver = controller
                                        .assign_id(id, hail_config.hardware_serial_number.clone())
                                        .await;

                                    match receiver.await {
                                        Ok(RequestResponse::Response(_)) => {
                                            log::debug!("Received assigned id config for {}", id);
                                        }
                                        Ok(RequestResponse::Timeout) => {
                                            log::debug!(
                                                "Received timeout for assigning id {} to {}",
                                                id,
                                                hail_config.hardware_serial_number
                                            )
                                        }
                                        Err(err) => {
                                            log::error!("Failed to request config: {}", err)
                                        }
                                    }
                                }
                                None => {
                                    log::warn!("No more free ids available for assigning");
                                    break;
                                }
                            };
                        }
                        Ok(RequestResponse::Timeout) => {
                            log::debug!("Received timeout for hail");
                            break;
                        }
                        Err(err) => {
                            log::error!("Failed to hail: {}", err);
                        }
                    }
                }

                **(in_progress.lock().await) = false;
            });
        } else {
            log::debug!("Discovery already in progress");
        }
    }
}
