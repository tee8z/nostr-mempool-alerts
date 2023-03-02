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
use crate::{configuration::{DatabaseSettings, Settings}, nostr_client::NostrClient};

pub struct Application {
    bot: Bot,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let bot = build_bot(
            connection_pool,
            configuration.bot.mempool_space_url,
            configuration.bot.nostr_relays
        ).await?;
        Ok(Self { bot })
    } 
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.bot.run().await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

//Want to create a single bot that runs mempoolspace and nostr_client in different threads
pub async fn build_bot(
    db_pool: PgPool,
    mempool_url: &str,
    nostr_relays: [&str]
) -> Result<Bot, anyhow::Error>{
    let db_pool = Data::new(db_pool);
    let mempool_client = MempoolClient::build(self, configuration, db);
    let nostr_client = NostrClient::build(self, configuration, db);
    let bot = Bot {
        mempool_client: mempool_client,
        nostr_client: nostr_client
    };    
    Ok(bot)
}