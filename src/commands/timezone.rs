use bonsaidb::core::schema::SerializedCollection;
use chrono::FixedOffset;
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

use crate::db::{self, GuildStats};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("timezone")
        .description("Check your server weed stats")
        .create_option(|option| {
            option
                .name("offset")
                .description("The UTF Offset you want to use")
                .required(true)
                .kind(CommandOptionType::Number)
        })
}

pub async fn run(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
    options: &[CommandDataOption],
) -> Result<(), serenity::Error> {
    let db = db::open().await.unwrap();
    let guild_id = match interaction.guild_id {
        Some(guild_id) => guild_id,
        None => return Ok(()),
    };

    let mut offset_data = None;

    for data in options {
        match data.name.as_str() {
            "offset" => offset_data = Some(data),
            _ => (),
        }
    }

    let content = {
        match offset_data {
            Some(data) => {
                let resolved = data.resolved.as_ref();
                match resolved {
                    Some(value) => {
                        if let CommandDataOptionValue::Number(i) = value {
                            let i = i.to_owned();
                            match FixedOffset::east_opt(i as i32 * 3600) {
                                Some(_) => {
                                    match GuildStats::get_async::<_, u64>(guild_id.into(), &db)
                                        .await
                                    {
                                        Ok(doc) => match doc {
                                            Some(mut doc) => {
                                                doc.contents.utc_offset = (i * 3600.0) as i32;
                                                doc.update_async(&db).await;
                                                "Timezone Updated!".to_string()
                                            }
                                            None => {
                                                GuildStats {
                                                    guild: guild_id,
                                                    utc_offset: (i * 3600.0) as i32,
                                                    ..Default::default()
                                                }
                                                .push_into_async(&db)
                                                .await;
                                                "Timezone Updated!".to_string()
                                            }
                                        },
                                        Err(e) => {
                                            error!("Error getting guild from database: {e:?}");
                                            "Error getting guild from database".to_string()
                                        }
                                    }
                                }
                                None => "Please enter a valid UTC Offset!".to_string(),
                            }
                        } else {
                            "Please enter a valid UTC Offset!".to_string()
                        }
                    }
                    None => "Please enter a valid UTC Offset!".to_string(),
                }
            }
            None => "Offset is required!".to_string(),
        }
    };

    interaction
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content(content))
        })
        .await?;

    Ok(())
}
