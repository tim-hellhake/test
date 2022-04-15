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
use crate::request::{Request, RequestResponse, ResponseMatcher};
use crate::throttle::Throttle;
use crate::transport::Transport;
use futures::FutureExt;
use std::sync::Arc;
use tokio::sync::oneshot::Receiver;
use tokio::{
    select,
    sync::{mpsc, oneshot, Mutex},
    time::{sleep, Duration},
};

fn command_timeout(command: &Commands, default_ms: u64) -> Duration {
    match command {
        Commands::Hail => Duration::from_millis(5000),
        Commands::ActivateScene(_) => Duration::from_millis(3000),
        Commands::DeactivateScene(_) => Duration::from_millis(3000),
        _ => Duration::from_millis(default_ms),
    }
}

#[derive(Clone)]
pub struct Controller {
    tx: mpsc::Sender<Request>,
    response_matcher: Arc<Mutex<ResponseMatcher>>,
}

impl Controller {
    pub fn start(config: crate::Config, transport: Arc<Mutex<dyn Transport>>) -> Self {
        let (tx, mut rx) = mpsc::channel::<Request>(100);
        let request = Arc::new(Mutex::new(ResponseMatcher::new()));
        let response_matcher = request.clone();
        let tx_delay_ms = config.expert_settings.tx_delay_ms;
        let response_timeout_ms = config.expert_settings.response_timeout_ms;

        tokio::spawn(async move {
            let mut throttle = Throttle::new(Duration::from_millis(tx_delay_ms));

            while let Some(request) = rx.recv().await {
                throttle.throttle().await;

                let command = request.command();

                match transport.lock().await.send(command.clone()).await {
                    Ok(_) => {
                        log::debug!("Sent command {:?}", command)
                    }
                    Err(err) => {
                        log::debug!("Failed to send command {:?}: {}", command, err)
                    }
                }

                let response_future = response_matcher
                    .lock()
                    .await
                    .wait_for_response_to(request)
                    .fuse();

                let timeout_future = sleep(command_timeout(&command, response_timeout_ms)).fuse();

                select! {
                    _response = response_future => {
                        throttle.reset_start();
                    },
                    () = timeout_future => {response_matcher.lock().await.timeout();}
                };
            }
        });

        Controller {
            tx,
            response_matcher: request,
        }
    }

    pub async fn check_response(&self, response: &Response) {
        self.response_matcher
            .lock()
            .await
            .handle_response(response)
            .await;
    }

    pub async fn set_value(&mut self, id: u8, value: u8) -> Receiver<RequestResponse<Value>> {
        let command = SetValueCommand { id, value };
        let (tx, rx) = oneshot::channel();
        let request = Request::SetValue { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn request_current_value(&mut self, id: u8) -> Receiver<RequestResponse<Value>> {
        let command = GetValueCommand { id };
        let (tx, rx) = oneshot::channel();
        let request = Request::GetValue { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn set_scene(
        &mut self,
        id: u8,
        scene: u8,
        ramp_duration: u8,
        level: u8,
    ) -> Receiver<RequestResponse<Scene>> {
        let command = SetSceneCommand {
            id,
            scene,
            ramp_duration,
            level,
        };
        let (tx, rx) = oneshot::channel();
        let request = Request::SetScene { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn clear_scene(&mut self, id: u8, scene: u8) -> Receiver<RequestResponse<Scene>> {
        let command = ClearSceneCommand { id, scene };
        let (tx, rx) = oneshot::channel();
        let request = Request::ClearScene { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn clear_scenes(&mut self, id: u8) -> Receiver<RequestResponse<Vec<Scene>>> {
        let command = ClearScenesCommand { id };
        let (tx, rx) = oneshot::channel();
        let request = Request::ClearScenes { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn request_scenes(&mut self, id: u8) -> Receiver<RequestResponse<Vec<Scene>>> {
        let command = GetScenesCommand { id };
        let (tx, rx) = oneshot::channel();
        let request = Request::GetScenes { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn activate_scene(&mut self, id: u8) -> Receiver<RequestResponse<()>> {
        let command = ActivateSceneCommand { id };
        let (tx, rx) = oneshot::channel();
        let request = Request::ActivateScene { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn deactivate_scene(&mut self, id: u8) -> Receiver<RequestResponse<()>> {
        let command = DeactivateSceneCommand { id };
        let (tx, rx) = oneshot::channel();
        let request = Request::DeactivateScene { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn request_config(&mut self, id: u8) -> Receiver<RequestResponse<Config>> {
        let command = GetConfigCommand { id };
        let (tx, rx) = oneshot::channel();
        let request = Request::GetConfig { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn hail(&mut self) -> Receiver<RequestResponse<Config>> {
        let (tx, rx) = oneshot::channel();
        let request = Request::Hail { tx };

        self.enqueue_and_wait(request, rx).await
    }

    pub async fn assign_id(
        &mut self,
        id: u8,
        serial_number: String,
    ) -> Receiver<RequestResponse<Config>> {
        let command = AssignIdCommand { id, serial_number };
        let (tx, rx) = oneshot::channel();
        let request = Request::AssignId { command, tx };

        self.enqueue_and_wait(request, rx).await
    }

    async fn enqueue_and_wait<T>(
        &mut self,
        request: Request,
        rx: Receiver<RequestResponse<T>>,
    ) -> Receiver<RequestResponse<T>> {
        log::debug!("Enqueuing request {:?}", request.command());

        self.tx
            .send(request)
            .await
            .expect("Receiver was dropped before request was sent");

        rx
    }
}
