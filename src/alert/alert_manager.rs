use super::{Alert, AlertKind, AlertKindHandler, AlertUpdate, RequestedAlert};
use crate::{
    bot::{Channels, Message},
    mempool::MempoolData,
};
use anyhow::{Context as AnyhowContext, Result};
use futures_util::Future;
use sqlx::PgPool;
use std::{
    io::ErrorKind,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};
use tracing::instrument;

//TODO: remove the need for the map_err by creating custom error types and conversion there

#[derive(Clone)]
pub struct AlertCommunication {
    pub nostr_com: Channels<String>,
    pub mempool_com: Channels<MempoolData>,
}
#[derive(Clone)]
pub struct AlertManager {
    pub db_pool: PgPool,
    pub communication: AlertCommunication,
    kill_signal: Arc<AtomicBool>,
}
impl AlertManager {
    #[instrument(skip_all)]
    pub async fn build(
        db: PgPool,
        communication: AlertCommunication,
        kill_signal: Arc<AtomicBool>,
    ) -> Result<AlertManager, anyhow::Error> {
        Ok(Self {
            db_pool: db,
            communication,
            kill_signal,
        })
    }

    /* NOTE: Items the manager needs to handle:
    1) listen to the nostr channel to see if a user as asked for a new activity to be tracked or asked for one to stop being tracked
    2) listen to the mempool channel for new blocks, update all the exisiting activity being tracked & send a message over the nostr channel for items have have hit a thresholdddd
    */
    #[instrument(skip_all)]
    pub async fn run(self) -> Result<(), std::io::Error> {
        let kill_signal = self.kill_signal.clone();
        let communication = self.communication.clone();
        let db_mempool = self.db_pool.clone();
        tokio::spawn(async move {
            listen_to_mempool(
                db_mempool.clone(),
                communication.clone(),
                kill_signal.clone(),
            )
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        });

        let kill_signal = self.kill_signal.clone();
        let communication = self.communication.clone();
        let db_nostr = self.db_pool.clone();
        tokio::spawn(async move {
            handle_nostr(db_nostr.clone(), communication.clone(), kill_signal)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        });

        Ok(())
    }
}

impl Future for AlertManager {
    type Output = Result<(), std::io::Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let async_fn = async { self.clone().run().await };
        let mut future = Box::pin(async_fn);

        match future.as_mut().poll(cx) {
            Poll::Ready(res) => match res {
                Ok(_) => Poll::Ready(Ok(())),
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

#[instrument(skip_all)]
async fn listen_to_mempool(
    db: PgPool,
    communication: AlertCommunication,
    kill_mempool_watch: Arc<AtomicBool>,
) -> Result<(), anyhow::Error> {
    let mempool_com = communication.mempool_com.clone();
    let nostr_com = communication.nostr_com.clone();

    loop {
        if kill_mempool_watch.load(Ordering::Relaxed) {
            break;
        }
        let new_block = mempool_com
            .listen
            .recv()
            .context("error reading from mempool recv channel")?;

        process_block_data(db.clone(), nostr_com.clone(), new_block.val.to_owned()).await?;
    }
    Ok(())
}

#[instrument(skip_all)]
async fn process_block_data(
    db: PgPool,
    nostr_com: Channels<String>,
    new_block: MempoolData,
) -> Result<(), anyhow::Error> {
    let active_alerts = get_active_alerts(db.clone())
        .await
        .context("error getting active alerts")?;

    let alerts_to_update = active_alerts
        .iter()
        .filter_map(|alert| handle_alert(alert.clone(), new_block.clone()))
        .collect::<Vec<Alert>>();

    batch_update_alerts(db.clone(), alerts_to_update.clone()).await?;

    alerts_to_update
        .iter()
        .filter(|alert| alert.should_send)
        .try_for_each(|alert| send_alert_to_nostr(alert, nostr_com.clone()))?;

    Ok(())
}

#[instrument]
fn send_alert_to_nostr(alert: &Alert, nostr_com: Channels<String>) -> Result<(), anyhow::Error> {
    let alert_string = alert.to_string().map_err(anyhow::Error::new)?;

    nostr_com
        .send
        .send(Message::from(alert_string))
        .map_err(anyhow::Error::new)?;
    Ok(())
}

#[instrument]
fn handle_alert(alert: Alert, new_block: MempoolData) -> Option<Alert> {
    match alert.kind {
        AlertKind::ConfirmHeight => alert
            .kind
            .update_confirm_height_alert(alert.clone(), new_block),
        AlertKind::FeeLevel => alert.kind.update_fee_level_alert(alert.clone(), new_block),
        AlertKind::BlockHeight => alert
            .kind
            .update_block_height_alert(alert.clone(), new_block),
    }
}
#[instrument(skip_all)]
async fn handle_nostr(
    db: PgPool,
    communication: AlertCommunication,
    kill_mempool_watch: Arc<AtomicBool>,
) -> Result<(), anyhow::Error> {
    let nostr_com = communication.nostr_com.clone();
    loop {
        if kill_mempool_watch.load(Ordering::Relaxed) {
            break;
        }
        let new_alert_request = nostr_com.listen.recv().map_err(anyhow::Error::new)?;
        create_alert_monitor(db.clone(), new_alert_request.val.into()).await?;
    }
    Ok(())
}

#[instrument(skip(db_pool))]
async fn get_active_alerts(db_pool: PgPool) -> Result<Vec<Alert>, anyhow::Error> {
    sqlx::query!(
        r#"
        SELECT 
            id, 
            alert_type_id, 
            active,
            requestor_pk, 
            threshold_num, 
            event_data_identifier,
            block_state
        FROM alerts
        WHERE active;
        "#,
    )
    .fetch_all(&db_pool)
    .await
    .map_err(anyhow::Error::new)
    .map(|rows| {
        rows.iter()
            .map(|row| {
                let mut threshold_convt: Option<u64> = None;
                if row.threshold_num.is_some() {
                    threshold_convt = Some(row.threshold_num.unwrap() as u64);
                }
                Alert {
                    id: row.id,
                    kind: AlertKind::from(row.alert_type_id),
                    should_send: false,
                    active: row.active,
                    requestor_pk: row.requestor_pk.to_owned(),
                    threshold_num: threshold_convt,
                    event_data_identifier: row.event_data_identifier.to_owned(),
                    block_state: None,
                }
            })
            .collect::<Vec<Alert>>()
    })
}

#[instrument(skip(db_pool))]
async fn create_alert_monitor(
    db_pool: PgPool,
    requested_alert: RequestedAlert,
) -> Result<Alert, anyhow::Error> {
    let mut threshold_convt: Option<i64> = None;
    if requested_alert.threshold_num.is_some() {
        threshold_convt = Some(requested_alert.threshold_num.unwrap() as i64);
    }
    let record = sqlx::query!(
        r#"
        INSERT INTO alerts (alert_type_id, requestor_pk, threshold_num, event_data_identifier)
        VALUES ($1, $2, $3, $4)
        RETURNING id, created_at
        "#,
        requested_alert.kind.to_int(),
        requested_alert.requestor_pk,
        threshold_convt,
        requested_alert.event_data_identifier
    )
    .fetch_one(&db_pool)
    .await
    .map_err(anyhow::Error::new)?;

    let active_alert = Alert {
        id: record.id,
        kind: requested_alert.kind,
        should_send: false,
        active: true,
        requestor_pk: requested_alert.requestor_pk.to_owned(),
        threshold_num: requested_alert.threshold_num,
        event_data_identifier: requested_alert.event_data_identifier.to_owned(),
        block_state: None,
    };
    Ok(active_alert)
}

#[instrument(skip_all)]
async fn batch_update_alerts(
    db_pool: PgPool,
    alerts_to_update: Vec<Alert>,
) -> Result<(), anyhow::Error> {
    let update_items: Vec<AlertUpdate> =
        alerts_to_update.iter().map(|alert| alert.into()).collect();
    let query = format!(
        "UPDATE alerts 
        SET 
            block_state = t.block_state,
            active = t.active
        FROM (VALUES {}) AS t(id, block_state, active)
        WHERE alerts.id = t.id;",
        update_items
            .iter()
            .map(|_| "(?, ?, ?)")
            .collect::<Vec<_>>()
            .join(",")
    );

    sqlx::query(&query)
        .execute(&db_pool)
        .await
        .map_err(anyhow::Error::new)?;

    Ok(())
}
