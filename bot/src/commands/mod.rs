// use crate::config::Config;
// use crate::errors::Error;
use mrsbfh::commands::command_generate;

pub mod dadded;
pub mod utils;

#[command_generate(bot_name = "Dad", description = "I'm your digital dad!")]
enum Commands {
    Dadded,
}
