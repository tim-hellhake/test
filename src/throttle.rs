/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::time::Instant;
use tokio::time::{sleep, Duration};

pub struct Throttle {
    time_to_wait: Duration,
    last_time: Option<Instant>,
}

impl Throttle {
    pub fn new(duration: Duration) -> Self {
        Self {
            time_to_wait: duration,
            last_time: None,
        }
    }

    pub fn reset_start(&mut self) {
        self.last_time = Some(Instant::now());
    }

    pub async fn throttle(&mut self) {
        if let Some(last_time) = self.last_time {
            let elapsed = last_time.elapsed().as_millis();
            let duration = self.time_to_wait.as_millis();

            if elapsed < duration {
                let remaining_millis = duration - elapsed;
                log::debug!("Delaying for {} ms", remaining_millis);
                sleep(Duration::from_millis(remaining_millis as u64)).await;
            }
        } else {
            self.reset_start();
        }
    }
}
