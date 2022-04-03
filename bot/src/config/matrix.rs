use chrono::Duration;
use mrsbfh::config::ConfigDerive;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, ConfigDerive)]
pub struct Config<'a> {
    pub homeserver_url: Cow<'a, str>,
    pub mxid: Cow<'a, str>,
    pub avatar: Cow<'a, str>,
    pub password: Cow<'a, str>,
    pub store_path: Cow<'a, str>,
    pub session_path: Cow<'a, str>,
    pub dadded_regex: Cow<'a, str>,
    // Database Connection String
    pub db: Option<Cow<'a, str>>,
    // Epoch Length in minutes
    pub epoch_length: i64,
    // For 1 in Chance
    pub dadded_chance: Option<i64>,
    // Say I love you 1 in Chance during a dadded
    pub love_me_chance: Option<i64>,
}

impl<'a> Config<'a> {
    pub fn get_epoch_length(&self) -> Duration {
        Duration::minutes(self.epoch_length)
    }
}
