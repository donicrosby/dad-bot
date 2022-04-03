extern crate clap;
extern crate lazy_static;

use crate::config::Config;
use clap::Parser;
use db::migration::*;
use db::sea_orm::*;
use mrsbfh::config::Loader;
use std::error::Error;
use tracing::*;

mod commands;
mod config;
mod errors;
#[cfg(test)]
mod integration_utils;
mod matrix;

#[derive(Parser, Debug)]
#[clap(
    name = "Dad Bot",
    author = "Jeansburger <@doni:jeansburger.net>",
    about = "Hi Matrix, I'm dad!"
)]
struct Args {
    #[clap(
        short,
        long,
        env = "CONFIG_PATH",
        default_value = "config.yml",
        value_name = "FILE"
    )]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .pretty()
        .with_thread_names(true)
        .init();

    info!("Booting up....");
    debug!("Creating arguments...");
    let args = Args::parse();
    info!("Loading configs...");
    let config = Config::load(args.config)?;
    info!("Setting up Client...");
    let client = &mut matrix::setup(config.clone()).await?;
    info!("Createing DB connection...");
    let db_conn_str = if let Some(conn_str) = config.db.clone() {
        conn_str.to_string()
    } else {
        String::from("sqlite://./dad.db?mode=rwc")
    };
    let db: DbConn = Database::connect(db_conn_str).await?;
    info!("Running DB Migrations...");
    Migrator::up(&db, None).await?;

    info!("Starting Sync...");
    matrix::start_sync(client, config, db).await?;
    Ok(())
}
