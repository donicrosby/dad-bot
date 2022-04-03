use crate::errors::Error;
use chrono::{DateTime, Duration, Local};
use db::sea_orm::*;
use db::utils as dbUtils;
use db::Dadded;
use getset::{Getters, Setters};
use tracing::*;

#[derive(Debug, Clone, Getters, Setters)]
pub struct DaddedManager {
    #[getset(get = "pub", set)]
    epoch_id: u32,
    #[getset(get = "pub", set)]
    next_epoch: DateTime<Local>,
    #[getset(get = "pub", set)]
    dadded_id: u32,
    #[getset(get = "pub", set)]
    awake_since_last_epoch: bool,
}

impl DaddedManager {
    pub fn new(epoch_id: u32, next_epoch: DateTime<Local>, dadded_id: u32) -> Self {
        Self {
            epoch_id,
            next_epoch,
            dadded_id,
            awake_since_last_epoch: false,
        }
    }

    pub async fn check_for_epoch_update(
        &mut self,
        db: &DbConn,
        now: DateTime<Local>,
        epoch_len: Duration,
    ) -> Result<&Self, Error> {
        if now > self.next_epoch {
            info!("Epoch boundry surpassed, creating new epoch...");
            let new_epoch =
                dbUtils::epochs::get_or_create_epoch(db, &now.into(), epoch_len).await?;
            let next_epoch =
                dbUtils::epochs::get_next_epoch_bound(db, new_epoch.id, epoch_len).await?;
            let new_dadded =
                dbUtils::dadded::get_or_create_dad_from_epoch(db, new_epoch.id).await?;
            let next_bound: DateTime<Local> = next_epoch.into();
            info!("Next Epoch boundry is now {}", next_bound);
            self.set_epoch_id(new_epoch.id);
            self.set_next_epoch(next_bound);
            self.set_dadded_id(new_dadded.id);
            if !self.awake_since_last_epoch() {
                self.set_awake_since_last_epoch(true);
            }
            Ok(self)
        } else {
            Ok(self)
        }
    }

    pub async fn increment_dadded(&mut self, db: &DbConn) -> Result<Dadded::Model, Error> {
        let dad = dbUtils::dadded::increament_dadded(db, self.dadded_id).await?;
        Ok(dad)
    }

    pub async fn get_current_dad(&mut self, db: &DbConn) -> Result<Dadded::Model, Error> {
        let dad = dbUtils::dadded::get_or_create_dad_from_epoch(db, self.epoch_id).await?;
        Ok(dad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration_utils::create_inmemory_db;
    use chrono::{TimeZone, Utc};

    #[tokio::test]
    async fn test_check_for_epoch_update_new_epoch_ticked() -> Result<(), Error> {
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

        assert_eq!(*mgr.epoch_id(), epoch.id + 1);
        assert_eq!(*mgr.dadded_id(), dadded.id + 1);
        assert_eq!(*mgr.next_epoch(), next_epoch + duration);
        assert_eq!(*mgr.awake_since_last_epoch(), true);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_for_epoch_update_same_epoch() -> Result<(), Error> {
        let db = create_inmemory_db().await?;
        let cur_time = Utc.ymd(2022, 4, 1).and_hms_milli(19, 15, 10, 300);
        let duration = Duration::days(1);
        let time_passed = Duration::hours(1);
        let epoch = dbUtils::epochs::get_or_create_epoch(&db, &cur_time.into(), duration).await?;
        let next_epoch = dbUtils::epochs::get_next_epoch_bound(&db, epoch.id, duration).await?;
        let dadded = dbUtils::dadded::get_or_create_dad_from_epoch(&db, epoch.id).await?;
        let mut mgr = DaddedManager::new(epoch.id, next_epoch.into(), dadded.id);

        let cur_time = cur_time + time_passed;

        mgr.check_for_epoch_update(&db, cur_time.into(), duration)
            .await?;

        assert_eq!(*mgr.epoch_id(), epoch.id);
        assert_eq!(*mgr.dadded_id(), dadded.id);
        assert_eq!(*mgr.next_epoch(), next_epoch);
        assert_eq!(*mgr.awake_since_last_epoch(), false);

        Ok(())
    }
}
