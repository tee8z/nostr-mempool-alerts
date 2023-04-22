use crate::alert::Alert;
use crate::bot::{self, Channels};
use crate::{configuration::NostrSettings, error_fmt::error_chain_fmt};
use futures_util::{future::join_all, Future};
use nostr_sdk::prelude::*;
use secrecy::{ExposeSecret, Secret};
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
    pub private_key: Secret<String>,
    pub public_key: String,
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
        let private_key = Secret::new(configuration.private_key.expose_secret().to_owned());
        let listen_relays = configuration.nostr_relays;
        let bot_keys =
            Keys::from_sk_str(private_key.expose_secret()).map_err(anyhow::Error::new)?;
        let client = Client::new(&bot_keys);
        let public_key = client.keys().public_key().to_string();

        for listen in listen_relays.iter() {
            client.add_relay(listen, None).await?;
        }
        client.connect().await;
        let metadata = Metadata::new()
       // .name("mempool space bot")
        //.display_name("mempool space bot")
        .about("a block notification bot that will publish a notification to a user when a block target has been hit or a block number has been reached");
        //.nip05()
        //.lud16()
        client.set_metadata(metadata).await?;
        Ok(Self {
            private_key,
            public_key,
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
        let direct_message_sender = tokio::spawn(async move {
            loop {
                tracing::info!("starting to listen for message from alert manager in nostr manager");
                let message = alert_listen
                    .recv()
                    .expect("error receiving message from alert manager");
                tracing::info!("new message picked up in nostr manager from alert manage: {:?}", message);

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
                if kill_alert_watcher.load(Ordering::Relaxed) {
                    break;
                }
            }
        });
        tasks.push(direct_message_sender);
        let kill_notification_watcher = kill_signal.clone();
        let notification_listener = tokio::spawn(async move {
            let mut notifcations = client_notification.notifications();
            while let Ok(notifcation) = notifcations.recv().await {
                if kill_notification_watcher.load(Ordering::Relaxed) {
                    break;
                }
                tracing::info!("notification: {notifcation:?}");
                alert_send
                    .send(bot::Message {
                        val: format!("{:?}", notifcation),
                    })
                    .expect("error sending message to the alert manager");
            }
        });
        tasks.push(notification_listener);
        let nostr_handler = join_all(tasks);
        nostr_handler.await;
        Ok(())
    }
}

#[instrument(skip_all)]
pub async fn build_nostr_message(alert: Alert) -> Result<NostrAlertMessage, NostrError> {
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
