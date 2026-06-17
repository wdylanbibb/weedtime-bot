use native_db::Models;
use once_cell::sync::Lazy;

static USER_MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<data::v1::UserStats>().unwrap();
    models
});

static GUILD_MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<data::v1::GuildStats>().unwrap();
    models
});

pub mod data {
    use native_db::{Key, ToKey, native_db};
    use native_model::{Model, native_model};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub type UserStats = v1::UserStats;
    pub type GuildStats = v1::GuildStats;

    pub mod v1 {
        use std::path::Path;

        use native_db::{Builder, Database, db_type};

        use super::*;

        #[derive(Debug, Clone, Copy)]
        pub struct UserId(serenity::all::UserId);

        impl Serialize for UserId {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_u64(self.0.get())
            }
        }

        impl<'de> Deserialize<'de> for UserId {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Ok(Self(serenity::all::UserId::new(u64::deserialize(
                    deserializer,
                )?)))
            }
        }

        impl UserId {
            pub fn get(&self) -> serenity::all::UserId {
                self.0
            }
        }

        impl ToKey for UserId {
            fn to_key(&self) -> Key {
                self.0.get().to_key()
            }

            fn key_names() -> Vec<String> {
                vec!["UserId".to_string()]
            }
        }

        impl From<serenity::all::UserId> for UserId {
            fn from(value: serenity::all::UserId) -> Self {
                UserId(value)
            }
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[native_model(id = 1, version = 1)]
        #[native_db]
        pub struct UserStats {
            #[primary_key]
            id: UserId,
            pub weed_times: u32,
            pub weed_crimes: u32,
            pub chains_started: u32,
            pub chains_broken: u32,
        }

        impl UserStats {
            pub fn id(&self) -> serenity::all::UserId {
                self.id.get()
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct GuildId(serenity::all::GuildId);

        impl Serialize for GuildId {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_u64(self.0.get())
            }
        }

        impl<'de> Deserialize<'de> for GuildId {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Ok(Self(serenity::all::GuildId::new(u64::deserialize(
                    deserializer,
                )?)))
            }
        }

        impl GuildId {
            pub fn get(&self) -> serenity::all::GuildId {
                self.0
            }
        }

        impl ToKey for GuildId {
            fn to_key(&self) -> Key {
                self.0.get().to_key()
            }

            fn key_names() -> Vec<String> {
                vec!["GuildId".to_string()]
            }
        }

        impl From<serenity::all::GuildId> for GuildId {
            fn from(value: serenity::all::GuildId) -> Self {
                GuildId(value)
            }
        }

        #[derive(Serialize, Deserialize, Debug)]
        #[native_model(id = 2, version = 1)]
        #[native_db]
        pub struct GuildStats {
            #[primary_key]
            id: GuildId,
            pub timezone: chrono_tz::Tz,
            pub weed_times: u32,
            pub weed_crimes: u32,
            pub longest_chain: u32,
        }

        impl GuildStats {
            pub fn id(&self) -> serenity::all::GuildId {
                self.id.get()
            }
        }

        pub trait WeedTimeDatabase {}

        pub struct UserStatsDatabase<'a>(Database<'a>);

        pub struct GuildStatsDatabase<'a>(Database<'a>);

        impl UserStatsDatabase<'static> {
            pub fn create(path: impl AsRef<Path>) -> Result<Self, db_type::Error> {
                Ok(Self(Builder::new().create(&crate::USER_MODELS, path)?))
            }

            pub fn open(path: impl AsRef<Path>) -> Result<Self, db_type::Error> {
                Ok(Self(Builder::new().open(&crate::USER_MODELS, path)?))
            }

            pub fn create_in_memory() -> Result<Self, db_type::Error> {
                Ok(Self(Builder::new().create_in_memory(&crate::USER_MODELS)?))
            }
        }

        impl<'a> UserStatsDatabase<'a> {
            pub fn get(
                &self,
                user_id: serenity::all::UserId,
            ) -> Result<Option<UserStats>, db_type::Error> {
                let r = self.0.r_transaction()?;
                r.get().primary::<UserStats>(UserId::from(user_id))
            }
        }

        impl GuildStatsDatabase<'static> {
            pub fn create(path: impl AsRef<Path>) -> Result<Self, db_type::Error> {
                Ok(Self(Builder::new().create(&crate::GUILD_MODELS, path)?))
            }

            pub fn open(path: impl AsRef<Path>) -> Result<Self, db_type::Error> {
                Ok(Self(Builder::new().open(&crate::GUILD_MODELS, path)?))
            }

            pub fn create_in_memory() -> Result<Self, db_type::Error> {
                Ok(Self(Builder::new().create_in_memory(&crate::GUILD_MODELS)?))
            }
        }

        impl<'a> GuildStatsDatabase<'a> {
            pub fn get(
                &self,
                guild_id: serenity::all::GuildId,
            ) -> Result<Option<GuildStats>, db_type::Error> {
                let r = self.0.r_transaction()?;
                r.get().primary::<GuildStats>(GuildId::from(guild_id))
            }
        }

        impl<'a> WeedTimeDatabase for UserStatsDatabase<'a> {}
        impl<'a> WeedTimeDatabase for GuildStatsDatabase<'a> {}
        impl<'a, 'b> WeedTimeDatabase for (UserStatsDatabase<'a>, GuildStatsDatabase<'b>) {}

        pub trait DbUpdate<T: WeedTimeDatabase> {
            fn commit(&self, db: &T) -> Result<(), db_type::Error>;
        }

        #[derive(Debug, Clone, Copy, Default)]
        pub struct UserStatsUpdate {
            pub user_id: Option<serenity::all::UserId>,
            pub weed_times: u32,
            pub weed_crimes: u32,
            pub chains_started: u32,
            pub chains_broken: u32,
        }

        impl UserStatsUpdate {
            pub fn new(user_id: serenity::all::UserId) -> Self {
                Self {
                    user_id: Some(user_id),
                    ..Self::default()
                }
            }
        }

        impl<'a> DbUpdate<UserStatsDatabase<'a>> for UserStatsUpdate {
            fn commit(&self, db: &UserStatsDatabase) -> Result<(), db_type::Error> {
                let Some(user_id) = self.user_id else {
                    return Ok(());
                };

                let rw = db.0.rw_transaction()?;
                let mut stats = rw
                    .get()
                    .primary::<UserStats>(UserId::from(user_id))?
                    .unwrap_or(UserStats {
                        id: UserId::from(user_id),
                        weed_times: 0,
                        weed_crimes: 0,
                        chains_started: 0,
                        chains_broken: 0,
                    });

                stats.weed_times = stats.weed_times.saturating_add(self.weed_times);
                stats.weed_crimes = stats.weed_crimes.saturating_add(self.weed_crimes);
                stats.chains_started = stats.chains_started.saturating_add(self.chains_started);
                stats.chains_broken = stats.chains_broken.saturating_add(self.chains_broken);

                rw.upsert(stats)?;
                rw.commit()?;
                Ok(())
            }
        }

        #[derive(Debug, Clone, Copy, Default)]
        pub struct GuildStatsUpdate {
            pub guild_id: Option<serenity::all::GuildId>,
            pub timezone: Option<chrono_tz::Tz>,
            pub weed_times: u32,
            pub weed_crimes: u32,
            pub longest_chain: Option<u32>,
        }

        impl GuildStatsUpdate {
            pub fn new(guild_id: serenity::all::GuildId) -> Self {
                Self {
                    guild_id: Some(guild_id),
                    ..Self::default()
                }
            }
        }

        impl<'a> DbUpdate<GuildStatsDatabase<'a>> for GuildStatsUpdate {
            fn commit(&self, db: &GuildStatsDatabase) -> Result<(), db_type::Error> {
                let Some(guild_id) = self.guild_id else {
                    return Ok(());
                };

                let rw = db.0.rw_transaction()?;
                let mut stats = rw
                    .get()
                    .primary::<GuildStats>(GuildId::from(guild_id))?
                    .unwrap_or(GuildStats {
                        id: GuildId::from(guild_id),
                        timezone: self.timezone.unwrap_or(chrono_tz::Tz::America__New_York),
                        weed_times: 0,
                        weed_crimes: 0,
                        longest_chain: 0,
                    });

                if let Some(timezone) = self.timezone {
                    stats.timezone = timezone;
                }
                stats.weed_times = stats.weed_times.saturating_add(self.weed_times);
                stats.weed_crimes = stats.weed_crimes.saturating_add(self.weed_crimes);
                if let Some(longest_chain) = self.longest_chain {
                    stats.longest_chain = stats.longest_chain.max(longest_chain);
                }

                rw.upsert(stats)?;
                rw.commit()?;
                Ok(())
            }
        }

        impl<'a, 'b> DbUpdate<(UserStatsDatabase<'a>, GuildStatsDatabase<'b>)>
            for (UserStatsUpdate, GuildStatsUpdate)
        {
            fn commit(
                &self,
                db: &(UserStatsDatabase<'a>, GuildStatsDatabase<'b>),
            ) -> Result<(), db_type::Error> {
                self.0.commit(&db.0)?;
                self.1.commit(&db.1)?;
                Ok(())
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn commits_user_stats_updates() -> Result<(), db_type::Error> {
                let db = UserStatsDatabase::create_in_memory()?;
                let user_id = serenity::all::UserId::new(42);

                UserStatsUpdate {
                    user_id: Some(user_id),
                    weed_times: 2,
                    weed_crimes: 1,
                    chains_started: 1,
                    chains_broken: 1,
                }
                .commit(&db)?;

                UserStatsUpdate {
                    user_id: Some(user_id),
                    weed_times: 1,
                    ..Default::default()
                }
                .commit(&db)?;

                let r = db.0.r_transaction()?;
                let stats = r
                    .get()
                    .primary::<UserStats>(UserId::from(user_id))?
                    .unwrap();
                assert_eq!(stats.weed_times, 3);
                assert_eq!(stats.weed_crimes, 1);
                assert_eq!(stats.chains_started, 1);
                assert_eq!(stats.chains_broken, 1);

                Ok(())
            }

            #[test]
            fn commits_guild_stats_updates() -> Result<(), db_type::Error> {
                let db = GuildStatsDatabase::create_in_memory()?;
                let guild_id = serenity::all::GuildId::new(420);

                GuildStatsUpdate {
                    guild_id: Some(guild_id),
                    weed_times: 1,
                    longest_chain: Some(3),
                    ..Default::default()
                }
                .commit(&db)?;

                GuildStatsUpdate {
                    guild_id: Some(guild_id),
                    weed_crimes: 2,
                    longest_chain: Some(2),
                    ..Default::default()
                }
                .commit(&db)?;

                let r = db.0.r_transaction()?;
                let stats = r
                    .get()
                    .primary::<GuildStats>(GuildId::from(guild_id))?
                    .unwrap();
                assert_eq!(stats.weed_times, 1);
                assert_eq!(stats.weed_crimes, 2);
                assert_eq!(stats.longest_chain, 3);

                Ok(())
            }
        }
    }
}
