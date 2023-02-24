use actix_web::web::Data;
use std::{
    future::Future,
    io, mem,
    pin::Pin,
    task::{Context, Poll},
    thread,
    time::Duration,
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use futures_core::{future::BoxFuture, Stream};
use crate::configuration::{DatabaseSettings, Settings};

pub struct Bot {
    mempool_space: String,
    fut: BoxFuture<'static, io::Result<()>>,
}


pub struct Application {
    mempool_space_url: String,
    bot: Bot,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
       Ok( Self { mempool_space_url: "".to_owned() }) 
    } 
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.bot.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

//Want to create a single bot that runs mempoolspace and nostr_client in differen threads
//the process may need to run in worker pools?
pub async fn run(
    db_pool: PgPool,
) -> Result<Bot, anyhow::Error>{
    let db_pool = Data::new(db_pool);
    let bot = Bot { mempool_space_url: ("").to_owned() };
    
    Ok(bot)
}