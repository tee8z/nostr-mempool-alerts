use std::{future::{Ready, IntoFuture, ready}, sync::{atomic::{AtomicBool, Ordering}, Arc}};
use sqlx::PgPool;
use tokio::{sync::{mpsc, mpsc::{Sender, Receiver}}, signal::unix::{SignalKind, signal}};
use std::thread;
use crate::{nostr_manager::NostrManager, mempool_manager::MempoolManager, alert_manager::AlertManager};




/*
Lifecycle of an alert
1) user messages bot via nostr "hey I'd like to listen for block height 755678"
2) nostr_manager listens to nostr stream for message & stores new messages
3) nostr_manager sends "watch" request to alert_manager
4) mempool_manager makes batch requests/subscriptions for all needed data in background, store in local DB? 
5) mempool_manger, when new data comes in will notify alert_manager
7) alert_manager on "watch request" from nostr_manager, add to in-memory collection of currently watching
8) alert_manger will have a different type of alert threads for each user type of request
9) alert_manager de-dup alerts, keep track of when last sent
8) alert_manager decides if an alert needs to be sent, pass the required payload to the nostr_manager
10) nostr_mananger fires alert off to user
 */


/*
 * - block height - all same data
 * - mempool fee - all same data
 * - transaction confirmation height - can be calculated if we know when the first block it was found it was
 * - utxo movement - this is user request specific and thus trickiest to implement
 */


#[derive(Clone)]
pub struct Channels {
    pub send: Sender<Message>,
    pub listen: Arc<Receiver<Message>>,
}

pub struct Bot {
    pub db_pool: PgPool,
    pub alert_manager: AlertManager,
    pub mempool_manager: MempoolManager,
    pub nostr_manager: NostrManager,
    pub kill_signal: Arc<AtomicBool>,
}

impl Bot {
    pub fn run(self) -> Result<(), std::io::Error>{
        
        //spin up the two clients that internally handle keeping themselves running
        tokio::spawn(async {
            self.mempool_manager.await
        });

        tokio::spawn(async {
            self.nostr_manager.await
        });

        tokio::spawn(async {
            self.alert_manager.await
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
