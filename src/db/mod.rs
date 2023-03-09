use bonsaidb::{
    core::schema::{Collection, Schema},
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
    user: UserId,
    pub total_weed_times: u32,
    pub total_weed_crimes: u32,
    pub chains_started: u32,
    pub chains_broken: u32,
}

pub async fn open() -> Result<AsyncDatabase, bonsaidb::local::Error> {
    AsyncDatabase::open::<WeedTimeSchema>(StorageConfiguration::new("basic.bonsaidb")).await
}
