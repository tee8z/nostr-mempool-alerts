use crate::{
    alert::{AlertCommunication, AlertManager},
    bot::{Bot, Channels, Message},
    configuration::{DatabaseSettings, NostrSettings, Settings},
    mempool::{MempoolData, MempoolManager},
    nostr::NostrManager,
};
use signal_hook::flag;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tracing::instrument;
pub struct Application {
    bot: Bot,
}

impl Application {
    #[instrument(skip_all)]
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let bot = build_bot(
            connection_pool,
            &configuration.mempool.url,
            configuration.nostr,
        )
        .await?;
        Ok(Self { bot })
    }
    #[instrument(skip_all)]
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        flag::register(
            signal_hook::consts::SIGTERM,
            Arc::clone(&self.bot.kill_signal),
        )?;
        self.bot.await
    }
}

#[instrument(skip_all)]
pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

#[instrument(skip_all)]
pub async fn build_bot(
    db_pool: PgPool,
    mempool_url: &str,
    nostr_configuration: NostrSettings,
) -> Result<Bot, anyhow::Error> {
    //NOTE: wires up communication between processes
    let (send_to_nostr, listen_from_nostr) = crossbeam_channel::unbounded::<Message<String>>();
    let (send_to_alert_nostrbot, listen_from_alert) =
        crossbeam_channel::unbounded::<Message<String>>();
    let alert_nostr = Channels {
        send: send_to_nostr,
        listen: listen_from_alert,
    };
    let nostr_comm = Channels {
        send: send_to_alert_nostrbot,
        listen: listen_from_nostr,
    };
    let (send_to_alert_membot, listen_from_membot) =
        crossbeam_channel::unbounded::<Message<MempoolData>>();
    let alert_mempool = Channels {
        send: send_to_alert_membot,
        listen: listen_from_membot,
    };
    let alert_coms = AlertCommunication {
        mempool_com: alert_mempool.clone(),
        nostr_com: alert_nostr,
    };
    let kill_signal = Arc::new(AtomicBool::new(false));
    //NOTE: wires up background processes (will need one for each network we want to support, ie mainnet, testnet, signet, regtest)
    //TODO: make mempool_managers configurable (mainnet/testnet/signet/regtest)
    let mempool_manager = MempoolManager::build(
        mempool_url,
        alert_mempool,
        "mainnet".into(),
        kill_signal.clone(),
    );
    let nostr_manager = NostrManager::build(
        db_pool.clone(),
        nostr_configuration,
        nostr_comm,
        kill_signal.clone(),
    )
    .await?;
    let alert_manager = AlertManager::build(db_pool, alert_coms, kill_signal.clone()).await?;

    //TODO: add trace around DB queries
    let bot = Bot {
        mempool_manager,
        nostr_manager,
        alert_manager,
        kill_signal,
    };
    Ok(bot)
}
