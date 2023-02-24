use nostr_sdk::{client, prelude::Metadata};



async fn post_nostr() {
    let client = client::new(&bot_keys);

    client.add_relay(&relay_url, None).await?;

    client.connect().await;

    let metadata = Metadata::new()
        .name("block bot")
        .display_name("block bot")
        .about("a block notification bot that will publish a notification to a user when a block target has been hit or a block number has been reached")
        //.nip05()
        //.lud16()
    client.set_metadata(metadata).await?;

    client.publis_
}