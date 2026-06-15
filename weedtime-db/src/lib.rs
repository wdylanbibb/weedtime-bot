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
    use serde::{Deserialize, Serialize};

    pub type UserStats = v1::UserStats;
    pub type GuildStats = v1::GuildStats;

    pub mod v1 {
        use native_db::{Database, db_type};

        use super::*;

        #[derive(Serialize, Deserialize, Debug)]
        pub struct UserId(serenity::all::UserId);

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

        #[derive(Serialize, Deserialize, Debug)]
        pub struct GuildId(serenity::all::GuildId);

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

        pub trait WeedTimeDatabase {}

        pub struct UserStatsDatabase<'a>(Database<'a>);

        pub struct GuildStatsDatabase<'a>(Database<'a>);

        impl<'a> WeedTimeDatabase for UserStatsDatabase<'a> {}
        impl<'a> WeedTimeDatabase for GuildStatsDatabase<'a> {}
        impl<'a, 'b> WeedTimeDatabase for (UserStatsDatabase<'a>, GuildStatsDatabase<'b>) {}

        pub trait DbUpdate<T: WeedTimeDatabase> {
            fn commit(&self, db: &T) -> Result<(), db_type::Error>;
        }

        pub struct UserStatsUpdate;

        impl<'a> DbUpdate<UserStatsDatabase<'a>> for UserStatsUpdate {
            fn commit(&self, _db: &UserStatsDatabase) -> Result<(), db_type::Error> {
                Ok(())
            }
        }

        pub struct GuildStatsUpdate;

        impl<'a> DbUpdate<GuildStatsDatabase<'a>> for GuildStatsUpdate {
            fn commit(&self, _db: &GuildStatsDatabase) -> Result<(), db_type::Error> {
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
    }
}
