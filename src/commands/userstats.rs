use bonsaidb::core::schema::SerializedCollection;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::{
        command::CommandOptionType,
        interaction::{
            application_command::{
                ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
            },
            InteractionResponseType,
        },
    },
    prelude::Context,
};
use tracing::error;

use crate::db::{self, UserStats};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("userstats")
        .description("Check your or a friend's weed stats")
        .create_option(|option| {
            option
                .name("user")
                .description("the user you want to see the stats of")
                .kind(CommandOptionType::User)
        })
}

pub async fn run(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    options: &[CommandDataOption],
) -> Result<(), serenity::Error> {
    let db = db::open().await.unwrap();
    let mut user_data = None;

    for data in options {
        match data.name.as_str() {
            "user" => user_data = Some(data),
            _ => (),
        }
    }

    let user = match user_data {
        Some(data) => {
            let resolved = data.resolved.as_ref();
            match resolved {
                Some(value) => {
                    if let CommandDataOptionValue::User(user, _) = value {
                        user
                    } else {
                        &interaction.user
                    }
                }
                None => &interaction.user,
            }
        }
        None => &interaction.user,
    };

    let UserStats {
        user: _,
        total_weed_times,
        total_weed_crimes,
        chains_started,
        chains_broken,
    } = match UserStats::get_async::<_, u64>(user.id.into(), &db).await {
        Ok(doc) => match doc {
            Some(doc) => doc.contents,
            None => {
                UserStats {
                    user: user.id,
                    ..Default::default()
                }
                .push_into_async(&db)
                .await
                .unwrap()
                .contents
            }
        },
        Err(e) => {
            error!("Error getting user_id from db: {e:?}");
            return Ok(());
        }
    };

    interaction
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.embed(|e| {
                        e.color(0x25fe03)
                            .title("WEED STATS")
                            .author(|a| a.name(user.name.clone()))
                            .thumbnail(user.avatar_url().unwrap())
                            .fields([
                                ("Weed Times", total_weed_times, true),
                                ("Weed Crimes", total_weed_crimes, true),
                                ("Chains Started", chains_started, true),
                                ("Chains Broken", chains_broken, true),
                            ])
                    })
                })
        })
        .await?;
    Ok(())
}
