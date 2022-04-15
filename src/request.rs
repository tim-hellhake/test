/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::protocol::decoder::{Config, Response, Scene, Value};
use crate::protocol::encoder::{
    ActivateSceneCommand, AssignIdCommand, ClearSceneCommand, ClearScenesCommand, Commands,
    DeactivateSceneCommand, GetConfigCommand, GetScenesCommand, GetValueCommand, SetSceneCommand,
    SetValueCommand,
};
use std::fmt::Debug;
use std::mem;
use std::time::Instant;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub enum Request {
    SetValue {
        command: SetValueCommand,
        tx: oneshot::Sender<RequestResponse<Value>>,
    },
    GetValue {
        command: GetValueCommand,
        tx: oneshot::Sender<RequestResponse<Value>>,
    },
    SetScene {
        command: SetSceneCommand,
        tx: oneshot::Sender<RequestResponse<Scene>>,
    },
    ClearScene {
        command: ClearSceneCommand,
        tx: oneshot::Sender<RequestResponse<Scene>>,
    },
    ClearScenes {
        command: ClearScenesCommand,
        tx: oneshot::Sender<RequestResponse<Vec<Scene>>>,
    },
    GetScenes {
        command: GetScenesCommand,
        tx: oneshot::Sender<RequestResponse<Vec<Scene>>>,
    },
    ActivateScene {
        command: ActivateSceneCommand,
        tx: oneshot::Sender<RequestResponse<()>>,
    },
    DeactivateScene {
        command: DeactivateSceneCommand,
        tx: oneshot::Sender<RequestResponse<()>>,
    },
    GetConfig {
        command: GetConfigCommand,
        tx: oneshot::Sender<RequestResponse<Config>>,
    },
    Hail {
        tx: oneshot::Sender<RequestResponse<Config>>,
    },
    AssignId {
        command: AssignIdCommand,
        tx: oneshot::Sender<RequestResponse<Config>>,
    },
}

impl Request {
    pub fn command(&self) -> Commands {
        match self {
            Request::SetValue { command, tx: _ } => Commands::SetValue(command.to_owned()),
            Request::GetValue { command, tx: _ } => Commands::GetValue(command.to_owned()),
            Request::SetScene { command, tx: _ } => Commands::SetScene(command.to_owned()),
            Request::ClearScene { command, tx: _ } => Commands::ClearScene(command.to_owned()),
            Request::ClearScenes { command, tx: _ } => Commands::ClearScenes(command.to_owned()),
            Request::GetScenes { command, tx: _ } => Commands::GetScenes(command.to_owned()),
            Request::ActivateScene { command, tx: _ } => {
                Commands::ActivateScene(command.to_owned())
            }
            Request::DeactivateScene { command, tx: _ } => {
                Commands::DeactivateScene(command.to_owned())
            }
            Request::GetConfig { command, tx: _ } => Commands::GetConfig(command.to_owned()),
            Request::Hail { .. } => Commands::Hail,
            Request::AssignId { command, tx: _ } => Commands::AssignId(command.to_owned()),
        }
    }
}

#[derive(Debug)]
pub enum RequestResponse<T> {
    Response(T),
    Timeout,
}

pub struct ResponseMatcher {
    request: Option<(Instant, Request, oneshot::Sender<()>)>,
    scenes: Vec<Scene>,
}

impl ResponseMatcher {
    pub fn new() -> Self {
        ResponseMatcher {
            request: None,
            scenes: Vec::new(),
        }
    }

    pub fn wait_for_response_to(&mut self, request: Request) -> oneshot::Receiver<()> {
        log::trace!("Waiting for response to {:?}", request);
        let (tx, rx) = oneshot::channel();
        self.request = Some((Instant::now(), request, tx));

        rx
    }

    pub async fn handle_response(&mut self, response: &Response) {
        let request = mem::replace(&mut self.request, None);

        if let Some((instant, request, tx)) = request {
            self.request = self.handle_request(instant, request, response, tx);
            log::trace!("New request is {:?}", self.request);
        }
    }

    fn handle_request(
        &mut self,
        instant: Instant,
        request: Request,
        response: &Response,
        tx_complete: Sender<()>,
    ) -> Option<(Instant, Request, oneshot::Sender<()>)> {
        log::trace!("Matching {:?} {:?}", request, response);
        let command = request.command();

        match (request, response) {
            (
                Request::SetValue {
                    command: SetValueCommand { id, .. },
                    tx,
                },
                Response::Value(value),
            ) if id == value.id => {
                log_response_time(instant, &command);

                log_send_error(tx.send(RequestResponse::Response(value.to_owned())));
                log_send_error(tx_complete.send(()));
                None
            }
            (
                Request::GetValue {
                    command: GetValueCommand { id },
                    tx,
                },
                Response::Value(value),
            ) if id == value.id => {
                log_response_time(instant, &command);

                log_send_error(tx.send(RequestResponse::Response(value.to_owned())));
                log_send_error(tx_complete.send(()));
                None
            }
            (
                Request::SetScene {
                    command: SetSceneCommand { id, scene, .. },
                    tx,
                },
                Response::Scene(response_scene),
            ) if id == response_scene.id && scene == response_scene.scene => {
                log_response_time(instant, &command);

                log_send_error(tx.send(RequestResponse::Response(response_scene.to_owned())));
                log_send_error(tx_complete.send(()));
                None
            }
            (
                Request::ClearScene {
                    command: ClearSceneCommand { id, scene, .. },
                    tx,
                },
                Response::Scene(response_scene),
            ) if id == response_scene.id && scene == response_scene.scene => {
                log_response_time(instant, &command);

                log_send_error(tx.send(RequestResponse::Response(response_scene.to_owned())));
                log_send_error(tx_complete.send(()));
                None
            }
            (
                Request::ClearScenes {
                    command: ClearScenesCommand { id },
                    tx,
                },
                Response::Scene(scene),
            ) if id == scene.id => match scene.scene {
                1 => {
                    self.scenes.clear();
                    self.scenes.push(scene.to_owned());

                    Some((
                        instant,
                        Request::GetScenes {
                            command: GetScenesCommand { id },
                            tx,
                        },
                        tx_complete,
                    ))
                }
                64 => {
                    log_response_time(instant, &command);

                    let scenes = std::mem::take(&mut self.scenes);
                    log_send_error(tx.send(RequestResponse::Response(scenes)));
                    log_send_error(tx_complete.send(()));
                    None
                }
                _ => {
                    self.scenes.push(scene.to_owned());

                    Some((
                        instant,
                        Request::ClearScenes {
                            command: ClearScenesCommand { id },
                            tx,
                        },
                        tx_complete,
                    ))
                }
            },
            (
                Request::GetScenes {
                    command: GetScenesCommand { id },
                    tx,
                },
                Response::Scene(scene),
            ) if id == scene.id => match scene.scene {
                1 => {
                    self.scenes.clear();
                    self.scenes.push(scene.to_owned());

                    Some((
                        instant,
                        Request::GetScenes {
                            command: GetScenesCommand { id },
                            tx,
                        },
                        tx_complete,
                    ))
                }
                64 => {
                    log_response_time(instant, &command);

                    let scenes = std::mem::take(&mut self.scenes);
                    log_send_error(tx.send(RequestResponse::Response(scenes)));
                    log_send_error(tx_complete.send(()));
                    None
                }
                _ => {
                    self.scenes.push(scene.to_owned());

                    Some((
                        instant,
                        Request::GetScenes {
                            command: GetScenesCommand { id },
                            tx,
                        },
                        tx_complete,
                    ))
                }
            },
            (
                Request::ActivateScene {
                    command: ActivateSceneCommand { .. },
                    tx,
                },
                Response::Value(_),
            ) => {
                log_response_time(instant, &command);

                log_send_error(tx.send(RequestResponse::Response(())));
                log_send_error(tx_complete.send(()));
                None
            }
            (
                Request::DeactivateScene {
                    command: DeactivateSceneCommand { .. },
                    tx,
                },
                Response::Value(_),
            ) => {
                log_response_time(instant, &command);

                log_send_error(tx.send(RequestResponse::Response(())));
                log_send_error(tx_complete.send(()));
                None
            }
            (
                Request::GetConfig {
                    command: GetConfigCommand { id },
                    tx,
                },
                Response::Config(config),
            ) if id == config.id => {
                log_response_time(instant, &command);

                log_send_error(tx.send(RequestResponse::Response(config.to_owned())));
                log_send_error(tx_complete.send(()));
                None
            }
            (Request::Hail { tx }, Response::Config(config)) => {
                log_response_time(instant, &command);

                log_send_error(tx.send(RequestResponse::Response(config.to_owned())));
                log_send_error(tx_complete.send(()));
                None
            }
            (
                Request::AssignId {
                    command: AssignIdCommand { id, .. },
                    tx,
                },
                Response::Config(config),
            ) if id == config.id => {
                log_response_time(instant, &command);

                log_send_error(tx.send(RequestResponse::Response(config.to_owned())));
                log_send_error(tx_complete.send(()));
                None
            }
            (request, _) => Some((instant, request, tx_complete)),
        }
    }

    pub fn timeout(&mut self) {
        let request = mem::replace(&mut self.request, None);
        log::trace!("Timeout {:?}", request);

        if let Some((_, request, _)) = request {
            match request {
                Request::SetValue { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::GetValue { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::SetScene { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::ClearScene { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::ClearScenes { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::GetScenes { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::ActivateScene { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::DeactivateScene { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::GetConfig { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::Hail { tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
                Request::AssignId { command: _, tx } => {
                    log_send_error(tx.send(RequestResponse::Timeout));
                }
            };
        }
    }
}

fn log_response_time(instant: Instant, command: &Commands) {
    log::debug!(
        "Received response for {:?} after {:?} ms",
        command,
        instant.elapsed().as_secs_f64() * 1000_f64
    );
}

fn log_send_error<T: Debug>(result: Result<(), T>) {
    if let Err(value) = result {
        log::warn!("Receiver was dropped before {:?} could be send", value);
    }
}
