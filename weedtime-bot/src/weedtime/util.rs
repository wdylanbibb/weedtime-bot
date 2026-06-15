use std::sync::Arc;

use chrono::{DateTime, Timelike};
use chrono_tz::Tz;
use serenity::all::{ChannelId, Context};
use whirlwind::ShardMap;

use crate::{MessageCount, WeedTimeMessage};

pub fn combo_to_emojis(combo: u32) -> String {
    // Get the amount of times a number can be divided by 10 without going under 10
    let count = std::iter::successors(Some(combo), |&n| (n >= 10).then_some(n / 10)).count();
    // Nums is an array of single digit numbers that make up the combo amount
    let nums = (0..count as u32)
        .map(|n| combo / 10_u32.pow(n) % 10)
        .rev()
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

pub fn is_420(timestamp: DateTime<Tz>) -> bool {
    let (_, hour) = timestamp.hour12();
    let minute = timestamp.minute();

    hour == 4 && minute == 20
}

pub fn has_unique_elements<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + std::hash::Hash,
{
    let mut uniq = std::collections::HashSet::new();
    iter.into_iter().all(move |x| uniq.insert(x))
}

pub async fn get_map(ctx: &Context) -> Arc<ShardMap<ChannelId, WeedTimeMessage>> {
    let data_read = ctx.data.read().await;
    data_read
        .get::<MessageCount>()
        .expect("MessageCount not found in TypeMap")
        .clone()
}
