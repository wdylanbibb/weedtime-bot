mod weedtime;

use std::{env, error::Error, fs, path::Path, sync::Arc};

use chrono_tz::Tz;
use serenity::{
    Client,
    all::{ChannelId, Context, CreateMessage, EventHandler, GatewayIntents, Message, UserId},
    async_trait,
    prelude::TypeMapKey,
};
use tracing::{error, warn};
use weedtime_db::data::v1::{
    DbUpdate, GuildStats, GuildStatsDatabase, UserStats, UserStatsDatabase,
};
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

type WeedTimeDatabases = (UserStatsDatabase<'static>, GuildStatsDatabase<'static>);

struct Handler {
    db: Arc<WeedTimeDatabases>,
}

fn open_or_create_user_db(
    path: impl AsRef<Path>,
) -> Result<UserStatsDatabase<'static>, Box<dyn Error>> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        Ok(UserStatsDatabase::open(path)?)
    } else {
        Ok(UserStatsDatabase::create(path)?)
    }
}

fn open_or_create_guild_db(
    path: impl AsRef<Path>,
) -> Result<GuildStatsDatabase<'static>, Box<dyn Error>> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        Ok(GuildStatsDatabase::open(path)?)
    } else {
        Ok(GuildStatsDatabase::create(path)?)
    }
}

fn open_or_create_databases() -> Result<WeedTimeDatabases, Box<dyn Error>> {
    let user_db_path =
        env::var("WEEDTIME_USER_DB_PATH").unwrap_or_else(|_| "data/user-stats.db".to_string());
    let guild_db_path =
        env::var("WEEDTIME_GUILD_DB_PATH").unwrap_or_else(|_| "data/guild-stats.db".to_string());

    Ok((
        open_or_create_user_db(user_db_path)?,
        open_or_create_guild_db(guild_db_path)?,
    ))
}

fn is_stats_command(content: &str) -> bool {
    matches!(content.trim(), "!weedstats" | "!weed stats")
}

fn format_user_stats(user_id: UserId, stats: Option<UserStats>) -> String {
    match stats {
        Some(stats) => format!(
            "<@{}>\nWeed times: {}\nWeed crimes: {}\nChains started: {}\nChains broken: {}",
            user_id.get(),
            stats.weed_times,
            stats.weed_crimes,
            stats.chains_started,
            stats.chains_broken
        ),
        None => format!("<@{}>\nNo weed stats yet.", user_id.get()),
    }
}

fn format_guild_stats(stats: Option<GuildStats>) -> String {
    match stats {
        Some(stats) => format!(
            "Server\nWeed times: {}\nWeed crimes: {}\nLongest chain: {}",
            stats.weed_times, stats.weed_crimes, stats.longest_chain
        ),
        None => "Server\nNo weed stats yet.".to_string(),
    }
}

async fn handle_stats_command(
    ctx: &Context,
    msg: &Message,
    db: &WeedTimeDatabases,
) -> Result<bool, serenity::Error> {
    if !is_stats_command(&msg.content.to_lowercase()) {
        return Ok(false);
    }

    let user_id = msg
        .mentions
        .first()
        .map(|user| user.id)
        .unwrap_or(msg.author.id);
    let user_stats = match db.0.get(user_id) {
        Ok(stats) => stats,
        Err(e) => {
            error!("Failed to fetch user stats for {user_id}: {e:?}");
            None
        }
    };

    let guild_stats = msg.guild_id.and_then(|guild_id| match db.1.get(guild_id) {
        Ok(stats) => stats,
        Err(e) => {
            error!("Failed to fetch guild stats for {guild_id}: {e:?}");
            None
        }
    });

    let mut response = format_user_stats(user_id, user_stats);
    if msg.guild_id.is_some() {
        response.push_str("\n\n");
        response.push_str(&format_guild_stats(guild_stats));
    }

    msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().content(response))
        .await?;

    Ok(true)
}

#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event. This is called whenever a new message is received.
    //
    // Event handlers are dispatched through a threadpool, and so multiple events can be dispatched
    // simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        let ctx = Arc::new(ctx);
        let msg = Arc::new(msg);
        let db = self.db.clone();

        tokio::spawn(async move {
            let timestamp = (*msg.timestamp).with_timezone::<Tz>(&Tz::America__New_York);

            let is_weed_time = is_420(timestamp);
            let contains_weed_time = msg.content.to_lowercase().contains("weed time");

            if msg.author.id == ctx.cache.current_user().id {
                return;
            }

            match handle_stats_command(&ctx, &msg, db.as_ref()).await {
                Ok(true) => return,
                Ok(false) => {}
                Err(e) => {
                    warn!("Stats command error: {e:?}");
                    return;
                }
            }

            let update = match (is_weed_time, contains_weed_time) {
                (false, true) => WeedCrime::update(&ctx, &msg).await,
                (true, false) => BrokenChain::update(&ctx, &msg).await,
                (true, true) => WeedTime::update(&ctx, &msg).await,
                _ => Ok(None),
            };

            match update {
                Ok(Some(update)) => {
                    if let Err(e) = update.commit(db.as_ref()) {
                        error!(
                            "Database commit error (is_weed_time: {is_weed_time}, contains_weed_time: {contains_weed_time}): {e:?}"
                        );
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    error!(
                        "Error (is_weed_time: {is_weed_time}, contains_weed_time: {contains_weed_time}): {e:?}"
                    );
                }
            };
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
    let db = Arc::new(open_or_create_databases().expect("Err creating databases"));
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will be automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { db })
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
