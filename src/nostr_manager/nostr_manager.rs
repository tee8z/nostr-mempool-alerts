use crate::bot::{self, Channels, Message};
use crate::{configuration::NostrSettings, error_fmt::error_chain_fmt};
use nostr_sdk::prelude::*;
use secrecy::{ExposeSecret, Secret};
use futures_util::{future::join_all, Future};
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

#[derive(thiserror::Error)]
pub enum NostrError {
    #[error("Failed to send request")]
    FailedToSend,
    #[error("Failed to validate pubkey")]
    FailedPubkeyValidation,
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
    client: Client,
    pub listen_relays: Vec<String>,
    alert_manager: Channels<String>,
    pub kill_signal: Arc<AtomicBool>,
}

impl NostrManager {
    pub async fn build(
        configuration: NostrSettings,
        alert_manager: Channels<String>,
        kill_signal: Arc<AtomicBool>,
    ) -> Result<Self, anyhow::Error> {
        let private_key = Secret::new(configuration.private_key.expose_secret().to_owned());
        let listen_relays = configuration.nostr_relays;
        let bot_keys = Keys::from_sk_str(private_key.expose_secret()).unwrap();
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
            private_key: private_key,
            public_key: public_key,
            client: client,
            listen_relays: listen_relays,
            alert_manager: alert_manager,
            kill_signal: kill_signal,
        })
    }

    pub async fn get_client_pk(self) -> String {
        "test".to_owned()
    }
    pub async fn run(self) -> Result<(), std::io::Error> {
        let kill_signal = self.kill_signal.clone();
        let alert_listen = self.alert_manager.listen.clone();
        let alert_send = self.alert_manager.send.clone();
        let client = self.client.clone();
        let client_notification = client.clone();
        // let client_pk = self.get_client_pk().await;
        let kill_alert_watcher = kill_signal.clone();
        let mut tasks = vec![];
        let direct_message_sender = tokio::spawn(async move {
            loop {
                let message = alert_listen.recv().unwrap();
                let fake_pk = "testing_pk";
                direct_message_nostr(client.clone(), fake_pk, message)
                    .await
                    .unwrap();
                if kill_alert_watcher.load(Ordering::Relaxed) {
                    break
                }
            }
        });
        tasks.push(direct_message_sender);
        let kill_notification_watcher = kill_signal.clone();
        let notification_listener = tokio::spawn(async move {
            let mut notifcations = client_notification.notifications();
            while let Ok(notifcation) = notifcations.recv().await {
                println!("{notifcation:?}");
                alert_send
                    .send(bot::Message {
                        val: format!("{:?}", notifcation),
                    })
                    .unwrap();
                if kill_notification_watcher.load(Ordering::Relaxed) {
                    return
                }
            }
        });
        tasks.push(notification_listener);
        let nostr_handler = join_all(tasks);
        nostr_handler.await;
        Ok(())
    }
}

pub async fn direct_message_nostr(
    client: Client,
    client_pk: &str,
    msg: Message<String>,
) -> Result<(), NostrError> {
    let pubkey = XOnlyPublicKey::from_str(client_pk.into())
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
        // Call an async function using the `async` keyword and `await` keyword
        let async_fn = async { self.clone().run().await };

        // Convert the async function to a future using `Box::pin`
        let mut future = Box::pin(async_fn);

        // Poll the future using `poll` on the returned `Pin` reference
        match future.as_mut().poll(cx) {
            Poll::Ready(res) => match res {
                Ok(_) => return Poll::Ready(Ok(())),
                Err(e) => {
                    return Poll::Ready(Err(std::io::Error::new(
                        ErrorKind::Other,
                        format!("unexpected error in running nostr tasks: {:?}", e),
                    )))
                }
            },
            Poll::Pending => Poll::Pending,
        }
    }
}