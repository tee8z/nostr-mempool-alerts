use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use crate::{
    bot::{Bot, Channels, Message},
    configuration::{DatabaseSettings, NostrSettings, Settings},
    nostr_manager::NostrManager,
    mempool_manager::MempoolManager, alert_manager::{AlertManager,AlertCommunication}
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
    // wire up communication between processes
    let (send_to_nostr, mut listen_from_nostr) = mpsc::channel::<Message>(0);
    let (send_to_alert_nostrbot, mut listen_from_alert) = mpsc::channel::<Message>(0);
    let alert_nostr = Channels {
        send: send_to_nostr,
        listen: listen_from_alert.into()
    };
    let nostr_comm = Channels {
        send: send_to_alert_nostrbot,
        listen: listen_from_nostr.into(),
    };
    let (send_to_membot, mut listen_from_alert) = mpsc::channel::<Message>(0);
    let (send_to_alert_membot, mut listen_from_membot) = mpsc::channel::<Message>(0);
    let mempool_comm = Channels {
        send: send_to_membot,
        listen: listen_from_alert.into(),
    };
    let alert_mempool = Channels {
        send: send_to_alert_membot,
        listen: listen_from_membot.into()
    };
    let alert_coms = AlertCommunication {
        mempool_com: alert_mempool,
        nostr_com: alert_nostr
    };

    // wire up background processes
    let mempool_manager = MempoolManager::build(mempool_url, db_pool.clone(), nostr_comm.clone(), "mainnet".into()).await;
    let nostr_manager = NostrManager::build(nostr_configuration, db_pool.clone(), mempool_comm.clone()).await?;
    let alert_manger = AlertManager::build(db_pool.clone(), alert_coms).await;

    //TODO: add trace around DB queries
    let bot = Bot {
        db_pool: db_pool.clone(),
        mempool_manager: mempool_manager,
        nostr_manager: nostr_manager,
        alert_manager: alert_manger,
        kill_signal: Arc::new(AtomicBool::new(false))
    };
    Ok(bot)
}
