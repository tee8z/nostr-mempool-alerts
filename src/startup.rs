use sqlx::{PgPool, postgres::PgPoolOptions};
use crate::{configuration::{DatabaseSettings, Settings, NostrSettings}, nostr_client::NostrClient, mempool_client::MempoolClient};
use crate::{bot::Bot};

pub struct Application {
    bot: Bot,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let bot = build_bot(
            connection_pool,
            &configuration.bot.mempool_url,
            configuration.bot.nostr_settings
        ).await?;
        Ok(Self { bot })
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

//Want to create a single bot that runs mempoolspace and nostr_client in different threads
pub async fn build_bot(
    db_pool: PgPool,
    mempool_url: &str,
    nostr_configuration: NostrSettings
) -> Result<Bot, anyhow::Error>{
    //TODO: add trace around DB queries

    let mempool_client = MempoolClient::build(mempool_url, db_pool.clone()).await;
    let nostr_client = NostrClient::build(nostr_configuration, db_pool.clone()).await?;
    
    let bot = Bot {
        db_pool: db_pool.clone(),
        mempool_client: mempool_client,
        nostr_client: nostr_client
    };    
    Ok(bot)
}