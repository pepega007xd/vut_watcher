use serenity::{async_trait, model::prelude::*, prelude::*};
use std::{collections::HashSet, error::Error, time::Duration};

const GUILD_ID_KENTUS_BLENTUS: u64 = 1200141239975153674;

const KENTUS_CHANNEL_NAME: &str = "kentusovy-dristy";

const REFRESH_DURATION: Duration = Duration::from_secs(30);

const INPUT_IDS_TO_WATCH: &'static [&'static str] = &["rj[281096][400451]-334770"];

async fn get_registration_page() -> Option<String> {
    let client = reqwest::Client::new();
    client
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
        .ok()
}

async fn get_button() -> Option<String> {
    let input = get_registration_page().await.unwrap();
    let html = scraper::Html::parse_document(&input);
    let input_buttons_selector = scraper::Selector::parse(r#"input"#).ok().unwrap();

    Some(
        html.select(&input_buttons_selector)
            // get all input buttons on watchlist
            .filter(|input| {
                if let Some(id) = input.value().id() {
                    INPUT_IDS_TO_WATCH.contains(&id)
                } else {
                    false
                }
            })
            // which are not disabled (full)
            .filter(|input| input.value().attr("disabled").is_none())
            .next()?
            .html(),
    )
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

        let kentus_channel = get_channel(
            &ready.guilds,
            GUILD_ID_KENTUS_BLENTUS,
            &ctx,
            KENTUS_CHANNEL_NAME,
        )
        .await
        .unwrap();

        loop {
            if let Some(button) = get_button().await {
                println!("Available button: {button}");
                kentus_channel
                    .say(
                        &ctx.http,
                        "⚠️ @pepega007xd @.raptorka @pevol2 **ITU cviko v utery 10:00 dostupne**",
                    )
                    .await
                    .unwrap();
                // send only one message
                break;
            }

            tokio::time::sleep(REFRESH_DURATION).await;
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        match msg.content.as_str() {
            "!kentus-test" => {
                msg.reply(&ctx.http, "I'm fine").await.unwrap();
            }
            "!kentus-check" => {
                msg.reply(
                    &ctx.http,
                    format!("enabled button: {:?}", get_button().await),
                )
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
