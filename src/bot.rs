
/*
TODO: here we need to handle the business logic of marshalling mempool data/messages into alerts that are need by the clients of this bot

 */

use sqlx::PgPool;
//mpsc::channel()
struct Bot {
    db: PgPool,
    pub mempool_client: MempoolClient,
    pub nostr_client: NostrClient
}

impl Bot {
    pub async fn run() {
        let (tx, rx) = mpsc::channel();
    // create the threads here, 1 for mempool, 1 for nostr
    // need to communicate between the two, or communicate one way and poll the DB for new alerts? 

    }
}