use serenity::{async_trait, model::prelude::*, prelude::*};
use std::{collections::HashSet, error::Error, time::Duration};

const GUILD_ID_PADI_SERVER: u64 = 1035650956383227995;
const GUILD_ID_KENTUS_BLENTUS: u64 = 1200141239975153674;

const PADI_CHANNEL_NAME: &str = "webhooci-hovnaci";
const KENTUS_CHANNEL_NAME: &str = "kentusovy-dristy";

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
    html.select(&rows_selector).next()?; // fails if page doesn't contain the table

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
        eprintln!("{} started", ready.user.name);

        let info_channel =
            get_channel(&ready.guilds, GUILD_ID_PADI_SERVER, &ctx, PADI_CHANNEL_NAME)
                .await
                .unwrap();
        let err_channel = get_channel(
            &ready.guilds,
            GUILD_ID_KENTUS_BLENTUS,
            &ctx,
            KENTUS_CHANNEL_NAME,
        )
        .await
        .unwrap();

        let mut prev_subjects = get_subjects().await.unwrap();
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            if let Some(new_subjects) = get_subjects().await {
                if new_subjects == prev_subjects {
                    continue;
                }
                let difference = new_subjects
                    .difference(&prev_subjects)
                    .map(|s| format!("{s}\n"))
                    .collect::<String>();
                prev_subjects = new_subjects;

                let message = format!("⚠️  @everyone Registration page changed. These subjects are now available: \n{difference} https://www.vut.cz/studis/student.phtml?sn=registrace_vyucovani");
                eprintln!("{message}");
                // info_channel.say(&ctx.http, &message).await.unwrap();
                let debug_message = format!("⚠️  @pepega007xd Registration page changed. These subjects are now available: \n{difference} https://www.vut.cz/studis/student.phtml?sn=registrace_vyucovani");
                err_channel.say(&ctx.http, &debug_message).await.unwrap();
            } else {
                eprintln!("⚠️ Fetching registration page failed.⚠️ \n https://www.vut.cz/studis/student.phtml?sn=registrace_vyucovani");
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        println!("message");
        match msg.content.as_str() {
            "!kentus-test" => {
                msg.reply(&ctx.http, "I'm fine").await.unwrap();
            }
            "!kentus-manual" => {
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
            _ => (),
        };
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
