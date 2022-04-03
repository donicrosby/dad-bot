use crate::commands::utils::{DaddedManager, RngManager};
use crate::config::Config;
use chrono::Local;
use db::sea_orm::DbConn;
use db::utils::{dadded, epochs};
use matrix_sdk::{ruma::MxcUri, Client, ClientConfig, Session as SDKSession, SyncSettings};
use mrsbfh::{url::Url, utils::Session};
use std::{convert::TryFrom, error::Error, fs, path::Path, sync::Arc};
use tokio::sync::Mutex;
use tracing::*;
use rand_chacha::ChaChaRng;
use rand::SeedableRng;


mod sync;

pub async fn setup(config: Config<'_>) -> Result<Client, Box<dyn Error>> {
    info!("Beginning Matrix Setup");
    let store_path_string = config.store_path.to_string();
    let store_path = Path::new(&store_path_string);
    if !store_path.exists() {
        fs::create_dir_all(store_path)?;
    }
    let client_config = ClientConfig::new().store_path(fs::canonicalize(&store_path)?);

    let homeserver_url =
        Url::parse(&config.homeserver_url).expect("Couldn't parse the homeserver URL");

    let client = Client::new_with_config(homeserver_url, client_config).unwrap();

    if let Some(session) = Session::load(config.session_path.parse().unwrap()) {
        info!("Starting relogin");

        let session = SDKSession {
            access_token: session.access_token,
            device_id: session.device_id.into(),
            user_id: matrix_sdk::ruma::UserId::try_from(session.user_id.as_str()).unwrap(),
        };

        if let Err(e) = client.restore_login(session).await {
            error!("{}", e);
        };
        info!("Finished relogin");
    } else {
        info!("Starting login");
        let login_response = client
            .login(&config.mxid, &config.password, None, Some("dad-bot"))
            .await;
        match login_response {
            Ok(login_response) => {
                info!("Session: {:#?}", login_response);
                let session = Session {
                    homeserver: client.homeserver().await.to_string(),
                    user_id: login_response.user_id.to_string(),
                    access_token: login_response.access_token,
                    device_id: login_response.device_id.into(),
                };
                session.save(config.session_path.parse().unwrap())?;
            }
            Err(e) => error!("Error while login: {}", e),
        }
        info!("Finished login");
    }

    info!("logged in as {}", config.mxid);
    info!("Updating bot avatar if needed...");
    let avatar_uri = config.avatar.to_string();
    let cur_avatar_uri = client.avatar_url().await?;
    if !avatar_uri.is_empty() {
        debug!("Config Avatar is not empty...");
        let avatar_uri = MxcUri::from(avatar_uri);
        if avatar_uri.is_valid() {
            debug!("Config Avatar is valid...");
            match cur_avatar_uri {
                Some(cur) => {
                    if avatar_uri != cur {
                        info!("Updating Avatar!");
                        client.set_avatar_url(Some(&avatar_uri)).await?;
                    } else {
                        info!("Avatar is the same as in the config not updating...");
                    }
                }
                None => {
                    info!("Updating Avatar!");
                    client.set_avatar_url(Some(&avatar_uri)).await?;
                }
            }
        }
    }

    Ok(client)
}

pub async fn start_sync(
    client: &mut Client,
    config: Config<'static>,
    db: DbConn,
) -> Result<(), Box<dyn Error>> {
    client.register_event_handler(mrsbfh::sync::autojoin).await;
    let config = Arc::new(Mutex::new(config));
    let cloned_config = Arc::clone(&config);
    let db = Arc::new(Mutex::new(db));
    let cloned_db = Arc::clone(&db);

    let now = Local::now();
    let config_options = cloned_config.lock().await.clone();

    let epoch_length = config_options.get_epoch_length();
    info!("Initalizing Dadded Epoch Manager...");
    let epoch = epochs::get_or_create_epoch(&*db.lock().await, &now.into(), epoch_length).await?;
    let next_epoch =
        epochs::get_next_epoch_bound(&*db.lock().await, epoch.id, epoch_length).await?;
    let dadded = dadded::get_or_create_dad_from_epoch(&*db.lock().await, epoch.id).await?;

    let dad_manager = Arc::new(Mutex::new(DaddedManager::new(
        epoch.id,
        next_epoch.into(),
        dadded.id,
    )));
    info!("Intializing Dadded RNG Manager...");

    let dadded_chance = config_options.dadded_chance;
    let love_me_chance = config_options.love_me_chance;
    let rng = ChaChaRng::from_entropy();
    let rng_manager = Arc::new(Mutex::new(RngManager::new(
        dadded_chance,
        love_me_chance,
        rng
    )));
    client
        .register_event_handler(move |ev, room, client| {
            let handler_config = Arc::clone(&cloned_config);
            let handler_db = Arc::clone(&cloned_db);
            let handler_dad_manager = Arc::clone(&dad_manager);
            let handler_rng_manager = Arc::clone(&rng_manager);
            sync::on_room_message(
                ev,
                room,
                client,
                handler_config,
                handler_db,
                handler_dad_manager,
                handler_rng_manager
            )
        })
        .await;

    info!("Starting full Sync...");
    client.sync(SyncSettings::default()).await;

    Ok(())
}
