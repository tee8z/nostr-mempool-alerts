use sqlx::PgPool;
use std::{
    sync::{atomic::{AtomicBool}, Arc},
    task::{Context, Poll}, io::ErrorKind};
use futures_util::Future;
use crate::{bot::Channels, mempool_manager::MempoolData};

#[derive(Clone)]
pub struct AlertCommunication {
    pub nostr_com: Channels<String>,
    pub mempool_com: Channels<MempoolData>
}
#[derive(Clone)]
pub struct AlertManager {
    pub db_pool: PgPool,
    pub communication: AlertCommunication, 
    kill_signal: Arc<AtomicBool>,
}
impl AlertManager {
    pub async fn build(db: PgPool, communication: AlertCommunication, kill_signal: Arc<AtomicBool>) -> AlertManager {
       
        Self {
            db_pool: db,
            communication: communication,
            kill_signal: kill_signal
        }
    }
    pub async fn run(self) -> Result<(),std::io::Error> { 
        let _kill_signal = self.kill_signal.clone();
        /*tokio::spawn(async {
            mempool_ws
                .listen_for_new_block(connect_addr, send_new_block)
                .await;
        });

        tokio::spawn(async {
            self.handle_new_block(listen_for_new_block).await;
        });*/
        //keep thread alive until kill signal is sent
        
        Ok(())
    }
}


impl Future for AlertManager {
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
                        format!("unexpected error in running alert manager: {:?}", e),
                    )))
                }
            },
            Poll::Pending => Poll::Pending,
        }
    }
}