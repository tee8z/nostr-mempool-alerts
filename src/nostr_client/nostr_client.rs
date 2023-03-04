use crate::bot::{Channels, Message};
use crate::{configuration::NostrSettings, error_fmt::error_chain_fmt};
use nostr_sdk::prelude::*;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use std::fmt::Debug;
use std::future::{IntoFuture, Ready, ready};
use std::str::FromStr;

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

pub struct NostrClient {
    private_key: Secret<String>,
    public_key: String,
    client: Client,
    listen_relays: Vec<String>,
    db_pool: PgPool,
    mempool_comm: Channels,
}

impl NostrClient {
    pub async fn build(
        configuration: NostrSettings,
        db: PgPool,
        mempool_comm: Channels,
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
        .name("block bot")
        .display_name("block bot")
        .about("a block notification bot that will publish a notification to a user when a block target has been hit or a block number has been reached");
        //.nip05()
        //.lud16()
        client.set_metadata(metadata).await?;
        Ok(Self {
            private_key: private_key,
            public_key: public_key,
            client: client,
            listen_relays: listen_relays,
            db_pool: db,
            mempool_comm: mempool_comm,
        })
    }

    pub async fn direct_message_nostr(
        self,
        client_pk: &str,
        msg: Message,
    ) -> Result<(), NostrError> {
        let pubkey = XOnlyPublicKey::from_str(client_pk.into())
            .map_err(|_| NostrError::FailedPubkeyValidation)?;

        self.client
            .send_direct_msg(pubkey, msg.val)
            .await
            .map_err(|_| NostrError::FailedToSend)
            .map(|_| ())
    }

    pub async fn listen_messages(self) {
        let mut notifcations = self.client.notifications();
        while let Ok(notifcation) = notifcations.recv().await {
            println!("{notifcation:?}");

            /*TODO:
             * Add to the DB the user's alert request, use their pubkey to assoicate the alert
             * Send a response back to the user who requested the alert to confirm it was recieved
             */
        }
    }
}


impl IntoFuture for NostrClient {
    type Output = Result<(), std::io::Error>;
    type IntoFuture = Ready<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        //TODO: call the forever loop here to continue listen to our configured list of relays
        ready(Ok(()))
    }
}