mod weedtime;

use std::{env, error::Error, fs, path::Path, sync::Arc};

use chrono_tz::Tz;
use serenity::{
    Client,
    all::{
        ChannelId, Command, CommandInteraction, CommandOptionType, Context, EventHandler,
        GatewayIntents, Interaction, Message, Ready, ResolvedValue, User, UserId,
    },
    async_trait,
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateEmbedAuthor,
        CreateInteractionResponse, CreateInteractionResponseMessage,
    },
    model::colour::Colour,
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

fn stats_commands() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("userstats")
            .description("Show weed stats for a user")
            .add_option(CreateCommandOption::new(
                CommandOptionType::User,
                "user",
                "The user to show stats for",
            )),
        CreateCommand::new("serverstats").description("Show weed stats for this server"),
    ]
}

fn user_stats_embed(user: &User, stats: Option<UserStats>) -> CreateEmbed {
    let embed = CreateEmbed::new()
        .title(format!("{}'s Weed Stats", user.name))
        .author(CreateEmbedAuthor::new(user.name.clone()).icon_url(user.face()))
        .thumbnail(user.face())
        .colour(Colour::DARK_GREEN);

    if let Some(stats) = stats {
        embed
            .field("Weed times", stats.weed_times.to_string(), true)
            .field("Weed crimes", stats.weed_crimes.to_string(), true)
            .field("Chains started", stats.chains_started.to_string(), true)
            .field("Chains broken", stats.chains_broken.to_string(), true)
    } else {
        embed.description("No weed stats yet.")
    }
}

fn guild_stats_embed(
    name: String,
    icon_url: Option<String>,
    stats: Option<GuildStats>,
) -> CreateEmbed {
    let mut author = CreateEmbedAuthor::new(name.clone());
    if let Some(icon_url) = icon_url.clone() {
        author = author.icon_url(icon_url);
    }

    let mut embed = CreateEmbed::new()
        .title(format!("{name} Weed Stats"))
        .author(author)
        .colour(Colour::DARK_GREEN);

    if let Some(icon_url) = icon_url {
        embed = embed.thumbnail(icon_url);
    }

    if let Some(stats) = stats {
        embed
            .field("Weed times", stats.weed_times.to_string(), true)
            .field("Weed crimes", stats.weed_crimes.to_string(), true)
            .field("Longest chain", stats.longest_chain.to_string(), true)
    } else {
        embed.description("No weed stats yet.")
    }
}

async fn respond_with_embed(
    ctx: &Context,
    command: &CommandInteraction,
    embed: CreateEmbed,
) -> Result<(), serenity::Error> {
    command
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed),
            ),
        )
        .await
}

async fn respond_with_content(
    ctx: &Context,
    command: &CommandInteraction,
    content: impl Into<String>,
) -> Result<(), serenity::Error> {
    command
        .create_response(
            ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .ephemeral(true),
            ),
        )
        .await
}

async fn handle_user_stats_command(
    ctx: &Context,
    command: &CommandInteraction,
    db: &WeedTimeDatabases,
) -> Result<(), serenity::Error> {
    let target = command
        .data
        .options()
        .into_iter()
        .find_map(|option| match option.value {
            ResolvedValue::User(user, _) if option.name == "user" => Some(user.clone()),
            _ => None,
        })
        .unwrap_or_else(|| command.user.clone());

    let stats = match db.0.get(target.id) {
        Ok(stats) => stats,
        Err(e) => {
            error!("Failed to fetch user stats for {}: {e:?}", target.id);
            None
        }
    };

    respond_with_embed(ctx, command, user_stats_embed(&target, stats)).await
}

async fn handle_guild_stats_command(
    ctx: &Context,
    command: &CommandInteraction,
    db: &WeedTimeDatabases,
) -> Result<(), serenity::Error> {
    let Some(guild_id) = command.guild_id else {
        return respond_with_content(ctx, command, "Server stats are only available in a server.")
            .await;
    };

    let guild = guild_id.to_partial_guild(ctx).await?;
    let stats = match db.1.get(guild_id) {
        Ok(stats) => stats,
        Err(e) => {
            error!("Failed to fetch guild stats for {guild_id}: {e:?}");
            None
        }
    };

    let icon_url = guild.icon_url();
    respond_with_embed(ctx, command, guild_stats_embed(guild.name, icon_url, stats)).await
}

async fn handle_stats_interaction(
    ctx: &Context,
    command: &CommandInteraction,
    db: &WeedTimeDatabases,
) -> Result<(), serenity::Error> {
    match command.data.name.as_str() {
        "userstats" => handle_user_stats_command(ctx, command, db).await,
        "serverstats" => handle_guild_stats_command(ctx, command, db).await,
        _ => Ok(()),
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        match Command::set_global_commands(&ctx.http, stats_commands()).await {
            Ok(commands) => {
                tracing::info!(
                    "Registered {} slash commands for {}",
                    commands.len(),
                    ready.user.name
                );
            }
            Err(e) => error!("Failed to register slash commands: {e:?}"),
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Interaction::Command(command) = interaction else {
            return;
        };

        if let Err(e) = handle_stats_interaction(&ctx, &command, self.db.as_ref()).await {
            warn!("Stats interaction error: {e:?}");
        }
    }

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
