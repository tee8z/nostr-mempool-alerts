use super::{BlockTip, MempoolData, RecommendedFees, TransactionID};
use crate::{
    bot::{self, Channels},
    mempool_manager::MempoolRaw,
};
use crossbeam_channel::{Receiver, Sender};
use futures_util::{future::join_all, pin_mut, FutureExt, SinkExt, StreamExt};
use nostr_sdk::nostr::url;
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    str,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
    vec, io::ErrorKind,
};
use tokio_tungstenite::{connect_async, tungstenite};

#[derive(Clone)]
pub struct MempoolManager {
    pub mempool_space: String,
    connect_addr: String,
    pub kill_signal: Arc<AtomicBool>,
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
}

pub struct MempoolNetworkWS {}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MempoolMessage {
    pub action: String,
    pub data: Option<Vec<String>>,
}

impl MempoolNetworkWS {
    //listen to mempool websocket to be notified new block was found
    async fn listen_for_new_block(
        self,
        connect_addr: String,
        new_block_comm: Sender<bot::Message<MempoolRaw>>,
        kill_signal: Arc<AtomicBool>,
    ) {
        let url = url::Url::parse(connect_addr.as_str()).unwrap();
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

        let (mut write, read) = ws_stream.split();
        let init_message = serde_json::to_string(&MempoolMessage {
            action: "init".to_owned(),
            data: None,
        })
        .unwrap();
        let set_want = serde_json::to_string(&MempoolMessage {
            action: "want".to_owned(),
            data: Some(vec!["blocks".to_owned()]),
        })
        .unwrap();

        write
            .send(tokio_tungstenite::tungstenite::Message::Text(init_message))
            .await
            .expect("failed to send init message to mempool.space");
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(set_want))
            .await
            .expect("failed to send want message to mempool.space");

        let mut tasks = vec![];
        let ping_kill_signal = kill_signal.clone();
        let ping = tokio::spawn(async move {
            let spleed_time = tokio::time::Duration::new(30, 0);
            loop {
                if ping_kill_signal.clone().load(Ordering::Relaxed) {
                    tracing::info!("stopping pinging mempool.space");
                    return;
                }
                let ping = tungstenite::protocol::Message::Ping(vec![0; 124]);
                match write.send(ping).await {
                    Ok(_) => {
                        tokio::time::sleep(spleed_time).await;
                    }
                    Err(e) => {
                        tracing::error!("error sending ping message to mempool.space: {:?}", e);
                        break;
                    }
                }
            }
        })
        .boxed();
        tasks.push(ping);

        let read_operations = tokio::spawn(async move {
            let read_operation = {
                read.for_each(|message| async {
                    if kill_signal.load(Ordering::Relaxed) {
                        tracing::info!("stopping listening for new blocks from the mempool");
                        return;
                    }
                    let data = match message {
                        Ok(message) => Some(message.into_data()),
                        Err(e) => {
                            tracing::error!("error listening for new blocks: {:?}", e);
                            None
                        }
                    };
                    if data == None || data == Some(vec![0; 124]) {
                        return;
                    }
                    let binary_data = data.unwrap();
                    let new_data = match String::from_utf8(binary_data) {
                        Ok(v) => v,
                        Err(e) => panic!("invalid UTF-8 sequence: {}", e),
                    };

                    let data_val: MempoolRaw = serde_json::from_str(&new_data)
                        .expect("error marshalling mempool websocket data to block root");
                    tracing::info!("new block was found from the mempool! {:?}", data_val.backend_info.hostname);
                    new_block_comm.send(bot::Message { val: data_val }).unwrap();
                })
            };
            pin_mut!(read_operation);
            tracing::info!("starting to listen for new blocks");
            tracing::info!("listening to: {}", connect_addr);
            read_operation.await
        })
        .boxed();
        tasks.push(read_operations);
        let mempool_handlers = join_all(tasks);
        mempool_handlers.await;
    }
}

impl MempoolManager {
    pub async fn build(
        mempool_url: &str,
        alert_manager: Channels<MempoolData>,
        network_type: String,
        kill_signal: Arc<AtomicBool>,
    ) -> MempoolManager {
        let mut api_endpoint = "api/v1/ws";
        let combo = format!("{}/{}/", network_type, api_endpoint);
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
            send_to_alert_manager: alert_manager.send,
            connect_addr: connect_addr,
            kill_signal: kill_signal,
        }
    }

    pub async fn run(self) -> bool {
        let mempool_ws = MempoolNetworkWS {};
        let kill_signal = self.kill_signal.clone();
        let connect_addr = self.connect_addr.clone();
        let (send_new_block, listen_for_new_block) =
            crossbeam_channel::unbounded::<bot::Message<MempoolRaw>>();
        let kill_mempool_watching_new_block = kill_signal.clone();
        let mempool_manager = self.clone();
        let base_url = mempool_manager.mempool_space.clone();
        let send_to_alert = self.send_to_alert_manager.clone();
        let kill_handle_new_block = kill_signal.clone();
        let mut tasks = vec![];
        let watch_for_new_block = tokio::spawn(async move {
            if kill_mempool_watching_new_block.load(Ordering::Relaxed) {
                tracing::info!("stopping watching the mempool");
                return;
            }
            mempool_ws
                .listen_for_new_block(
                    connect_addr,
                    send_new_block,
                    kill_mempool_watching_new_block.clone(),
                )
                .await;
        })
        .boxed();
        let watch_current_state = tokio::spawn(async move {
            //get current state on start up and send to alert manager
            tracing::info!("getting initial state");
            build_mempool_state(base_url.clone(), send_to_alert.clone()).await;
            if kill_handle_new_block.load(Ordering::Relaxed) {
                tracing::info!("stopping listening for new block state");
                return;
            }
            //wait for new block and send new state to alert manager
            handle_new_block(
                listen_for_new_block,
                send_to_alert.clone(),
                kill_handle_new_block.clone(),
            )
            .await;
        })
        .boxed();
        tasks.push(watch_current_state);
        tasks.push(watch_for_new_block);
        let watchers = join_all(tasks);
        watchers.await;
        true
    }
}

impl Future for MempoolManager {
    type Output = Result<(), std::io::Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Call an async function using the `async` keyword and `await` keyword
        let async_fn = async { self.clone().run().await };

        // Convert the async function to a future using `Box::pin`
        let mut future = Box::pin(async_fn);

        // Poll the future using `poll` on the returned `Pin` reference
        match future.as_mut().poll(cx) {
            Poll::Ready(result) =>{
                if result {
                    return Poll::Ready(Ok(()))
                }
                 Poll::Ready(Err(std::io::Error::new(ErrorKind::Other,"unexpected error running mempool manager")))
                },
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn handle_new_block(
    new_block_recv: Receiver<bot::Message<MempoolRaw>>,
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
    kill_new_block_watch: Arc<AtomicBool>,
) {
    loop {
        if kill_new_block_watch.load(Ordering::Relaxed) {
            break;
        }
        let new_block = match new_block_recv.recv() {
            Ok(block_data) => Some(block_data),
            Err(e) => {
                tracing::error!("error reading from new block recv channel: {:?}", e);
                None
            }
        };
        if new_block.is_none() {
            continue;
        }
        let last_block = new_block.clone();
        tracing::info!("a new block was found! {:?}", last_block.unwrap().val.blocks.last().unwrap());
        create_and_send_new_block(new_block.unwrap().val, send_to_alert_manager.clone()).await;
    }
}

async fn create_and_send_new_block(
    new_block: MempoolRaw,
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
) {
    let newest_block = new_block.blocks.last().unwrap();
    let fees = new_block.fees;
    let transaction_ids: Vec<TransactionID> = new_block
        .transactions
        .into_iter()
        .map(|transaction| TransactionID {
            tx_id: transaction.txid,
        })
        .collect();
    let mempool_data = MempoolData {
        fees: RecommendedFees { 
            fastest_fee: fees.fastest_fee, 
            half_hour_fee: fees.half_hour_fee, 
            hour_fee: fees.hour_fee, 
            economy_fee: fees.economy_fee, 
            minimum_fee: fees.minimum_fee 
        },
        transactions: transaction_ids,
        block: BlockTip { height: newest_block.height as u64, hash: newest_block.id.to_owned() }
    };
    let _ = send_current_state(send_to_alert_manager.clone(), mempool_data).await;
}

async fn build_mempool_state(
    base_url: String,
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
) {
    tracing::info!("pulling new block data from mempool.space");
    let current_block = current_block(base_url.clone()).await;
    let transactions = transactions(base_url.clone(), current_block.hash.clone()).await;
    let mempool_fees = mempool_recommended_fees(base_url.clone()).await;
    let mempool_data = MempoolData {
        fees: mempool_fees,
        transactions: transactions,
        block: current_block,
    };
    tracing::info!("sending new block data to alert manager");
    let _ = send_current_state(send_to_alert_manager.clone(), mempool_data).await;
}

async fn current_block(base_url: String) -> BlockTip {
    let client = reqwest::Client::new();
    let height_url = format!("{}/api/blocks/tip/height", base_url);
    let height_response = client
        .get(height_url)
        .send()
        .await
        .expect("failed to get tip height.");
    let raw_height = height_response.text().await.unwrap();
    let converted_height = raw_height.parse::<u64>().unwrap();

    let url = format!("{}/api/blocks/tip/hash", base_url);
    let hash_response = client
        .get(url)
        .send()
        .await
        .expect("failed to get tip hash.");
    let hash = hash_response.text().await.unwrap();

    BlockTip {
        hash: hash,
        height: converted_height,
    }
}

async fn transactions(base_url: String, block_hash: String) -> Vec<TransactionID> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/block/{block_hash}/txids", base_url);
    let response = client
        .get(url)
        .send()
        .await
        .expect("Failed to get transactions.");
    let transactions: Vec<TransactionID> = response
        .text()
        .await
        .iter()
        .map(|tx| TransactionID {
            tx_id: tx.to_owned(),
        })
        .collect();
    transactions
}

async fn mempool_recommended_fees(base_url: String) -> RecommendedFees {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/fees/recommended", base_url);
    let response = client
        .get(url)
        .send()
        .await
        .expect("Failed to get recommended fees.");
    let response_body = response.json::<RecommendedFees>().await.unwrap();
    response_body
}

async fn send_current_state(
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
    message: MempoolData,
) {
    send_to_alert_manager
        .send(bot::Message {
            val: message.into(),
        })
        .unwrap();
}
