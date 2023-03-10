/*
* TODO:
*
* First implement with just rest endpoints and poll
* need methods to listen or poll for
* 1) get current block heigth & block hash: https://mempool.space/api/blocks/tip/height & https://mempool.space/api/blocks/tip/hash
* 2) get transactions included in new block: https://mempool.space/docs/api/rest#get-block-transaction-ids
* 3) get current mempool low/medium/high fees: (notified when they change) https://mempool.space/api/v1/fees/recommended
*
* Second, use websockets to be notifed when a new block is found (skip rest calls when possible)
* * websocket items
* const ws = websocket.initClient({
         options: ['blocks'],
       });
*/

use crate::bot::{self, Channels};
use futures_util::{pin_mut, StreamExt};
use nostr_sdk::nostr::url;
use reqwest::Client;
use std::{
    future::{ready, IntoFuture, Ready}, 
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    thread};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_tungstenite::connect_async;

pub struct MempoolManager {
    pub http_client: Client,
    alert_manager: Channels,
    pub mempool_space: String,
    connect_addr: String,
    pub kill_signal: Arc<AtomicBool>,
}

pub struct MempoolNetworkWS {}

impl MempoolNetworkWS {
    //listen to mempool websocket to be notified new block was found
    async fn listen_for_new_block(
        self,
        connect_addr: String,
        new_block_comm: Sender<bot::Message>,
    ) {
        print!("{}", connect_addr);
        let url = url::Url::parse(connect_addr.as_str()).unwrap();
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

        let (_, read) = ws_stream.split();

        let read_operation = {
            read.for_each(|message| async {
                let data = message.unwrap().into_data();
                println!("{:?}", data);
                new_block_comm
                    .send(bot::Message {
                        val: "current block height".into(),
                    })
                    .await
                    .unwrap();
            })
        };
        pin_mut!(read_operation);
        read_operation.await
    }
}

impl MempoolManager {
    pub async fn build(
        mempool_url: &str,
        alert_manager: Channels,
        network_type: String,
        kill_signal: Arc<AtomicBool>,
    ) -> MempoolManager {
        let client = reqwest::Client::new();
        let mut api_endpoint = "/api/v1/ws";
        let combo = format!("{}/{}", network_type, api_endpoint);
        if network_type != "mainnet" {
            api_endpoint = combo.as_ref();
        }
        let connect_addr = format!(
            "{}/{}",
            "wss://mempool.space".to_owned(),
            api_endpoint.to_owned()
        );
        Self {
            mempool_space: mempool_url.to_owned(),
            http_client: client,
            alert_manager: alert_manager,
            connect_addr: connect_addr,
            kill_signal: kill_signal,
        }
    }

    //https://mempool.space/api/v1/fees/recommended
    async fn mempool_recommended_fees(self) -> Self {
        println!("made it to mempool fees");
        self
    }
    //https://mempool.space/api/block/{block_hash}/txids
    async fn transactions(self) -> Self {
        println!("made it to transactions");
        self
    }
    //https://mempool.space/api/blocks/tip/height
    //https://mempool.space/api/blocks/tip/hash
    async fn current_block(self, new_block: bot::Message) -> Self {
        println!("made it to current block {:?}", new_block);
        self
    }
    async fn send_alert(self) {
        self.alert_manager
            .send
            .send(bot::Message {
                val: "sending".into(),
            })
            .await
            .unwrap();
    }

    //TODO: add in memory map of value want to send to alert manager
    async fn handle_new_block(self, mut new_block_recv: Receiver<bot::Message>) {
        let new_block = new_block_recv.recv().await.unwrap();
        print!("new_block {:?}", new_block);
        let _ = self
            .current_block(new_block)
            .await
            .transactions()
            .await
            .mempool_recommended_fees()
            .await
            .send_alert()
            .await;
    }
    pub fn run(self) -> Result<(),std::io::Error> { 
        let kill_signal = self.kill_signal.clone();
        let mempool_ws = MempoolNetworkWS {};
        let connect_addr = self.connect_addr.clone();
        let (send_new_block, listen_for_new_block) = mpsc::channel::<bot::Message>(1);
        let kill_mempool_watching_new_block = kill_signal.clone();
        tokio::spawn(async move {
            mempool_ws
                .listen_for_new_block(connect_addr, send_new_block)
                .await;
            if kill_mempool_watching_new_block.load(Ordering::Relaxed) {
                return
            }
        });
        let kill_handle_new_block = kill_signal.clone();
        tokio::spawn(async move {
            self.handle_new_block(listen_for_new_block).await;
            if kill_handle_new_block.load(Ordering::Relaxed) {
                return
            }
        });
        //keep thread alive until kill signal is sent
        while !kill_signal.load(Ordering::Relaxed) { 
            thread::park();
        }
        Ok(())
    }
}

impl IntoFuture for MempoolManager {
    type Output = Result<(), std::io::Error>;
    type IntoFuture = Ready<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        ready(self.run())
    }
}
