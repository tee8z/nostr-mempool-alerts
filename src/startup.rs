use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::bot::Bot;
use crate::{
    bot::{Channels, Message},
    configuration::{DatabaseSettings, NostrSettings, Settings},
    mempool_client::MempoolClient,
    nostr_client::NostrClient,
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::sync::mpsc;
use signal_hook::flag;
pub struct Application {
    bot: Bot,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let bot = build_bot(
            connection_pool,
            &configuration.bot.mempool_url,
            configuration.bot.nostr_settings,
        )
        .await?;
        Ok(Self { bot })
    }
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        flag::register(signal_hook::consts::SIGTERM, Arc::clone(&self.bot.kill_signal))?;
        self.bot.await
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
    nostr_configuration: NostrSettings,
) -> Result<Bot, anyhow::Error> {
    let (send_to_nostr, mut listen_from_nostr) = mpsc::channel::<Message>(0);
    let nostr_comm = Channels {
        send: send_to_nostr,
        listen: listen_from_nostr,
    };
    let (send_to_membot, mut listen_from_membot) = mpsc::channel::<Message>(0);
    let mempool_comm = Channels {
        send: send_to_membot,
        listen: listen_from_membot,
    };
    let mempool_client = MempoolClient::build(mempool_url, db_pool.clone(), nostr_comm).await;
    let nostr_client =
        NostrClient::build(nostr_configuration, db_pool.clone(), mempool_comm).await?;
    //TODO: add trace around DB queries
    let bot = Bot {
        db_pool: db_pool.clone(),
        mempool_client: mempool_client,
        nostr_client: nostr_client,
        kill_signal: Arc::new(AtomicBool::new(false))
    };
    Ok(bot)
}
