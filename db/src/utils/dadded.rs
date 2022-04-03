use crate::errors::Error;
use crate::sea_orm::*;
use crate::{Dadded, Epoch};
use tracing::*;

pub async fn get_or_create_dad_from_epoch(
    db: &DbConn,
    epoch_id: u32,
) -> Result<Dadded::Model, Error> {
    if let Some(epoch) = Epoch::Entity::find_by_id(epoch_id).one(db).await? {
        if let Some(dad) = epoch.find_related(Dadded::Entity).one(db).await? {
            Ok(dad)
        } else {
            let dadded_model = Dadded::ActiveModel {
                epoch_id: Set(epoch_id),
                count: Set(0),
                ..Default::default()
            };
            let dad = dadded_model.insert(db).await?;
            info!(
                "Created Dadded {{ id: {}, epoch: {} }}",
                dad.id, dad.epoch_id
            );
            Ok(dad)
        }
    } else {
        Err(Error::EpochNotFound { id: epoch_id })
    }
}

pub async fn increament_dadded(db: &DbConn, dadded_id: u32) -> Result<Dadded::Model, Error> {
    if let Some(dadded) = Dadded::Entity::find_by_id(dadded_id).one(db).await? {
        let current_count = dadded.count;
        let mut active_dadded: Dadded::ActiveModel = dadded.into();
        active_dadded.count = Set(current_count + 1);
        let new_dadded = active_dadded.update(db).await?;
        info!(
            "Updated Dadded {{ id: {} }} count was: {}, now: {}",
            new_dadded.id, current_count, new_dadded.count
        );
        Ok(new_dadded)
    } else {
        Err(Error::DaddedNotFound { id: dadded_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::{DatabaseBackend, MockDatabase};
    use crate::utils::integration_utils;
    use chrono::TimeZone;
    use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};

    #[tokio::test]
    async fn test_get_or_create_dad_does_not_exist() -> Result<(), Error> {
        let d1 = NaiveDate::from_ymd(2022, 3, 16);
        let t1 = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt1 = NaiveDateTime::new(d1, t1);
        let epoch_id = 1;
        let dadded_id = 1;
        let count = 0;
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![vec![Epoch::Model {
                id: epoch_id,
                epoch: Local.from_utc_datetime(&dt1),
            }]])
            .append_query_results(vec![
                vec![],
                vec![Dadded::Model {
                    id: dadded_id,
                    epoch_id: epoch_id,
                    count: 0,
                }],
            ])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: dadded_id as u64,
                rows_affected: 1,
            }])
            .into_connection();
        let res_dad = get_or_create_dad_from_epoch(&db, epoch_id).await?;
        assert_eq!(dadded_id, res_dad.id);
        assert_eq!(epoch_id, res_dad.epoch_id);
        assert_eq!(count, res_dad.count);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_or_create_dad_does_exist() -> Result<(), Error> {
        let d1 = NaiveDate::from_ymd(2022, 3, 16);
        let t1 = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt1 = NaiveDateTime::new(d1, t1);
        let epoch_id = 1;
        let dadded_id = 1;
        let count = 32;
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![vec![Epoch::Model {
                id: epoch_id,
                epoch: Local.from_utc_datetime(&dt1),
            }]])
            .append_query_results(vec![vec![Dadded::Model {
                id: dadded_id,
                epoch_id: epoch_id,
                count: count,
            }]])
            .into_connection();
        let res_dad = get_or_create_dad_from_epoch(&db, epoch_id).await?;
        assert_eq!(dadded_id, res_dad.id);
        assert_eq!(epoch_id, res_dad.epoch_id);
        assert_eq!(count, res_dad.count);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_or_create_dad_epoch_doesnt_exist() -> Result<(), Error> {
        let epoch_id = 1;
        let epoch_vec: Vec<Epoch::Model> = vec![];
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![epoch_vec])
            .into_connection();
        let res_dad = get_or_create_dad_from_epoch(&db, epoch_id).await;
        assert_eq!(Error::EpochNotFound { id: 1 }, res_dad.unwrap_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_increament_dadded_count() -> Result<(), Error> {
        let d1 = NaiveDate::from_ymd(2022, 3, 16);
        let t1 = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt1 = NaiveDateTime::new(d1, t1);
        let epoch_id = 1;
        let dadded_id = 1;
        let count = 0;
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![vec![Epoch::Model {
                id: epoch_id,
                epoch: Local.from_utc_datetime(&dt1),
            }]])
            .append_query_results(vec![
                vec![Dadded::Model {
                    id: dadded_id,
                    epoch_id: epoch_id,
                    count: count,
                }],
                vec![Dadded::Model {
                    id: dadded_id,
                    epoch_id: epoch_id,
                    count: count,
                }],
                vec![Dadded::Model {
                    id: dadded_id,
                    epoch_id: epoch_id,
                    count: count + 1,
                }],
            ])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: dadded_id as u64,
                rows_affected: 1,
            }])
            .into_connection();
        let res_dad = get_or_create_dad_from_epoch(&db, epoch_id).await?;
        let new_dad = increament_dadded(&db, res_dad.id).await?;
        assert_eq!(new_dad.id, res_dad.id);
        assert_eq!(epoch_id, new_dad.epoch_id);
        assert_eq!(new_dad.count, res_dad.count + 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_get_or_create_dad_does_not_exist() -> Result<(), Error> {
        let db = integration_utils::create_inmemory_db().await?;
        let d1 = NaiveDate::from_ymd(2022, 3, 16);
        let t1 = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt1 = NaiveDateTime::new(d1, t1);

        let epoch_am = Epoch::ActiveModel {
            epoch: Set(Local.from_utc_datetime(&dt1)),
            ..Default::default()
        };
        let epoch = epoch_am.insert(&db).await?;

        let epoch_id = epoch.id;
        let dadded_id = 1;
        let count = 0;

        let res_dad = get_or_create_dad_from_epoch(&db, epoch_id).await?;
        assert_eq!(dadded_id, res_dad.id);
        assert_eq!(epoch_id, res_dad.epoch_id);
        assert_eq!(count, res_dad.count);
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_get_or_create_dad_does_exist() -> Result<(), Error> {
        let db = integration_utils::create_inmemory_db().await?;
        let d1 = NaiveDate::from_ymd(2022, 3, 16);
        let t1 = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt1 = NaiveDateTime::new(d1, t1);
        let count = 32;

        let epoch_am = Epoch::ActiveModel {
            epoch: Set(Local.from_utc_datetime(&dt1)),
            ..Default::default()
        };
        let epoch = epoch_am.insert(&db).await?;

        let dad_am = Dadded::ActiveModel {
            epoch_id: Set(epoch.id),
            count: Set(count),
            ..Default::default()
        };
        let dad = dad_am.insert(&db).await?;

        let res_dad = get_or_create_dad_from_epoch(&db, epoch.id).await?;
        assert_eq!(dad.id, res_dad.id);
        assert_eq!(dad.epoch_id, res_dad.epoch_id);
        assert_eq!(dad.count, res_dad.count);
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_get_or_create_dad_epoch_doesnt_exist() -> Result<(), Error> {
        let db = integration_utils::create_inmemory_db().await?;
        let epoch_id = 1;
        let res_dad = get_or_create_dad_from_epoch(&db, epoch_id).await;
        assert_eq!(Error::EpochNotFound { id: 1 }, res_dad.unwrap_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_increament_dadded_count() -> Result<(), Error> {
        let db = integration_utils::create_inmemory_db().await?;
        let d1 = NaiveDate::from_ymd(2022, 3, 16);
        let t1 = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt1 = NaiveDateTime::new(d1, t1);

        let count = 0;

        let epoch_am = Epoch::ActiveModel {
            epoch: Set(Local.from_utc_datetime(&dt1)),
            ..Default::default()
        };
        let epoch = epoch_am.insert(&db).await?;

        let dad_am = Dadded::ActiveModel {
            epoch_id: Set(epoch.id),
            count: Set(count),
            ..Default::default()
        };
        let dad = dad_am.insert(&db).await?;

        let res_dad = get_or_create_dad_from_epoch(&db, epoch.id).await?;
        let new_dad = increament_dadded(&db, res_dad.id).await?;
        assert_eq!(res_dad.id, dad.id);
        assert_eq!(new_dad.id, res_dad.id);
        assert_eq!(new_dad.epoch_id, epoch.id);
        assert_eq!(new_dad.count, res_dad.count + 1);
        Ok(())
    }
}
