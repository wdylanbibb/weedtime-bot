mod weedtime;

use std::{env, sync::Arc};

use chrono_tz::Tz;
use serenity::{
    Client,
    all::{ChannelId, Context, EventHandler, GatewayIntents, Message, UserId},
    async_trait,
    prelude::TypeMapKey,
};
use tracing::error;
use whirlwind::ShardMap;

use crate::weedtime::{
    states::{BrokenChain, MapUpdate, WeedCrime, WeedTime},
    util::is_420,
};

#[derive(Debug)]
struct WeedTimeMessage {
    msg: Option<Message>,
    users: Vec<UserId>,
    count: u32,
}

struct MessageCount;

impl TypeMapKey for MessageCount {
    type Value = Arc<ShardMap<ChannelId, WeedTimeMessage>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event. This is called whenever a new message is received.
    //
    // Event handlers are dispatched through a threadpool, and so multiple events can be dispatched
    // simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        let ctx = Arc::new(ctx);
        let msg = Arc::new(msg);

        tokio::spawn(async move {
            let timestamp = (*msg.timestamp).with_timezone::<Tz>(&Tz::America__New_York);

            // let is_weed_time = is_420(timestamp);
            let is_weed_time = true;
            let contains_weed_time = msg.content.to_lowercase().contains("weed time");

            if msg.author.id == ctx.cache.current_user().id {
                return;
            }

            if let Err(e) = match (is_weed_time, contains_weed_time) {
                (false, true) => WeedCrime::update(&ctx, &msg).await,
                (true, false) => BrokenChain::update(&ctx, &msg).await,
                (true, true) => WeedTime::update(&ctx, &msg).await,
                _ => Ok(None),
            } {
                error!(
                    "Error (is_weed_time: {is_weed_time}, contains_weed_time: {contains_weed_time}): {e:?}"
                );
            }
        });
    }
}

#[tokio::main]
async fn main() {
    // Initialize the logger to use environment variables.
    //
    // In this case, a good default is setting the environment variable `RUST_LOG` to `debug`
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will be automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<MessageCount>(Arc::new(ShardMap::new()));
    }

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will preform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
