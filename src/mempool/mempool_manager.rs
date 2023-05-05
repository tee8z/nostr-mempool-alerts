use super::{BlockTip, MempoolData, RecommendedFees, TransactionID};
use crate::{
    bot::{self, Channels},
    mempool::MempoolRaw,
};
use anyhow::Context as AnyhowContext;
use crossbeam_channel::{Receiver, Sender};
use futures_util::{future::join_all, FutureExt, SinkExt, StreamExt};
use nostr_sdk::nostr::url;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    io::ErrorKind,
    str,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
    time::Duration,
    vec,
};
use tokio_tungstenite::{connect_async, tungstenite};
use tracing::instrument;

#[derive(Clone, Debug)]
pub struct MempoolManager {
    pub mempool_space: String,
    connect_addr: String,
    kill_signal: Arc<AtomicBool>,
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
}
#[derive(Debug)]
pub struct MempoolNetworkWS {}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MempoolMessage {
    pub action: String,
    pub data: Option<Vec<String>>,
}

impl MempoolMessage {
    pub fn to_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

//TODO: make a custom error type that resolves to an anyhow error so the map_err() calls can be removed

impl MempoolNetworkWS {
    //NOTE: listen to mempool websocket to be notified new block was found
    #[instrument(skip_all)]
    async fn listen_for_new_block(
        self,
        connect_addr: String,
        new_block_comm: Sender<bot::Message<MempoolRaw>>,
        kill_signal: Arc<AtomicBool>,
    ) -> Result<(), anyhow::Error> {
        let url = url::Url::parse(connect_addr.as_str()).map_err(anyhow::Error::new)?;
        let (ws_stream, _) = connect_async(url.clone())
            .await
            .map_err(anyhow::Error::msg)?;

        let (mut write, mut read) = ws_stream.split();
        let init_message = &MempoolMessage {
            action: "init".to_owned(),
            data: None,
        }
        .to_string()
        .map_err(anyhow::Error::msg)?;

        let set_want = &MempoolMessage {
            action: "want".to_owned(),
            data: Some(vec!["blocks".to_owned()]),
        }
        .to_string()
        .map_err(anyhow::Error::msg)?;
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                init_message.clone().to_owned(),
            ))
            .await
            .map_err(anyhow::Error::msg)?;
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                set_want.clone().to_owned(),
            ))
            .await
            .map_err(anyhow::Error::msg)?;

        let sleep_time = Duration::from_millis(20000);
        let mut interval = tokio::time::interval(sleep_time);
        loop {
            tokio::select! {
                msg = read.next() => {
                    if kill_signal.clone().load(Ordering::Relaxed) {
                        break;
                    }
                    match msg {
                        Some(msg) => {
                            let msg = msg?;
                            if msg.is_text() {
                                let mempool_data: MempoolRaw = MempoolRaw::from(msg);
                                tracing::info!(
                                    "new block was found from the mempool! {:?}",
                                    url.clone()
                                );
                                new_block_comm
                                    .send(bot::Message { val: mempool_data })
                                    .unwrap_or_else(|e| {
                                        tracing::error!("error sending new mempool data for a block: {:?}", e)
                                    });
                                tracing::info!("starting waiting for next loop");
                            } else if msg.is_close() {
                                tracing::info!("stream to {:?} closed", url.clone());
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = interval.tick() => {
                    if kill_signal.clone().load(Ordering::Relaxed) {
                        tracing::info!("stopping pinging {:?}", url.clone());
                        break;
                    }
                    tracing::info!("next pinging loop");
                    let ping = tungstenite::protocol::Message::Ping(vec![0; 124]);
                    write
                        .send(ping)
                        .await
                        .map_err(anyhow::Error::msg)?;
                    tracing::info!("successfully pinged, waiting for: {:?}", sleep_time);
                }
            }
        }

        Ok(())
    }
}

impl MempoolManager {
    #[instrument(skip(alert_manager, kill_signal))]
    pub fn build(
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
        let websocket = format!("wss://{}", mempool_url);
        let connect_addr = format!("{}/{}", websocket, api_endpoint.to_owned());
        Self {
            mempool_space: format!("https://{}", mempool_url),
            send_to_alert_manager: alert_manager.send,
            connect_addr,
            kill_signal,
        }
    }

    #[instrument(skip(self))]
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
        let watch_for_new_block: std::pin::Pin<
            Box<dyn Future<Output = Result<(), tokio::task::JoinError>> + Send>,
        > = tokio::spawn(async move {
            if kill_mempool_watching_new_block.load(Ordering::Relaxed) {
                tracing::info!("stopping watching the mempool");
                return;
            }
            tracing::info!("starting mempool.space websocketlistener");
            mempool_ws
                .listen_for_new_block(
                    connect_addr,
                    send_new_block,
                    kill_mempool_watching_new_block.clone(),
                )
                .await
                .unwrap_or_else(|e| tracing::error!("error listening for a new block: {:?}", e));
            tracing::info!("shutting down mempool.space websocketlistener");
        })
        .boxed();
        let watch_current_state = tokio::spawn(async move {
            tracing::info!("getting initial state");
            build_mempool_state(base_url.clone(), send_to_alert.clone())
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("error trying to build the mempool state: {:?}", e)
                });
            if kill_handle_new_block.load(Ordering::Relaxed) {
                tracing::info!("stopping listening for new block state");
                return;
            }
            tracing::info!("starting new_block channel listener and current state emitter");
            handle_new_block(
                listen_for_new_block,
                send_to_alert.clone(),
                kill_handle_new_block.clone(),
            )
            .await
            .unwrap_or_else(|e| tracing::error!("error trying to handle new block: {:?}", e));
            tracing::info!(
                "shutting down starting new_block channel listener and current state emitter"
            );
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
        let async_fn = async { self.clone().run().await };
        let mut future = Box::pin(async_fn);

        match future.as_mut().poll(cx) {
            Poll::Ready(result) => {
                if result {
                    return Poll::Ready(Ok(()));
                }
                Poll::Ready(Err(std::io::Error::new(
                    ErrorKind::Other,
                    "unexpected error running mempool manager",
                )))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[instrument(skip_all)]
async fn handle_new_block(
    new_block_recv: Receiver<bot::Message<MempoolRaw>>,
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
    kill_new_block_watch: Arc<AtomicBool>,
) -> Result<(), anyhow::Error> {
    loop {
        if kill_new_block_watch.load(Ordering::Relaxed) {
            break;
        }
        let new_block = new_block_recv.recv().map_err(anyhow::Error::new)?;

        let last_block = new_block.clone();
        if last_block.val.blocks.is_some() {
            tracing::info!(
                "a new block was found! {:?}",
                last_block
                    .val
                    .blocks
                    .unwrap()
                    .last()
                    .context("error getting last block from data")
            );
        } else {
            tracing::info!(
                "a new block was found! {:?}",
                last_block
                    .val
                    .block
                    .context("error getting last block from data")
            );
        }

        create_and_send_new_block(new_block.val, send_to_alert_manager.clone())
            .await
            .unwrap_or_else(|e| tracing::error!("error creating and sending a new block: {:?}", e));
    }
    Ok(())
}

#[instrument(skip_all)]
#[allow(unused_assignments)]
async fn create_and_send_new_block(
    new_block: MempoolRaw,
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
) -> Result<(), anyhow::Error> {
    let mut newest_block = None;
    if new_block.blocks.is_some() {
        newest_block = Some(new_block.blocks.unwrap().last().unwrap().to_owned());
    } else {
        newest_block = Some(new_block.block.unwrap());
    }
    let fees = new_block.fees;
    let mut transaction_ids = None;
    if new_block.transactions.is_some() {
        transaction_ids = Some(
            new_block
                .transactions
                .unwrap()
                .into_iter()
                .map(|transaction| TransactionID {
                    tx_id: transaction.txid,
                })
                .collect(),
        );
    }
    let mempool_data = MempoolData {
        fees: Some(RecommendedFees {
            fastest_fee: fees.fastest_fee,
            half_hour_fee: fees.half_hour_fee,
            hour_fee: fees.hour_fee,
            economy_fee: fees.economy_fee,
            minimum_fee: fees.minimum_fee,
        }),
        transactions: transaction_ids,
        block: BlockTip {
            height: newest_block.clone().unwrap().height as u64,
            hash: newest_block.unwrap().id,
        },
    };
    tracing::info!("current block height: {:?}", mempool_data.block.height);
    send_current_state(send_to_alert_manager.clone(), mempool_data).await
}

#[instrument(skip(send_to_alert_manager))]
async fn build_mempool_state(
    base_url: String,
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
) -> Result<(), anyhow::Error> {
    tracing::info!("pulling new block data from mempool.space");
    let current_block = current_block(base_url.clone()).await?;
    tracing::info!("block_tip: {:?}", current_block);

    let transactions = transactions(base_url.clone(), current_block.hash.clone()).await?;
    tracing::info!("transactions: {:?}", transactions);

    let mempool_fees = mempool_recommended_fees(base_url.clone()).await?;
    tracing::info!("mempool_fees: {:?}", mempool_fees);

    let mempool_data = MempoolData {
        fees: mempool_fees,
        transactions: Some(transactions),
        block: current_block,
    };
    tracing::info!("sending new block data to alert manager {:?}", mempool_data);
    send_current_state(send_to_alert_manager.clone(), mempool_data).await
}

//TODO: better error handling here, don't want the process to die due to mepool.space being down, should just zombiefy
#[instrument]
async fn current_block(base_url: String) -> Result<BlockTip, anyhow::Error> {
    let client = reqwest::Client::new();
    let height_url = format!("{}/api/blocks/tip/height", base_url);
    let height_response = client.get(height_url).send().await?;
    if !height_response.status().is_success() {
        tracing::error!("error getting block height: {:?}", height_response);
        return Err(anyhow::Error::msg("error getting block height"));
    }

    let raw_height = height_response.text().await.map_err(anyhow::Error::new)?;
    let converted_height = raw_height.parse::<u64>().map_err(anyhow::Error::new)?;

    let url = format!("{}/api/blocks/tip/hash", base_url);
    let hash_response = client.get(url).send().await?;

    if !hash_response.status().is_success() {
        tracing::error!("error getting block hash: {:?}", hash_response);
        return Err(anyhow::Error::msg("error getting block hash"));
    }

    let hash = hash_response.text().await.map_err(anyhow::Error::new)?;

    Ok(BlockTip {
        hash,
        height: converted_height,
    })
}

#[instrument]
async fn transactions(
    base_url: String,
    block_hash: String,
) -> Result<Vec<TransactionID>, anyhow::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/block/{block_hash}/txids", base_url);
    let response = client.get(url).send().await.map_err(anyhow::Error::new)?;

    if !response.status().is_success() {
        tracing::error!("error getting transaction: {:?}", response);
        return Err(anyhow::Error::msg("error getting transaction"));
    }
    let transactions: Vec<TransactionID> = response
        .text()
        .await
        .iter()
        .map(|tx| TransactionID {
            tx_id: tx.to_owned(),
        })
        .collect();
    Ok(transactions)
}

#[instrument]
async fn mempool_recommended_fees(
    base_url: String,
) -> Result<Option<RecommendedFees>, anyhow::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/fees/recommended", base_url);
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        tracing::error!("error getting recommended fees: {:?}", response);
        return Err(anyhow::Error::msg("error getting recommended fees"));
    }

    tracing::info!("recommended fees: {:?}", response);
    let res = response
        .json::<RecommendedFees>()
        .await
        .map_err(anyhow::Error::new)?;
    Ok(Some(res))
}

#[instrument(skip(send_to_alert_manager))]
async fn send_current_state(
    send_to_alert_manager: Sender<bot::Message<MempoolData>>,
    message: MempoolData,
) -> Result<(), anyhow::Error> {
    send_to_alert_manager
        .send(bot::Message { val: message })
        .map_err(anyhow::Error::new)
}
