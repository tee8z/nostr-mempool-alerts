use crossbeam_channel::{Receiver, Sender};
use futures_util::{future::join_all, Future};
use std::{
    io::ErrorKind,
    sync::{
        atomic::AtomicBool,
        Arc,
    },
    task::{Context, Poll},
};

use crate::{
    alert_manager::AlertManager, mempool_manager::MempoolManager, nostr_manager::NostrManager,
};

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
pub struct Channels<T> {
    pub send: Sender<Message<T>>,
    pub listen: Receiver<Message<T>>,
}
#[derive(Clone)]
pub struct Bot {
    pub alert_manager: AlertManager,
    pub mempool_manager: MempoolManager,
    pub nostr_manager: NostrManager,
    pub kill_signal: Arc<AtomicBool>,
}

impl Bot {
    pub async fn run(self) -> Result<(), std::io::Error> {
        let mut tasks = vec![];
        //spin up the two clients that internally handle keeping themselves running
        let mempool_manager_task = tokio::spawn(async { self.mempool_manager.await });
        tasks.push(mempool_manager_task);
        let nostr_manager_task = tokio::spawn(async { self.nostr_manager.await });
        tasks.push(nostr_manager_task);
        let alert_manager_task = tokio::spawn(async { self.alert_manager.await });
        tasks.push(alert_manager_task);
        let all_tasks = join_all(tasks);
        all_tasks.await;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct Message<T> {
    pub val: T,
}

impl Future for Bot {
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
                        format!("unexpected error in running bot tasks: {:?}", e),
                    )))
                }
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
