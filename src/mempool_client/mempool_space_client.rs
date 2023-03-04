/*
 * TODO:
 *
 * need methods to listen or poll for
 * 1) utxo movement
 * 2) transaction confirmation height
 * 3) mempool fee hitting a given threshold
 * 4) a given block height has been reached
 */

use std::{sync::mpsc::Receiver, future::{Ready, IntoFuture, ready}};

use reqwest::{Client, Url};
use sqlx::PgPool;
use tokio_tungstenite::{tungstenite::{connect, WebSocket}, MaybeTlsStream};
use tokio::sync::mpsc::Sender;
use crate::bot::{Message, Channels};

pub struct MempoolClient {
    mempool_space: String,
    http_client: Client,
    websocket_clients: Vec<MempoolNetworkWS>,
    db_pool: PgPool,
    nostr_comm: Channels, 
}

pub struct MempoolNetworkWS {
    base_url: String,
    network_type: String
}

impl MempoolClient {
    pub async fn build(mempool_url: &str, db: PgPool, nostr_comm: Channels) -> MempoolClient {
        let client = reqwest::Client::new();
        let mempool_network_ws = MempoolNetworkWS{
            base_url: "wss://mempool.space/api/v1/ws".to_owned(),
            network_type: "mainnet".to_owned(),
        };

        let mut websocket_clients = Vec::new();
        websocket_clients.push(mempool_network_ws);

        Self {
            mempool_space: mempool_url.to_owned(),
            websocket_clients: websocket_clients,
            http_client: client,
            db_pool: db,
            nostr_comm: nostr_comm,
        }
    }

    // each function should handle their own websocket connection
    async fn utxo_movements(self) {}

    async fn transaction_height(self) {}

    async fn mempool_fee(self) {}

    async fn block_height(self) {}
}


impl IntoFuture for MempoolClient {
    type Output = Result<(), std::io::Error>;
    type IntoFuture = Ready<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        //TODO: call the forever loop here to continue listen to the collection of websockets
        ready(Ok(()))
    }
}