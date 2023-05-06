use crate::alert::Alert;
use crate::bot::{Channels, Message};
use crate::{configuration::NostrSettings, error_fmt::error_chain_fmt};
use crossbeam_channel::Sender;
use futures_util::{future::join_all, Future};
use nostr_sdk::prelude::*;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::io::ErrorKind;
use std::{
    fmt::Debug,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};
use tracing::instrument;

use super::NostrAlertMessage;

#[derive(thiserror::Error)]
pub enum NostrError {
    #[error("Failed to send request")]
    FailedToSend,
    #[error("Failed to validate pubkey")]
    FailedPubkeyValidation,
    #[error("Failed to save alert notification")]
    FailedToSave,
}

impl Debug for NostrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
#[derive(Clone)]
pub struct NostrManager {
    pub keys: Keys,
    pub listen_relays: Vec<String>,
    pub kill_signal: Arc<AtomicBool>,
    alert_manager: Channels<String>,
    client: Client,
    db: PgPool,
}

impl NostrManager {
    #[instrument(skip_all)]
    pub async fn build(
        db: PgPool,
        configuration: NostrSettings,
        alert_manager: Channels<String>,
        kill_signal: Arc<AtomicBool>,
    ) -> Result<Self, anyhow::Error> {
        let private_key =
            SecretKey::from_bech32(configuration.private_key.expose_secret().to_owned())?;
        let listen_relays = configuration.nostr_relays;
        let keys = Keys::new(private_key);
        let opts = Options::new().wait_for_send(false);
        let client = Client::with_opts(&keys, opts);
        let public_key = client.keys().public_key().to_bech32()?;
        tracing::info!("nostr pubkey for bot: {:?}", public_key);
        for listen in listen_relays.iter() {
            client.add_relay(listen, None).await?;
        }
        client.connect().await;
        let metadata = Metadata::new()
        .name("mempool_space_bot")
        .display_name("mempool space bot")
        .about("a block notification bot that will publish a notification to a user when a block target has been hit or a block number has been reached");
        //.nip05()
        //.lud16()
        client.set_metadata(metadata).await?;

        let subscription = Filter::new()
            .pubkey(keys.public_key())
            .kind(Kind::EncryptedDirectMessage)
            .since(Timestamp::now());

        client.subscribe(vec![subscription]).await;

        Ok(Self {
            keys,
            client,
            listen_relays,
            alert_manager,
            kill_signal,
            db,
        })
    }
    //TODO: remove the need for expect() calls
    #[instrument(skip_all)]
    pub async fn run(self) -> Result<(), std::io::Error> {
        let kill_signal = self.kill_signal.clone();
        let alert_listen = self.alert_manager.listen.clone();
        let alert_send = self.alert_manager.send.clone();
        let client = self.client.clone();
        let client_notification = client.clone();
        let kill_alert_watcher = kill_signal.clone();
        let mut tasks = vec![];
        let keys = self.keys.clone();
        let direct_message_sender = tokio::spawn(async move {
            loop {
                if kill_alert_watcher.load(Ordering::Relaxed) {
                    break;
                }
                tracing::info!(
                    "starting to listen for message from alert manager in nostr manager"
                );
                let raw_message: Option<Message<String>> = alert_listen.recv().ok();
                if raw_message.is_none() {
                    tracing::warn!("no data found in message from alert manager in nostr manager");

                    continue;
                }
                let message = raw_message.unwrap();
                tracing::info!(
                    "new message picked up in nostr manager from alert manage: {:?}",
                    message
                );

                let alert: Alert = serde_json::from_str(&message.val)
                    .expect("error trying to convert alert json into struct");

                let nostr_message = build_nostr_message(alert)
                    .await
                    .expect("error bulding nostr message from alert");
                direct_message_nostr(client.clone(), nostr_message.clone())
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!("error sending direct message to nostr: {:?}", e)
                    });
                update_alert(self.db.clone(), nostr_message)
                    .await
                    .unwrap_or_else(|e| tracing::error!("error updating nostr alert: {:?}", e));
            }
        });
        tasks.push(direct_message_sender);
        let kill_notification_watcher = kill_signal.clone();

        let notification_listener = tokio::spawn(async move {
            if kill_notification_watcher.load(Ordering::Relaxed) {
                return;
            }
            match client_notification
                .clone()
                .handle_notifications(|notification| async {
                    if kill_notification_watcher.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    if let RelayPoolNotification::Event(_url, event) = notification {
                        if event.kind == Kind::EncryptedDirectMessage {
                            match decrypt(
                                &keys.clone().secret_key()?,
                                &event.pubkey,
                                &event.content,
                            ) {
                                Ok(msg) => {
                                    tracing::info!("notification: {msg:?}");
                                    let content: String = match msg.as_str() {
                                        "/block_height" => block_height(alert_send.clone(), msg),
                                        "/fees" => fees(alert_send.clone(), msg),
                                        "/transaction" => transaction(alert_send.clone(), msg),
                                        "/help" => help(),
                                        _ => String::from(
                                            "Invalid command, send /help to see all commands.",
                                        ),
                                    };

                                    client_notification
                                        .clone()
                                        .send_direct_msg(event.pubkey, content)
                                        .await?;
                                }
                                Err(e) => {
                                    tracing::error!("Impossible to decrypt direct message: {e}")
                                }
                            }
                        }
                    }
                    Ok(())
                })
                .await
            {
                Ok(_) => {}
                Err(e) => tracing::error!("error handling nostr notification: {:?}", e),
            }
        });
        tasks.push(notification_listener);
        let nostr_handler = join_all(tasks);
        nostr_handler.await;
        Ok(())
    }
}

//TODO: map the user's request to an alert needing to be set up
fn block_height(_alert_send: Sender<Message<String>>, _msg: String) -> String {
    "".to_string()
}

fn fees(_alert_send: Sender<Message<String>>, _msg: String) -> String {
    "".to_string()
}

fn transaction(_alert_send: Sender<Message<String>>, _msg: String) -> String {
    "".to_string()
}

fn help() -> String {
    let mut output = String::new();
    output.push_str("Commands:\n");
    output.push_str("/block_height - Be alerted a given block height has been reached. ex: `/block_height 61774`\n");
    output.push_str("/fees - Be alerted when mempool fees have reached a given level for a transaction to be confirmed in a half-hour. ex: `/fees 2.0`\n");
    output.push_str("/transaction - Be alerted when a transaction has reached a certain number of confirmations. ex: (be notified with this transaction has been confirmed 3 times) `/transaction 91b8def136d9b261dd23082dad6424f1d3e324107ab096eda5648b3cd269e0bc 3`\n");
    output.push_str("/help - Help");
    output
}

#[instrument(skip_all)]
pub async fn build_nostr_message(alert: Alert) -> Result<NostrAlertMessage, NostrError> {
    //TODO: build translation of alert to message
    Ok(NostrAlertMessage {
        client_pk: alert.requestor_pk,
        val: "".to_string(),
        id: alert.id,
    })
}

#[instrument(skip_all)]
pub async fn update_alert(db: PgPool, message: NostrAlertMessage) -> Result<(), NostrError> {
    sqlx::query!(
        r#"
        INSERT INTO notifications (alert_id, sent_message)
        VALUES ($1, $2)
        RETURNING id, sent_at
        "#,
        message.id,
        message.val
    )
    .fetch_one(&db)
    .await
    .map_err(|_| NostrError::FailedToSave)?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn direct_message_nostr(
    client: Client,
    msg: NostrAlertMessage,
) -> Result<(), NostrError> {
    let pubkey = XOnlyPublicKey::from_str(msg.client_pk.as_str())
        .map_err(|_| NostrError::FailedPubkeyValidation)?;

    client
        .send_direct_msg(pubkey, msg.val)
        .await
        .map_err(|_| NostrError::FailedToSend)
        .map(|_| ())
}

impl Future for NostrManager {
    type Output = Result<(), std::io::Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let async_fn = async { self.clone().run().await };
        let mut future = Box::pin(async_fn);

        match future.as_mut().poll(cx) {
            Poll::Ready(res) => match res {
                Ok(_) => Poll::Ready(Ok(())),
                Err(e) => Poll::Ready(Err(std::io::Error::new(
                    ErrorKind::Other,
                    format!("unexpected error in running nostr tasks: {:?}", e),
                ))),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
