use bonsaidb::core::schema::SerializedCollection;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
    prelude::Context,
};
use tracing::error;

use crate::db::{self, GuildStats};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("serverstats")
        .description("Check your server weed stats")
}

pub async fn run(
    ctx: &Context,
    interaction: &ApplicationCommandInteraction,
) -> Result<(), serenity::Error> {
    let db = db::open().await.unwrap();
    let guild_id = match interaction.guild_id {
        Some(guild_id) => guild_id,
        None => return Ok(()),
    };

    let guild = guild_id.to_partial_guild(&ctx.http).await.unwrap();

    let GuildStats {
        guild: _,
        total_weed_times,
        total_weed_crimes,
        longest_chain,
    } = match GuildStats::get_async::<_, u64>(guild_id.into(), &db).await {
        Ok(doc) => match doc {
            Some(doc) => doc.contents,
            None => return Ok(()),
        },
        Err(e) => {
            error!("Error getting guild_id from db: {e:?}");
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
                            .author(|a| a.name(guild.name.clone()))
                            .thumbnail(guild.icon_url().clone().unwrap())
                            .fields([
                                ("Weed Times", total_weed_times, true),
                                ("Weed Crimes", total_weed_crimes, true),
                                ("Longest Chain", longest_chain, true),
                            ])
                    })
                })
        })
        .await?;
    Ok(())
}
