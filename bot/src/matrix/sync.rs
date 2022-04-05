use crate::commands::match_command;
use crate::commands::utils::{DaddedManager, RngManager};
use crate::config::Config;
use crate::errors::Error;
use chrono::Local;
use db::sea_orm::DbConn;
use matrix_sdk::{
    room::Room,
    ruma::events::{
        room::message::{MessageEventContent, MessageType, TextMessageEventContent},
        AnyMessageEventContent, SyncMessageEvent,
    },
    Client,
};
use rand::RngCore;
use regex::{Regex, RegexBuilder};
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};
use tracing::*;

static DADDED_RE: OnceCell<Regex> = OnceCell::const_new();

async fn get_dadded_regex(config: &Config<'static>) -> &'static Regex {
    DADDED_RE
        .get_or_init(move || async {
            RegexBuilder::new(&config.dadded_regex.clone())
                .case_insensitive(true)
                .build()
                .unwrap()
        })
        .await
}

fn get_message_from_event(event: SyncMessageEvent<MessageEventContent>, room: Room) -> String {
    if let matrix_sdk::room::Room::Joined(_room) = room {
        if let matrix_sdk::ruma::events::SyncMessageEvent {
            content:
                matrix_sdk::ruma::events::room::message::MessageEventContent {
                    msgtype:
                        matrix_sdk::ruma::events::room::message::MessageType::Text(
                            matrix_sdk::ruma::events::room::message::TextMessageEventContent {
                                body: msg_body,
                                ..
                            },
                        ),
                    ..
                },
            ..
        } = event
        {
            msg_body
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}

fn create_dadded_text(dadded_regex: &Regex, msg: &str, should_love: bool) -> Option<String> {
    if let Some(dad_caps) = dadded_regex.captures(msg) {
        if let Some(im_named) = dad_caps.name("im") {
            let named_im_string = im_named.as_str().to_string();
            debug!("Found 'im' named group: {}", &named_im_string);
            named_im_string
        } else {
            let group_im_string = dad_caps.get(1).unwrap().as_str().to_string();
            debug!("Found group 1 ('im'): {}", &group_im_string);
            group_im_string
        };
        let to_be_dadded = if let Some(dadded_named) = dad_caps.name("dad_text") {
            let named_dad_text = dadded_named.as_str().to_string();
            debug!("Found 'dad_text' named group: {}", &named_dad_text);
            named_dad_text
        } else {
            let group_dad_text = dad_caps.get(2).unwrap().as_str().to_string();
            debug!("Found group 1 ('dad_text'): {}", &group_dad_text);
            group_dad_text
        };
        let im_dad = if should_love {
            String::from("I'm Dad and I love you")
        } else {
            String::from("I'm Dad")
        };
        let dadded_string = format!("Hi {}! {}!", to_be_dadded, im_dad);
        Some(dadded_string)
    } else {
        None
    }
}

async fn handle_dadded_text<T>(
    config: Arc<Mutex<Config<'static>>>,
    event: SyncMessageEvent<MessageEventContent>,
    room: Room,
    rng: Arc<Mutex<RngManager<T>>>,
) -> Option<String>
where
    T: RngCore + Send,
{
    let config = &*config.lock().await;
    let rng = &mut *rng.lock().await;
    let dadded_regex = get_dadded_regex(config).await;
    let msg = get_message_from_event(event, room);
    if rng.should_dad() {
        create_dadded_text(dadded_regex, &msg, rng.should_love_you())
    } else {
        None
    }
}

async fn dadded_manager_update_epoch(
    dad_handler: Arc<Mutex<DaddedManager>>,
    config: Arc<Mutex<Config<'static>>>,
    db: Arc<Mutex<DbConn>>,
) -> Result<(), Error> {
    let mgr = &mut *dad_handler.lock().await;
    let config = &*config.lock().await;
    let db = &*db.lock().await;
    let epoch_duration = config.get_epoch_length();
    mgr.check_for_epoch_update(db, Local::now(), epoch_duration)
        .await?;
    Ok(())
}

async fn dadded_manager_increment(
    dad_handler: Arc<Mutex<DaddedManager>>,
    db: Arc<Mutex<DbConn>>,
) -> Result<(), Error> {
    let mgr = &mut *dad_handler.lock().await;
    let db = &*db.lock().await;
    mgr.increment_dadded(db).await?;
    Ok(())
}

#[mrsbfh::commands::commands]
pub(crate) async fn on_room_message<T>(
    event: SyncMessageEvent<MessageEventContent>,
    room: Room,
    client: Client,
    config: Arc<Mutex<Config<'static>>>,
    db: Arc<Mutex<DbConn>>,
    dad_handler: Arc<Mutex<DaddedManager>>,
    rng_handler: Arc<Mutex<RngManager<T>>>,
) where
    T: RngCore + Send + 'static,
{
    let cloned_config = Arc::clone(&config);
    info!("Ticking manager epoch...");
    if let Err(e) = dadded_manager_update_epoch(
        Arc::clone(&dad_handler),
        Arc::clone(&config),
        Arc::clone(&db),
    )
    .await
    {
        error!("Error ticking epoch: {}", e);
        return;
    }
    if *room.own_user_id() != event.sender {
        if let Some(text) = handle_dadded_text(
            cloned_config,
            event.clone(),
            room.clone(),
            Arc::clone(&rng_handler),
        )
        .await
        {
            if let matrix_sdk::room::Room::Joined(room) = room.clone() {
                info!("Sending Dadded: {}", &text);
                let content = AnyMessageEventContent::RoomMessage(MessageEventContent::new(
                    MessageType::Text(TextMessageEventContent::markdown(text)),
                ));
                if let Err(e) = room.send(content, None).await {
                    error!("{}", e);
                } else {
                    // Update DB
                    info!("Incrementing Dadded Count...");
                    if let Err(e) =
                        dadded_manager_increment(Arc::clone(&dad_handler), Arc::clone(&db)).await
                    {
                        error!("{}", e);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_DADDED_RE: OnceCell<Regex> = OnceCell::const_new();
    static TEST_DADDED_RE_NAMED: OnceCell<Regex> = OnceCell::const_new();

    async fn get_test_regex() -> &'static Regex {
        TEST_DADDED_RE
            .get_or_init(move || async {
                RegexBuilder::new(r"\b((?:i|l)(?:(?:'|`|‛|‘|’|′|‵)?m| am))(?:\s+)([^\.!?]+)")
                    .case_insensitive(true)
                    .build()
                    .unwrap()
            })
            .await
    }

    async fn get_test_named_regex() -> &'static Regex {
        TEST_DADDED_RE_NAMED
            .get_or_init(move || async {
                RegexBuilder::new(
                    r"\b(?P<im>(?:i|l)(?:(?:'|`|‛|‘|’|′|‵)?m| am))(?:\s+)(?P<dad_text>[^\.!?]+)",
                )
                .case_insensitive(true)
                .build()
                .unwrap()
            })
            .await
    }

    #[tokio::test]
    async fn test_generate_dad_regex() -> Result<(), Error> {
        let re = get_test_regex().await;
        let named_re = get_test_named_regex().await;
        let chat_msg = String::from("I'm hungry.");
        let resp = create_dadded_text(re, &chat_msg, false).unwrap();
        let named_resp = create_dadded_text(named_re, &chat_msg, false).unwrap();
        let expected_resp = String::from("Hi hungry! I'm Dad!");
        assert_eq!(resp, expected_resp);
        assert_eq!(named_resp, expected_resp);
        Ok(())
    }

    #[tokio::test]
    async fn test_generate_dad_with_love() -> Result<(), Error> {
        let re = get_test_regex().await;
        let chat_msg = String::from("I'm hungry.");
        let resp = create_dadded_text(re, &chat_msg, true).unwrap();
        let expected_resp = String::from("Hi hungry! I'm Dad and I love you!");
        assert_eq!(resp, expected_resp);
        Ok(())
    }

    #[tokio::test]
    async fn test_regex_from_config() -> Result<(), Error> {
        let regex_str = String::from(
            r"\b(?P<im>(?:i|l)(?:(?:'|`|‛|‘|’|′|‵)?m| am))(?:\s+)(?P<dad_text>[^\.!?]+)",
        );
        let chat_msg = String::from("I'm hungry.");
        let expected_resp = String::from("Hi hungry! I'm Dad!");
        let re = RegexBuilder::new(&regex_str)
            .case_insensitive(true)
            .build()
            .unwrap();

        let resp = create_dadded_text(&re, &chat_msg, false).unwrap();

        assert_eq!(resp, expected_resp);

        Ok(())
    }
}
