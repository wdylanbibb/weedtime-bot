use chrono::Timelike;
use chrono_tz::Tz;
use serenity::all::{Context, CreateAttachment, CreateMessage, EditMessage, Message, UserId};
use weedtime_db::data::v1::{GuildStatsUpdate, UserStatsUpdate};

use crate::{
    WeedTimeMessage,
    weedtime::util::{combo_to_emojis, get_map, has_unique_elements},
};

pub trait MapUpdate {
    async fn update(
        ctx: &Context,
        msg: &Message,
    ) -> Result<Option<(UserStatsUpdate, GuildStatsUpdate)>, serenity::Error>;
}

pub struct WeedTime;

impl MapUpdate for WeedTime {
    async fn update(
        ctx: &Context,
        msg: &Message,
    ) -> Result<Option<(UserStatsUpdate, GuildStatsUpdate)>, serenity::Error> {
        let map = get_map(ctx).await.clone();
        let channel_id = msg.channel(&ctx.http).await?.id();
        let new_msg = channel_id
            .send_files(
                &ctx.http,
                vec![CreateAttachment::path("assets/420.png").await?],
                CreateMessage::new().content("WEED TIME!"),
            )
            .await?;

        enum WeedTimeState {
            Edit {
                msg: Message,
                count: u32,
            },
            Insert {
                msg: Message,
                users: Vec<UserId>,
                count: u32,
            },
        }

        let mut state: Option<WeedTimeState> = None;

        match map.get_mut(&channel_id).await {
            Some(mut weed_time_message) => {
                weed_time_message.users.push(msg.author.id);

                // let has_unique_users = has_unique_elements(weed_time_message.users.iter());
                let has_unique_users = true;

                if weed_time_message.msg.clone().is_some_and(|m| {
                    let weed_time_timestamp = m.timestamp.with_timezone(&Tz::America__New_York);
                    let timestamp = msg.timestamp.with_timezone(&Tz::America__New_York);

                    timestamp.date_naive() == weed_time_timestamp.date_naive()
                        && timestamp.hour() == weed_time_timestamp.hour()
                        && has_unique_users
                }) {
                    state = Some(WeedTimeState::Edit {
                        msg: weed_time_message.msg.clone().unwrap(),
                        count: weed_time_message.count,
                    });

                    weed_time_message.msg = Some(new_msg);
                    weed_time_message.count += 1;

                    tracing::info!(
                        "Weed time chain continuing (Count: {})",
                        weed_time_message.count
                    );

                    // DB Updates:
                    //   UserStats:
                    //     Weed Times + 1
                    //   GuildStats:
                    //     Weed Times + 1
                    //     Longest Chain Check
                } else {
                    // Chain broken or new weed time
                    weed_time_message.msg = Some(new_msg);
                    weed_time_message.users = vec![msg.author.id];
                    weed_time_message.count = 1;

                    tracing::info!(
                        "Non-unique user or new weed time. Restarting channel entry here."
                    );

                    // DB Updates:
                    //   UserStats:
                    //     Weed Times + 1
                    //     Chains Started + 1
                    //     if !has_unique_users:
                    //       Chains Broken + 1
                    //   GuildStats:
                    //     Weed Times + 1
                    //     Longest Chain Check
                }
            }
            None => {
                state = Some(WeedTimeState::Insert {
                    msg: new_msg,
                    users: vec![msg.author.id],
                    count: 1,
                });
                tracing::info!("Inserting channel entry.");

                // DB Updates:
                //   UserStats:
                //     Weed Times + 1
                //     Chains Started + 1
                //   GuildStats:
                //     Weed Times + 1
                //     Longest Chain Check
            }
        }

        // This is done because `weed_time_message` needs to be dropped before an `await` is used
        if let Some(state) = state {
            match state {
                WeedTimeState::Edit { mut msg, count } => msg.edit(&ctx.http, EditMessage::new().content(format!(
                    "<:4_:1083068784404865136><:2_:1083068782764900412><:0_:1083068785436672010> <:x_:1083098032268120075>{}",
                    combo_to_emojis(count)
                )).remove_all_attachments()).await?,
                WeedTimeState::Insert { msg, users, count } => {
                    map.insert(channel_id, WeedTimeMessage {
                        msg: Some(msg),
                        users,
                        count
                    }).await;
                }
            }
        }

        Ok(Some((UserStatsUpdate, GuildStatsUpdate)))
    }
}

pub struct WeedCrime;

impl MapUpdate for WeedCrime {
    async fn update(
        ctx: &Context,
        msg: &Message,
    ) -> Result<Option<(UserStatsUpdate, GuildStatsUpdate)>, serenity::Error> {
        let channel_id = msg.channel(&ctx.http).await?.id();

        channel_id
            .send_files(
                &ctx.http,
                vec![CreateAttachment::path("assets/420_jail.jpg").await?],
                CreateMessage::new().content("WEED CRIME!"),
            )
            .await?;

        // DB Updates:
        //   UserStats:
        //     Weed Crimes + 1
        //   GuildStats:
        //     Weed Crimes + 1

        Ok(Some((UserStatsUpdate, GuildStatsUpdate)))
    }
}

pub struct BrokenChain;

impl MapUpdate for BrokenChain {
    async fn update(
        ctx: &Context,
        msg: &Message,
    ) -> Result<Option<(UserStatsUpdate, GuildStatsUpdate)>, serenity::Error> {
        let map = get_map(ctx).await;

        if let Some(mut weed_time_message) = map.get_mut(&msg.channel(&ctx.http).await?.id()).await
        {
            weed_time_message.msg = None;
            weed_time_message.users = Vec::new();
            weed_time_message.count = 0;
            tracing::info!("Chain broken, resetting channel entry.");
        }

        // DB Updates:
        //   UserStats:
        //     Chains Broken + 1
        //   GuildStats:
        //     --

        Ok(Some((UserStatsUpdate, GuildStatsUpdate)))
    }
}

// WeedTime::update(&msg, &ctx)?.await -> Result<(UserStatsUpdate, GuildStatsUpdate), serenity::Error>
//     .commit(&db) -> Result<(), db_type::Error>
