
/*
TODO: here we need to handle the business logic of marshalling mempool data/messages into alerts that are need by the clients of this bot

 */

use std::future::{Ready, IntoFuture, ready};

use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::{nostr_client::NostrClient, mempool_client::MempoolClient};
//mpsc::channel()
pub struct Bot {
    pub db_pool: PgPool,
    pub mempool_client: MempoolClient,
    pub nostr_client: NostrClient
}

struct Message {
    val: String
}

impl IntoFuture for Bot {
    type Output = Result<(), std::io::Error>;
    type IntoFuture = Ready<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
         let (tx,mut rx) = mpsc::channel::<Message>(0);
         //implement the bot loop
        ready(Ok(()))
    }
}