use crate::commands::utils::DaddedManager;
use crate::config::Config;
use crate::errors::Error;
use matrix_sdk::ruma::events::{
    room::message::{MessageEventContent, MessageType, TextMessageEventContent},
    AnyMessageEventContent,
};
use mrsbfh::commands::command;
use mrsbfh::commands::extract::Extension;
// use regex::Regex;
use chrono::Duration;
use db::sea_orm::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::*;
use getset::{Getters, Setters};

#[derive(Debug, Clone, Getters, Setters)]
struct DadDurationText {
    #[getset(get = "pub", set)]
    days: i64,
    #[getset(get = "pub", set)]
    hours: i64,
    #[getset(get = "pub", set)]
    minutes: i64

}

impl DadDurationText {
    pub fn new(dur: Duration) -> Self {
        let days = dur.num_days();
        let dur = dur - Duration::days(dur.num_days());
        let hours = dur.num_hours();
        let dur = dur - Duration::hours(dur.num_hours());
        let minutes = dur.num_minutes();
        Self {
            days,
            hours,
            minutes
        }
    }

    pub fn get_text(self) -> String {
        self.get_time_string_from_duration()
    }

    #[inline]
    pub fn duration_time_to_string(num: i64, period_name: &str) -> (i64, String) {
        let period_str = match num {
            0 => String::new(),
            1 => period_name.to_string(),
            _ => format!("{}s", period_name),
        };
        (num, period_str)
    }


    pub fn get_time_string_from_duration(&self) -> String {
        let days = DadDurationText::duration_time_to_string(*self.days(), "day");
        let hours = DadDurationText::duration_time_to_string(*self.hours(), "hour");
        let minutes = DadDurationText::duration_time_to_string(*self.minutes(), "minute");
        let strings = vec![days, hours, minutes];
        let strings = strings.iter().filter(|sm| sm.0 > 0).collect::<Vec<_>>();
        match strings.len() {
            0 => String::from("instant"),
            1 => strings[0].1.clone(),
            _ => strings
                .iter()
                .filter(|v| v.0 > 0)
                .map(|v| format!("{} {}", v.0, v.1))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

#[command(help = "`!dadded` - How many times I've dadded today")]
pub async fn dadded<'a>(
    Extension(tx): Extension<Arc<Mutex<mrsbfh::Sender>>>,
    Extension(config): Extension<Arc<Mutex<Config<'a>>>>,
    Extension(db): Extension<Arc<Mutex<DbConn>>>,
    Extension(dad_handler): Extension<Arc<Mutex<DaddedManager>>>,
) -> Result<(), Error>
where
    Config<'a>: mrsbfh::config::Loader + Clone,
{
    let db = &*db.lock().await;
    let dad_handler = &mut *dad_handler.lock().await;
    let config = &*config.lock().await;
    let current_dads_resp = get_dads(db, dad_handler, config.get_epoch_length()).await?;
    let content = AnyMessageEventContent::RoomMessage(MessageEventContent::new(MessageType::Text(
        TextMessageEventContent::markdown(current_dads_resp),
    )));

    tx.lock().await.send(content).await?;
    Ok(())
}

async fn get_dads<'a>(
    db: &'a DbConn,
    dad_mgr: &'a mut DaddedManager,
    epoch_len: Duration,
) -> Result<String, Error> {
    let dad = dad_mgr.get_current_dad(db).await?;
    let times = match dad.count {
        1 => "time",
        _ => "times",
    };

    let resp = if *dad_mgr.awake_since_last_epoch() {
        format!(
            "I've dadded {} {} in the past {}",
            dad.count,
            times,
            DadDurationText::new(epoch_len).get_text()
        )
    } else {
        format!("I've dadded {} {} since my last nap", dad.count, times)
    };
    info!("Responding to dad request: {}", resp);
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration_utils::create_inmemory_db;
    use chrono::{TimeZone, Utc};
    use db::utils as dbUtils;

    #[tokio::test]
    async fn test_duration_time_to_string() -> Result<(), Error> {
        let none = DadDurationText::duration_time_to_string(0, "day");
        let single = DadDurationText::duration_time_to_string(1, "day");
        let multiple = DadDurationText::duration_time_to_string(2, "day");

        assert_eq!(none, (0, String::new()));
        assert_eq!(single, (1, String::from("day")));
        assert_eq!(multiple, (2, String::from("days")));
        Ok(())
    }

    #[tokio::test]
    async fn test_get_time_string_from_duration_all_components_exist() -> Result<(), Error> {
        let duration = Duration::days(1) + Duration::hours(1) + Duration::minutes(1);

        let time_str = DadDurationText::new(duration).get_text();
        assert_eq!(time_str, String::from("1 day 1 hour 1 minute"));
        Ok(())
    }

    #[tokio::test]
    async fn test_get_time_string_from_duration_some_components_exist() -> Result<(), Error> {
        let duration = Duration::days(2) + Duration::minutes(1);

        let time_str = DadDurationText::new(duration).get_text();
        assert_eq!(time_str, String::from("2 days 1 minute"));
        Ok(())
    }

    #[tokio::test]
    async fn test_get_time_string_from_duration_lone_item_is_unit() -> Result<(), Error> {
        let duration = Duration::minutes(1);

        let time_str = DadDurationText::new(duration).get_text();
        assert_eq!(time_str, String::from("minute"));
        Ok(())
    }

    #[tokio::test]
    async fn test_get_time_string_from_duration_less_than_a_minute_is_instant() -> Result<(), Error>
    {
        let duration = Duration::seconds(1);

        let time_str = DadDurationText::new(duration).get_text();
        assert_eq!(time_str, String::from("instant"));
        Ok(())
    }

    #[tokio::test]
    async fn test_get_dads() -> Result<(), Error> {
        let db = create_inmemory_db().await?;
        let cur_time = Utc.ymd(2022, 4, 1).and_hms_milli(19, 15, 10, 300);
        let duration = Duration::days(1);
        let epoch = dbUtils::epochs::get_or_create_epoch(&db, &cur_time.into(), duration).await?;
        let next_epoch = dbUtils::epochs::get_next_epoch_bound(&db, epoch.id, duration).await?;
        let dadded = dbUtils::dadded::get_or_create_dad_from_epoch(&db, epoch.id).await?;
        let mut mgr = DaddedManager::new(epoch.id, next_epoch.into(), dadded.id);
        let cur_time = cur_time + duration;

        mgr.check_for_epoch_update(&db, cur_time.into(), duration)
            .await?;

        let dad_string_none = get_dads(&db, &mut mgr, duration).await?;

        mgr.increment_dadded(&db).await?;
        let dad_string_one = get_dads(&db, &mut mgr, duration).await?;

        assert_eq!(
            dad_string_none,
            String::from("I've dadded 0 times in the past day")
        );
        assert_eq!(
            dad_string_one,
            String::from("I've dadded 1 time in the past day")
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_get_dads_not_awake_since_last_epoch() -> Result<(), Error> {
        let db = create_inmemory_db().await?;
        let cur_time = Utc.ymd(2022, 4, 1).and_hms_milli(19, 15, 10, 300);
        let duration = Duration::days(1);
        let epoch = dbUtils::epochs::get_or_create_epoch(&db, &cur_time.into(), duration).await?;
        let next_epoch = dbUtils::epochs::get_next_epoch_bound(&db, epoch.id, duration).await?;
        let dadded = dbUtils::dadded::get_or_create_dad_from_epoch(&db, epoch.id).await?;
        let mut mgr = DaddedManager::new(epoch.id, next_epoch.into(), dadded.id);

        mgr.increment_dadded(&db).await?;
        let dad_string_one = get_dads(&db, &mut mgr, duration).await?;

        assert_eq!(
            dad_string_one,
            String::from("I've dadded 1 time since my last nap")
        );

        Ok(())
    }
}
