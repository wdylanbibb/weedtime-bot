mod commands;
mod db;

use std::{collections::HashSet, env, sync::Arc};

use bonsaidb::core::schema::SerializedCollection;
use chrono::{DateTime, FixedOffset, TimeZone, Timelike};
use dashmap::{try_result::TryResult, DashMap};
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::StandardFramework,
    http::Http,
    model::{
        gateway::Ready,
        prelude::{
            command::Command, interaction::Interaction, ChannelId, GuildId, Message, UserId,
        },
    },
    prelude::*,
};
use tracing::{error, info};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

#[derive(Debug)]
struct WeedTimeMessage {
    // The previous message that was sent in the channel
    msg: Message,
    // The users that have done a successful weed time message
    users: Vec<UserId>,
    // The number of successive weed time messages
    count: u32,
}

fn combo_to_emojis(combo: u32) -> String {
    // Get the amount of times a number can be divided by 10 without going under 10
    let count = std::iter::successors(Some(combo), |&n| (n >= 10).then(|| n / 10)).count();
    // Nums is an array of single digit numbers that make up the combo amount
    let nums = (0..count as u32)
        .into_iter()
        .map(|n| combo / 10_u32.pow(n) % 10)
        .collect::<Vec<_>>();
    // Match each number with the emoji ('x' if is none (somehow))
    let mut str = "".to_owned();
    for n in nums {
        str.push_str(match n {
            0 => "<:combo0:1083097710112022739>",
            1 => "<:combo1:1083097662624112753>",
            2 => "<:combo2:1083097661814620302>",
            3 => "<:combo3:1083097660669567037>",
            4 => "<:combo4:1083097659360944168>",
            5 => "<:combo5:1083097655854510161>",
            6 => "<:combo6:1083097654927564830>",
            7 => "<:combo7:1083097653363101697>",
            8 => "<:combo8:1083097652834615356>",
            9 => "<:combo9:1083097651232374784>",
            _ => "<:x_:1083098032268120075>",
        })
    }
    str.to_string()
}

struct MessageCount;

impl TypeMapKey for MessageCount {
    // While you will be using RwLock or Mutex most of the time you want to modify data,
    // sometimes it's not required; like for example, with static data, or if you are using other
    // kinds of atomic operators.
    //
    // Arc should stay, to allow for the data lock to be closed early.
    // type Value = Arc<AtomicUsize>;
    type Value = Arc<DashMap<ChannelId, WeedTimeMessage>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    // We use the cache_ready event just in case some cache operation is required in whatever use
    // case you have for this.
    async fn cache_ready(&self, _ctx: Context, _guilds: Vec<GuildId>) {
        info!("Cache built successfully!");
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let ctx = Arc::new(ctx);
        let msg = Arc::new(msg);

        tokio::spawn(async move {
            let db = db::open().await.expect("Error opening database");

            let offset = match FixedOffset::east_opt({
                match msg.guild_id {
                    Some(id) => match db::GuildStats::get_async::<_, u64>(id.into(), &db).await {
                        Ok(doc) => match doc {
                            Some(doc) => doc.contents.utc_offset,
                            None => 0,
                        },
                        Err(e) => {
                            error!("Error opening guild in database: {e:?}");
                            0
                        }
                    },
                    None => 0,
                }
            }) {
                Some(offset) => offset,
                None => FixedOffset::east_opt(0).unwrap(),
            };

            let timestamp: DateTime<FixedOffset> =
                msg.timestamp.with_timezone(&TimeZone::from_offset(&offset));

            let is_weed_time = timestamp.hour() % 12 == 4 && timestamp.minute() == 20;
            // let is_weed_time = true;
            let contains_weed_time = msg.content.to_lowercase().contains("weed time");

            // We are verifying if the bot id is the same as the message author id.
            // Also return if it's not 4:20 AND the message does not contain "weed time"
            if msg.author.id == ctx.cache.current_user_id()
                || (!is_weed_time && !contains_weed_time)
            {
                return;
            }

            let channel_id = match msg.channel(&ctx.http).await {
                Ok(channel) => channel.id(),
                Err(_) => return,
            };
            // Since data is located in Context, this means you are also able to use it within events!

            // let mut messages = raw_messages.write().await;

            let mut event = None;

            fn has_unique_elements<T>(iter: T) -> bool
            where
                T: IntoIterator,
                T::Item: Eq + std::hash::Hash,
            {
                let mut uniq = HashSet::new();
                iter.into_iter().all(move |x| uniq.insert(x))
            }

            loop {
                // If the time is not 4:20 and the user wrote "weed time", reply with WEED CRIME
                if !is_weed_time && contains_weed_time {
                    match channel_id
                        .send_files(&ctx.http, vec!["assets/420_jail.jpg"], |m| {
                            m.content("WEED CRIME!")
                        })
                        .await
                    {
                        Ok(_) => {
                            // WEED CRIME EVENT
                            if let Some(guild_id) = msg.guild_id {
                                event = Some(db::WeedTimeEvent::WeedCrime(db::EventData {
                                    guild_id,
                                    user_id: msg.author.id,
                                }));
                            }
                        }
                        Err(e) => error!("Error sending weed crime message: {e:?}"),
                    }
                    break;
                }

                // It can now be assumed that it is 4:20
                let messages = {
                    let data_read = ctx.data.read().await;
                    data_read
                        .get::<MessageCount>()
                        .expect("Expected MessageCount in TypeMap.")
                        .clone()
                };

                match messages.try_get_mut(&channel_id) {
                    TryResult::Present(mut weed_time_message) => {
                        // Channel is present in map
                        weed_time_message.users.push(msg.author.id);

                        let has_unique_users = has_unique_elements(weed_time_message.users.iter());
                        // let has_unique_users = true;

                        let weed_time_timestamp: DateTime<FixedOffset> =
                            msg.timestamp.with_timezone(&TimeZone::from_offset(&offset));

                        if timestamp.date_naive() == weed_time_timestamp.date_naive()
                            && timestamp.hour() == weed_time_timestamp.hour()
                            && contains_weed_time
                            && has_unique_users
                        {
                            // It is the same weed time as in the last message sent to the channel
                            // and the message contains weed time (doesn't break combo)
                            let count = weed_time_message.count;
                            let attachments = weed_time_message
                                .msg
                                .attachments
                                .iter()
                                .map(|a| a.id)
                                .collect::<Vec<_>>();
                            // If count is 0, the previous message was not "weed time"
                            if count != 0 {
                                if let Err(why) = weed_time_message
                                    .msg
                                    .edit(&ctx.http, |m| {
                                        combo_to_emojis(count);
                                        m.content(format!(
                                            "<:4_:1083068784404865136><:2_:1083068782764900412><:0_:1083068785436672010> <:x_:1083098032268120075>{}",
                                            combo_to_emojis(count)
                                        ));
                                        for attachment in attachments {
                                            m.remove_existing_attachment(attachment);
                                        }
                                        m
                                    })
                                    .await
                                {
                                    error!("Error editing message: {why:?}");
                                    break;
                                }
                            }
                        } else {
                            // User broke chain
                            weed_time_message.users = Vec::new();
                            weed_time_message.count = 0;
                            if !contains_weed_time {
                                // CHAIN BROKEN EVENT
                                if let Some(guild_id) = msg.guild_id {
                                    event = Some(db::WeedTimeEvent::ChainBroken(db::EventData {
                                        guild_id,
                                        user_id: msg.author.id,
                                    }));
                                }
                                break;
                            }
                        }

                        weed_time_message.count += 1;

                        match channel_id
                            .send_files(&ctx.http, vec!["assets/420.png"], |m| {
                                if weed_time_message.count > 1 {
                                    m.content(format!(
                                        "<:x_:1083098032268120075>{}",
                                        combo_to_emojis(weed_time_message.count)
                                    ));
                                }
                                m
                            })
                            .await
                        {
                            Ok(msg) => {
                                if weed_time_message.count > 1 {
                                    // WEED TIME EVENT
                                    if let Some(guild_id) = msg.guild_id {
                                        event = Some(db::WeedTimeEvent::WeedTime(
                                            weed_time_message.count,
                                            db::EventData {
                                                guild_id,
                                                user_id: msg.author.id,
                                            },
                                        ));
                                    }
                                } else {
                                    // NEW CHAIN EVENT
                                    if let Some(guild_id) = msg.guild_id {
                                        event = Some(db::WeedTimeEvent::NewChain(db::EventData {
                                            guild_id,
                                            user_id: msg.author.id,
                                        }));
                                    }
                                }
                                weed_time_message.msg = msg;
                            }
                            Err(e) => {
                                error!("Error sending message: {e:?}");
                                break;
                            }
                        }
                        break;
                    }
                    TryResult::Absent => {
                        // Channel has not sent a weed time message
                        if contains_weed_time {
                            messages.insert(
                                channel_id,
                                WeedTimeMessage {
                                    msg: match channel_id
                                        .send_files(&ctx.http, vec!["assets/420.png"], |m| m)
                                        .await
                                    {
                                        Ok(msg) => {
                                            // New weed time chain started
                                            // NEW CHAIN EVENT
                                            if let Some(guild_id) = msg.guild_id {
                                                event = Some(db::WeedTimeEvent::NewChain(
                                                    db::EventData {
                                                        guild_id,
                                                        user_id: msg.author.id,
                                                    },
                                                ));
                                            }
                                            msg
                                        }
                                        Err(e) => {
                                            error!("Error sending message: {e:?}");
                                            break;
                                        }
                                    },
                                    users: vec![msg.author.id],
                                    count: 1,
                                },
                            );
                        }
                        break;
                    }
                    TryResult::Locked => (), // Loop again and test if map is unlocked
                };
            }

            if let Some(event) = event {
                if let Err(e) = db::commit_event(event, &db).await {
                    error!("Error commiting event to database: {e:?}");
                }
            }
        });
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        if let Ok(guild_id) = env::var("GUILD_ID") {
            let guild_id = GuildId(guild_id.parse().expect("GUILD_ID must be an integer"));

            if let Err(e) = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
                commands.create_application_command(|command| commands::timezone::register(command))
            })
            .await
            {
                error!("Error creating slash commands: {e:?}");
            }
        }

        if let Err(e) = Command::create_global_application_command(&ctx.http, |command| {
            commands::serverstats::register(command)
        })
        .await
        {
            error!("Error creating slash command: {e:?}");
        }

        if let Err(e) = Command::create_global_application_command(&ctx.http, |command| {
            commands::userstats::register(command)
        })
        .await
        {
            error!("Error creating slash command: {e:?}");
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            if let Err(e) = match command.data.name.as_str() {
                "serverstats" => commands::serverstats::run(&ctx, &command).await,
                "userstats" => {
                    commands::userstats::run(&ctx, &command, &command.data.options).await
                }
                "timezone" => commands::timezone::run(&ctx, &command, &command.data.options).await,
                _ => Ok(()),
            } {
                error!("Error executing slash command: {e:?}");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // This will load the environment variables located at `./.env`, relative to
    // the CWD. See `./.env.example` for an example on how to structure this.
    dotenv::dotenv().expect("Failed to load .env file");

    // Initialize the logger to use environment variables.
    //
    // In this case, a good default is setting the environment variable
    // `RUST_LOG` to `debug`.
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new().configure(|c| c.owners(owners).prefix("~"));

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<MessageCount>(Arc::new(DashMap::new()));
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
