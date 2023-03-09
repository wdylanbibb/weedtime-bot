use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

#[command]
pub async fn stats(_ctx: &Context, _msg: &Message) -> CommandResult {
    Ok(())
}
