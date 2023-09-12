use serenity::{async_trait, model::prelude::*, prelude::*};
use std::{collections::HashSet, error::Error, time::Duration};

const GID_AAAA: u64 = 1035650956383227995;
const GID_TEST: u64 = 633337442480422912;

async fn get_subjects() -> Option<HashSet<String>> {
    let client = reqwest::Client::new();
    let input = client
        .get("https://www.vut.cz/studis/student.phtml?sn=registrace_vyucovani")
        .header(
            reqwest::header::COOKIE,
            include_str!("../session-cookie").trim(),
        )
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    let html = scraper::Html::parse_document(&input);
    let rows_selector =
        scraper::Selector::parse("form#registrace_vyucovani>table>tbody>tr").ok()?;
    let names_selector = scraper::Selector::parse("label").ok()?;

    let names = html
        .select(&rows_selector)
        // second cell is the subject name
        .map(|row| row.select(&names_selector).nth(1).map(|x| x.inner_html()))
        .flatten()
        .filter(|s| !s.starts_with("Zobrazit")) // remove garbage line
        .collect::<HashSet<_>>();

    Some(names)
}

async fn get_channel(
    guilds: &[UnavailableGuild],
    guild_id: u64,
    ctx: &Context,
    channel_name: &str,
) -> Option<ChannelId> {
    let guild = guilds.iter().find(|g| g.id.0 == guild_id)?;
    let channels = guild.id.channels(&ctx.http).await.ok()?;
    Some(channels.into_iter().find(|c| c.1.name == channel_name)?.0)
}

struct DiscordHandler;

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} started", ready.user.name);

        let info_channel = get_channel(&ready.guilds, GID_AAAA, &ctx, "webhooci-hovnaci")
            .await
            .unwrap();
        let err_channel = get_channel(&ready.guilds, GID_TEST, &ctx, "bot-info")
            .await
            .unwrap();

        let mut prev_subjects = get_subjects().await.unwrap();
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let new_subjects = get_subjects().await;

            if let Some(new_subjects) = new_subjects {
                if new_subjects == prev_subjects {
                    continue;
                }
                let difference = new_subjects
                    .difference(&prev_subjects)
                    .map(|s| format!("{s}\n"))
                    .collect::<String>();
                prev_subjects = new_subjects;

                let message = format!("⚠️ Registration page changed. These subjects are now available: \n{difference} https://www.vut.cz/studis/student.phtml?sn=registrace_vyucovani");
                println!("{message}");
                info_channel.say(&ctx.http, &message).await.unwrap();
                err_channel.say(&ctx.http, &message).await.unwrap();
            } else {
                let message = format!("⚠️ Fetching registration page failed.⚠️ \n https://www.vut.cz/studis/student.phtml?sn=registrace_vyucovani");
                println!("{message}");
                err_channel.say(&ctx.http, &message).await.unwrap();
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!kentus-test" {
            msg.reply(&ctx.http, "I'm fine").await.unwrap();
        }
        if msg.content == "!kentus-manual" {
            let subjects = get_subjects()
                .await
                .unwrap()
                .into_iter()
                .map(|s| format!("{s}\n"))
                .collect::<String>();

            msg.reply(&ctx.http, format!("Manual poll: Available: \n {subjects}"))
                .await
                .unwrap();
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let mut bot = Client::builder(include_str!("../discord-token").trim(), intents)
        .event_handler(DiscordHandler)
        .await?;
    bot.start().await?;

    Ok(())
}
