//use tracing::*;
use crate::epochs;
use crate::errors::Error;
use crate::sea_orm::*;
use crate::Epoch;
use chrono::{DateTime, Duration, DurationRound, Local};
use tracing::*;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct EpochBounds {
    lower: DateTime<Local>,
    upper: DateTime<Local>,
}

fn get_epoch_boundry(date: &DateTime<Local>, epoch_len: Duration) -> Result<EpochBounds, Error>
{
    let lower_bound = date.duration_trunc(epoch_len)?;
    let upper_bound = lower_bound + epoch_len;
    let bounds = EpochBounds {
        lower: lower_bound,
        upper: upper_bound,
    };
    Ok(bounds)
}

async fn filter_epochs_by_date_time(
    db: &DbConn,
    bounds: EpochBounds,
) -> Result<Vec<Epoch::Model>, Error>
{
    let epochs = Epoch::Entity::find()
        .filter(Condition::all().add(epochs::Column::Epoch.between(bounds.lower, bounds.upper)))
        .all(db)
        .await?;
    Ok(epochs)
}

async fn find_epoch_by_datetime(
    db: &DbConn,
    date: &DateTime<Local>,
    epoch_len: Duration,
) -> Result<Option<Epoch::Model>, Error> {
    let bounds = get_epoch_boundry(date, epoch_len)?;
    let mut epochs = filter_epochs_by_date_time(db, bounds).await?;

    match epochs.len() {
        0 | 1 => Ok(epochs.pop()),
        _ => Err(Error::TooManyEpochs),
    }
}

pub async fn get_or_create_epoch(
    db: &DbConn,
    cur_time: &DateTime<Local>,
    epoch_len: Duration,
) -> Result<Epoch::Model, Error> {
    let epoch = find_epoch_by_datetime(db, cur_time, epoch_len).await?;
    match epoch {
        None => {
            let bounds = get_epoch_boundry(cur_time, epoch_len)?;
            let local_bounds = bounds.lower;
            let epoch_model = Epoch::ActiveModel {
                epoch: Set(local_bounds.to_owned()),
                ..Default::default()
            };
            let epoch = epoch_model.insert(db).await?;
            info!("Created Epoch {{ id {} }}", epoch.id);
            Ok(epoch)
        }
        Some(epoch) => Ok(epoch),
    }
}

pub async fn get_next_epoch_bound(
    db: &DbConn,
    cur_epoch_id: u32,
    epoch_len: Duration,
) -> Result<DateTime<Local>, Error> {
    if let Some(cur_epoch) = Epoch::Entity::find_by_id(cur_epoch_id).one(db).await? {
        let bounds = get_epoch_boundry(&cur_epoch.epoch, epoch_len)?;
        let mut found_epochs = filter_epochs_by_date_time(db, bounds.clone()).await?;
        found_epochs.retain(|e| e.id != cur_epoch_id);
        if found_epochs.is_empty() {
            Ok(bounds.upper)
        } else {
            found_epochs.sort_by(|e1, e2| e1.epoch.cmp(&e2.epoch));
            Ok(found_epochs[0].epoch)
        }
    } else {
        Err(Error::EpochNotFound { id: cur_epoch_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sea_orm::{DatabaseBackend, MockDatabase};
    use crate::utils::integration_utils;
    use chrono::TimeZone;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};

    fn create_multiple_naive_datetimes(
        init_date: NaiveDate,
        init_time: NaiveTime,
        tick: Duration,
        count: i32,
    ) -> Vec<NaiveDateTime> {
        let dt = NaiveDateTime::new(init_date, init_time);
        let mut res: Vec<NaiveDateTime> = Vec::new();
        let mut loop_ndt;
        for cntr in 0..count {
            loop_ndt = dt + (tick * cntr);
            res.push(loop_ndt.clone());
        }
        res
    }

    #[tokio::test]
    async fn test_epoch_util_lower_bound() {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::hours(1);
        let bounds_res = get_epoch_boundry(&date.into(), duration).unwrap();
        let expected_res : DateTime<Local> = Utc.ymd(2022, 3, 16).and_hms_milli(12, 0, 0, 0).into();
        assert_eq!(bounds_res.lower.to_string(), expected_res.to_string());
    }

    #[tokio::test]
    async fn test_epoch_util_upper_bound() {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::hours(1);
        let bounds_res = get_epoch_boundry(&date.into(), duration).unwrap();
        let expected_res : DateTime<Local> = Utc.ymd(2022, 3, 16).and_hms_milli(13, 0, 0, 0).into();
        assert_eq!(bounds_res.upper.to_string(), expected_res.to_string());
    }

    #[tokio::test]
    async fn test_epoch_util_both_bounds() {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::days(1);
        let bounds_res = get_epoch_boundry(&date.into(), duration).unwrap();
        let lower_res : DateTime<Local> = Utc.ymd(2022, 3, 16).and_hms_milli(0, 0, 0, 0).into();
        let upper_res : DateTime<Local> = Utc.ymd(2022, 3, 17).and_hms_milli(0, 0, 0, 0).into();
        assert_eq!(bounds_res.lower.to_string(), lower_res.to_string());
        assert_eq!(bounds_res.upper.to_string(), upper_res.to_string());
    }

    #[tokio::test]
    async fn test_epoch_util_non_unit_epoch() {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::hours(6);
        let bounds_res = get_epoch_boundry(&date.into(), duration).unwrap();
        let lower_res : DateTime<Local> = Utc.ymd(2022, 3, 16).and_hms_milli(12, 0, 0, 0).into();
        let upper_res : DateTime<Local> = Utc.ymd(2022, 3, 16).and_hms_milli(18, 0, 0, 0).into();
        assert_eq!(bounds_res.lower.to_string(), lower_res.to_string());
        assert_eq!(bounds_res.upper.to_string(), upper_res.to_string());
    }

    #[tokio::test]
    async fn test_epoch_util_increment() {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::days(1);
        let init_bounds = get_epoch_boundry(&date.into(), duration).unwrap();
        let bounds_res = get_epoch_boundry(&init_bounds.upper, duration).unwrap();
        let lower_res : DateTime<Local> = Utc.ymd(2022, 3, 17).and_hms_milli(0, 0, 0, 0).into();
        let upper_res : DateTime<Local> = Utc.ymd(2022, 3, 18).and_hms_milli(0, 0, 0, 0).into();
        assert_eq!(bounds_res.lower.to_string(), lower_res.to_string());
        assert_eq!(bounds_res.upper.to_string(), upper_res.to_string());
    }

    #[tokio::test]
    async fn test_find_no_epochs() -> Result<(), Error> {
        let res_vec: Vec<Vec<Epoch::Model>> = vec![vec![]];
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(res_vec)
            .into_connection();
        let duration = Duration::days(1);
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);

        let find_res = find_epoch_by_datetime(&db, &date.into(), duration).await?;

        assert_eq!(find_res, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_find_correct_epoch() -> Result<(), Error> {
        let d = NaiveDate::from_ymd(2022, 3, 16);
        let t = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt = NaiveDateTime::new(d, t);
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![vec![Epoch::Model {
                id: 1,
                epoch: Local.from_local_datetime(&dt).unwrap(),
            }]])
            .into_connection();
        let duration = Duration::days(1);
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);

        let expected = Epoch::Model {
            id: 1,
            epoch: Local.from_local_datetime(&dt).unwrap(),
        };

        let find_res = find_epoch_by_datetime(&db, &date.into(), duration).await?;

        assert_eq!(find_res, Some(expected));

        Ok(())
    }

    #[tokio::test]
    async fn test_find_too_many_epochs() -> Result<(), Error> {
        let d1 = NaiveDate::from_ymd(2022, 3, 16);
        let t1 = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt1 = NaiveDateTime::new(d1, t1);
        let d2 = NaiveDate::from_ymd(2022, 3, 16);
        let t2 = NaiveTime::from_hms_milli(6, 0, 0, 0);
        let dt2 = NaiveDateTime::new(d2, t2);

        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![vec![
                Epoch::Model {
                    id: 1,
                    epoch: Local.from_local_datetime(&dt1).unwrap(),
                },
                Epoch::Model {
                    id: 2,
                    epoch: Local.from_local_datetime(&dt2).unwrap(),
                },
            ]])
            .into_connection();
        let duration = Duration::days(1);
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);

        let res = find_epoch_by_datetime(&db, &date.into(), duration).await;
        assert_eq!(res.unwrap_err(), Error::TooManyEpochs);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_epoch() -> Result<(), Error> {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::days(1);
        let init_bounds = get_epoch_boundry(&date.into(), duration).unwrap();

        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![vec![Epoch::Model {
                id: 1,
                epoch: init_bounds.lower,
            }]])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let expected_epoch = Epoch::Model {
            id: 1,
            epoch: init_bounds.lower,
        };
        let epoch = get_or_create_epoch(&db, &date.into(), duration).await?;
        assert_eq!(epoch, expected_epoch);
        Ok(())
    }

    #[tokio::test]
    async fn test_epoch_already_exists() -> Result<(), Error> {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let later_date = Utc.ymd(2022, 3, 16).and_hms_milli(13, 7, 8, 50);
        let duration = Duration::days(1);
        let init_bounds = get_epoch_boundry(&date.into(), duration).unwrap();

        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![
                vec![Epoch::Model {
                    id: 1,
                    epoch: init_bounds.lower,
                }],
                vec![Epoch::Model {
                    id: 1,
                    epoch: init_bounds.lower,
                }],
            ])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let expected_epoch = Epoch::Model {
            id: 1,
            epoch: init_bounds.lower,
        };
        let epoch = get_or_create_epoch(&db, &date.into(), duration).await?;
        assert_eq!(epoch, expected_epoch);
        let repeat_epoch = get_or_create_epoch(&db, &later_date.into(), duration).await?;
        assert_eq!(repeat_epoch, expected_epoch);

        Ok(())
    }

    #[tokio::test]
    async fn test_find_next_epoch_bound_next_is_none() -> Result<(), Error> {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::days(1);
        let correct_bounds = get_epoch_boundry(&date.into(), duration.clone()).unwrap();

        let d = NaiveDate::from_ymd(2022, 3, 16);
        let t = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt = NaiveDateTime::new(d, t);
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![
                vec![Epoch::Model {
                    id: 1,
                    epoch: Local.from_local_datetime(&dt).unwrap(),
                }],
                vec![Epoch::Model {
                    id: 1,
                    epoch: Local.from_local_datetime(&dt).unwrap(),
                }],
            ])
            .into_connection();

        let next_bound = get_next_epoch_bound(&db, 1, duration).await?;

        assert_eq!(next_bound, correct_bounds.upper);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_next_epoch_bound_next_is_existing_epoch() -> Result<(), Error> {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::days(1);
        let correct_bounds = get_epoch_boundry(&date.into(), duration.clone()).unwrap();

        let d = NaiveDate::from_ymd(2022, 3, 16);
        let t = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let times_vec = create_multiple_naive_datetimes(d, t, duration.clone(), 2);
        let epoch_models = times_vec
            .into_iter()
            .enumerate()
            .map(|(c, t)| Epoch::Model {
                id: (c + 1) as u32,
                epoch: Local.from_utc_datetime(&t).to_owned(),
            })
            .collect::<Vec<_>>();
        let cur_epoch = vec![epoch_models[0].clone()];
        let searched_epochs = epoch_models;
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![cur_epoch, searched_epochs])
            .into_connection();

        let next_bound = get_next_epoch_bound(&db, 1, duration).await?;

        assert_eq!(next_bound, correct_bounds.upper);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_next_epoch_bound_next_multiple_found_get_closest() -> Result<(), Error> {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(1, 1, 2, 100);
        let duration = Duration::hours(12);
        let correct_bounds = get_epoch_boundry(&date.into(), duration.clone()).unwrap();

        let d = NaiveDate::from_ymd(2022, 3, 16);
        let t = NaiveTime::from_hms_milli(0, 0, 0, 0);

        let times_vec = create_multiple_naive_datetimes(d, t, duration.clone(), 2);
        let epoch_models = times_vec
            .into_iter()
            .enumerate()
            .map(|(c, t)| Epoch::Model {
                id: (c + 1) as u32,
                epoch: Local.from_utc_datetime(&t),
            })
            .collect::<Vec<_>>();
        let cur_epoch = vec![epoch_models[0].clone()];
        let searched_epochs = epoch_models;
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results(vec![cur_epoch, searched_epochs])
            .into_connection();
        let supplied_duration = Duration::days(1);
        let next_bound = get_next_epoch_bound(&db, 1, supplied_duration).await?;

        assert_eq!(next_bound, correct_bounds.upper);
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_find_no_epochs() -> Result<(), Error> {
        let db = integration_utils::create_inmemory_db().await?;
        let duration = Duration::days(1);
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);

        let find_res = find_epoch_by_datetime(&db, &date.into(), duration).await?;

        assert_eq!(find_res, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_integration_create_new_epoch() -> Result<(), Error> {
        let db = integration_utils::create_inmemory_db().await?;

        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::days(1);
        let init_bounds = get_epoch_boundry(&date.into(), duration).unwrap();

        let expected_epoch = Epoch::Model {
            id: 1,
            epoch: init_bounds.lower,
        };
        let epoch = get_or_create_epoch(&db, &date.into(), duration).await?;
        assert_eq!(epoch, expected_epoch);
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_test_existing_epoch() -> Result<(), Error> {
        let db = integration_utils::create_inmemory_db().await?;

        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::days(1);
        let init_bounds = get_epoch_boundry(&date.into(), duration).unwrap();
        get_or_create_epoch(&db, &date.into(), duration).await?;

        let expected_epoch = Epoch::Model {
            id: 1,
            epoch: init_bounds.lower,
        };
        let epoch = get_or_create_epoch(&db, &date.into(), duration).await?;
        assert_eq!(epoch, expected_epoch);
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_find_next_epoch_bound_next_is_none() -> Result<(), Error> {
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::days(1);
        let correct_bounds = get_epoch_boundry(&date.into(), duration.clone()).unwrap();

        let d = NaiveDate::from_ymd(2022, 3, 16);
        let t = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let dt = NaiveDateTime::new(d, t);

        let epoch = Epoch::ActiveModel {
            epoch: Set(Local.from_local_datetime(&dt).unwrap().to_owned()),
            ..Default::default()
        };

        let db = integration_utils::create_inmemory_db().await?;
        let epoch = epoch.insert(&db).await?;

        let next_bound = get_next_epoch_bound(&db, epoch.id, duration).await?;

        assert_eq!(next_bound, correct_bounds.upper);
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_find_next_epoch_bound_next_is_existing_epoch() -> Result<(), Error> {
        let db = integration_utils::create_inmemory_db().await?;
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(12, 1, 2, 100);
        let duration = Duration::days(1);
        let correct_bounds = get_epoch_boundry(&date.into(), duration.clone()).unwrap();

        let d = NaiveDate::from_ymd(2022, 3, 16);
        let t = NaiveTime::from_hms_milli(0, 0, 0, 0);
        let times_vec = create_multiple_naive_datetimes(d, t, duration.clone(), 2);
        let epoch_models = times_vec
            .into_iter()
            .map(|t| Epoch::ActiveModel {
                epoch: Set(Local.from_local_datetime(&t).unwrap().to_owned()),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        for am in epoch_models {
            am.insert(&db).await?;
        }
        let next_bound = get_next_epoch_bound(&db, 1, duration).await?;

        assert_eq!(next_bound, correct_bounds.upper);
        Ok(())
    }

    #[tokio::test]
    async fn test_integration_find_next_epoch_bound_next_multiple_found_get_closest(
    ) -> Result<(), Error> {
        let db = integration_utils::create_inmemory_db().await?;
        let date = Utc.ymd(2022, 3, 16).and_hms_milli(1, 1, 2, 100);
        let duration = Duration::hours(12);
        let correct_bounds = get_epoch_boundry(&date.into(), duration.clone()).unwrap();

        let d = NaiveDate::from_ymd(2022, 3, 16);
        let t = NaiveTime::from_hms_milli(0, 0, 0, 0);

        let times_vec = create_multiple_naive_datetimes(d, t, duration.clone(), 2);
        let epoch_models = times_vec
            .into_iter()
            .map(|t| Epoch::ActiveModel {
                epoch: Set(Local.from_utc_datetime(&t).to_owned()),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        for am in epoch_models {
            am.insert(&db).await?;
        }

        let supplied_duration = Duration::days(1);
        let next_bound = get_next_epoch_bound(&db, 1, supplied_duration).await?;

        assert_eq!(next_bound, correct_bounds.upper);
        Ok(())
    }
}
