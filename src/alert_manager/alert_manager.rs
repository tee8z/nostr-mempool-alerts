use sqlx::PgPool;
use std::{
    future::{ready, IntoFuture, Ready}, 
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    thread};
use crate::bot::Channels;


pub struct AlertCommunication {
    pub nostr_com: Channels,
    pub mempool_com: Channels
}

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
    pub fn run(self) -> Result<(),std::io::Error> { 
        let kill_signal = self.kill_signal.clone();
        /*tokio::spawn(async {
            mempool_ws
                .listen_for_new_block(connect_addr, send_new_block)
                .await;
        });

        tokio::spawn(async {
            self.handle_new_block(listen_for_new_block).await;
        });*/
        //keep thread alive until kill signal is sent
        while !kill_signal.load(Ordering::Relaxed) { 
            thread::park();
        }
        Ok(())
    }
}


impl IntoFuture for AlertManager {
    type Output = Result<(), std::io::Error>;
    type IntoFuture = Ready<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        ready(self.run())
    }
}