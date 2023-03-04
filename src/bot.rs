use std::{future::{Ready, IntoFuture, ready}, sync::{atomic::{AtomicBool, Ordering}, Arc}};
use sqlx::PgPool;
use tokio::{sync::{mpsc, mpsc::{Sender, Receiver}}, signal::unix::{SignalKind, signal}};
use std::thread;
use crate::{nostr_client::NostrClient, mempool_client::MempoolClient};

pub struct Channels {
    pub send: Sender<Message>,
    pub listen: Receiver<Message>,
}

pub struct Bot {
    pub db_pool: PgPool,
    pub mempool_client: MempoolClient,
    pub nostr_client: NostrClient,
    pub kill_signal: Arc<AtomicBool>,
}

impl Bot {
    pub fn run(self) -> Result<(), std::io::Error>{
        //spin up the two clients that internally handle keeping themselves running
        tokio::spawn(async {
            self.mempool_client.await
        });

        tokio::spawn(async {
            self.nostr_client.await
        });
        //keep thread alive until kill signal is sent
        while !self.kill_signal.load(Ordering::Relaxed) { 
            thread::park();
        }
        Ok(())
    }
}

pub struct Message {
    pub val: String
}

impl IntoFuture for Bot {
    type Output = Result<(), std::io::Error>;
    type IntoFuture = Ready<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        ready(self.run())
    }
}
