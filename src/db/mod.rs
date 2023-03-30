use bonsaidb::{
    core::{
        connection::AsyncConnection,
        schema::{Collection, Schema, SerializedCollection},
    },
    local::{
        config::{Builder, StorageConfiguration},
        AsyncDatabase,
    },
};
use serde::{Deserialize, Serialize};
use serenity::model::prelude::{GuildId, UserId};

#[derive(Debug, Schema)]
#[schema(name = "WeedTimeSchema", collections = [GuildStats, UserStats])]
pub struct WeedTimeSchema;

#[derive(Debug, Serialize, Deserialize, Default, Collection, PartialEq, Eq, PartialOrd, Ord)]
#[collection(name = "GuildStats")]
#[collection(natural_id = |stats: &Self| Some(stats.guild.into()))]
pub struct GuildStats {
    pub guild: GuildId,
    pub total_weed_times: u32,
    pub total_weed_crimes: u32,
    pub longest_chain: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Collection, PartialEq, Eq, PartialOrd, Ord)]
#[collection(name = "UserStats")]
#[collection(natural_id = |stats: &Self| Some(stats.user.into()))]
pub struct UserStats {
    pub user: UserId,
    pub total_weed_times: u32,
    pub total_weed_crimes: u32,
    pub chains_started: u32,
    pub chains_broken: u32,
}

#[derive(Debug)]
pub struct EventData {
    pub guild_id: GuildId,
    pub user_id: UserId,
}

#[derive(Debug)]
pub enum WeedTimeEvent {
    // New Chain will update server db by incrementing the amount of weed times and setting longest
    // chain to one if it is 0.
    // It will update the user db by incrementing the chains started for the user.
    NewChain(EventData),
    // Chain broken will increment the chains broken variable in the user db by one.
    ChainBroken(EventData),
    // Weed Crime will increment the weed crime variable in the user db by one.
    WeedCrime(EventData),
    // Weed Time will increment the weed time variable in the server db by one and update the
    // longest chain variable if the chain length is greater than the current longest chain.
    // It will update the weed time variable in the user db by one.
    WeedTime(u32, EventData),
}

pub async fn open() -> Result<AsyncDatabase, bonsaidb::local::Error> {
    AsyncDatabase::open::<WeedTimeSchema>(StorageConfiguration::new("basic.bonsaidb")).await
}

pub async fn commit_event<C: AsyncConnection>(
    event: WeedTimeEvent,
    connection: &C,
) -> Result<(), bonsaidb::local::Error> {
    match event {
        WeedTimeEvent::NewChain(EventData { guild_id, user_id }) => {
            // Increment total_weed_times in GuildStats and set longest_chain to 1 if it is less
            // than one
            // Increment chains_started and total_weed_times in UserStats

            match GuildStats::get_async::<_, u64>(guild_id.into(), connection).await? {
                Some(mut guild_stats) => {
                    guild_stats.contents.total_weed_times += 1;
                    if guild_stats.contents.longest_chain > 1 {
                        guild_stats.contents.longest_chain = 1;
                    }
                    guild_stats.update_async(connection).await?;
                }
                None => {
                    GuildStats {
                        guild: guild_id,
                        total_weed_times: 1,
                        longest_chain: 1,
                        ..Default::default()
                    }
                    .push_into_async(connection)
                    .await?;
                }
            }

            match UserStats::get_async::<_, u64>(user_id.into(), connection).await? {
                Some(mut user_stats) => {
                    user_stats.contents.chains_started += 1;
                    user_stats.contents.total_weed_times += 1;
                    user_stats.update_async(connection).await?;
                }
                None => {
                    UserStats {
                        user: user_id,
                        total_weed_times: 1,
                        chains_started: 1,
                        ..Default::default()
                    }
                    .push_into_async(connection)
                    .await?;
                }
            }
        }
        WeedTimeEvent::ChainBroken(EventData { user_id, .. }) => {
            // Increment chains_broken in UserStats
            match UserStats::get_async::<_, u64>(user_id.into(), connection).await? {
                Some(mut user_stats) => {
                    user_stats.contents.chains_broken += 1;
                    user_stats.update_async(connection).await?;
                }
                None => {
                    UserStats {
                        user: user_id,
                        chains_broken: 1,
                        ..Default::default()
                    }
                    .push_into_async(connection)
                    .await?;
                }
            }
        }
        WeedTimeEvent::WeedCrime(EventData { guild_id, user_id }) => {
            // Increment total_weed_crimes in GuildStats
            // Increment total_weed_crimes in UserStats

            match GuildStats::get_async::<_, u64>(guild_id.into(), connection).await? {
                Some(mut guild_stats) => {
                    guild_stats.contents.total_weed_crimes += 1;
                    guild_stats.update_async(connection).await?;
                }
                None => {
                    GuildStats {
                        guild: guild_id,
                        total_weed_crimes: 1,
                        ..Default::default()
                    }
                    .push_into_async(connection)
                    .await?;
                }
            }

            match UserStats::get_async::<_, u64>(user_id.into(), connection).await? {
                Some(mut user_stats) => {
                    user_stats.contents.total_weed_crimes += 1;
                    user_stats.update_async(connection).await?;
                }
                None => {
                    UserStats {
                        user: user_id,
                        total_weed_crimes: 1,
                        ..Default::default()
                    }
                    .push_into_async(connection)
                    .await?;
                }
            }
        }
        WeedTimeEvent::WeedTime(count, EventData { guild_id, user_id }) => {
            // Increment total_weed_times in GuildStats and set longest_chain to count if count is
            // greater than longest_chain
            // Increment total_weed_times in UserStats

            match GuildStats::get_async::<_, u64>(guild_id.into(), connection).await? {
                Some(mut guild_stats) => {
                    guild_stats.contents.total_weed_times += 1;
                    if guild_stats.contents.longest_chain < count {
                        guild_stats.contents.longest_chain = count;
                    }
                    guild_stats.update_async(connection).await?;
                }
                None => {
                    GuildStats {
                        guild: guild_id,
                        total_weed_times: 1,
                        longest_chain: 1,
                        ..Default::default()
                    }
                    .push_into_async(connection)
                    .await?;
                }
            }

            match UserStats::get_async::<_, u64>(user_id.into(), connection).await? {
                Some(mut user_stats) => {
                    user_stats.contents.total_weed_times += 1;
                    user_stats.update_async(connection).await?;
                }
                None => {
                    UserStats {
                        user: user_id,
                        total_weed_times: 1,
                        ..Default::default()
                    }
                    .push_into_async(connection)
                    .await?;
                }
            }
        }
    }
    Ok(())
}
