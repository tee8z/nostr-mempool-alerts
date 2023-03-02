use crate::configuration::NostrSettings;
use nostr_sdk::prelude::*;
use secrecy::{Secret, ExposeSecret};
use sqlx::PgPool;


pub struct NostrClient {
    private_key: Secret<String>,
    public_key: String,
    client: Client,
    listen_relays: Vec<String>,
    db_pool: PgPool
}

impl NostrClient {
    pub async fn build(configuration: NostrSettings, db: PgPool) -> Result<Self, anyhow::Error> {
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
        })
    }
    /*
        for listen in self.listen_relays.iter() {
            self.client.add_relay(listen, None).await?;
        }

        self.client.connect().await;
        let metadata = Metadata::new()
        .name("block bot")
        .display_name("block bot")
        .about("a block notification bot that will publish a notification to a user when a block target has been hit or a block number has been reached");
            //.nip05()
            //.lud16()
        
        self.client.set_metadata(metadata).await?;
        Ok((self))
    }   */

    pub async fn post_nostr(self, relay_name: String) {
        /*
         TODO: 
            * there will be multiple types of alert messages, that logic will probably needed to be handled here
            * maybe we use the strategy pattern here for posting the different types?
         */
    }

    pub async fn listen_messages(self){
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
