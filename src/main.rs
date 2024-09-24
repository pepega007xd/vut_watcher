use serenity::{async_trait, model::prelude::*, prelude::*};
use std::{error::Error, io::Write, time::Duration};

const GUILD_ID_KENTUS_BLENTUS: u64 = 1200141239975153674;

const KENTUS_CHANNEL_NAME: &str = "kentusovy-dristy";

const REFRESH_DURATION: Duration = Duration::from_secs(60);

async fn get_registration_page() -> Option<String> {
    let client = reqwest::Client::new();
    let page = client
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
        .ok();

    let mut output_file = std::fs::File::create("./input.html").unwrap();
    output_file.write(page.clone().unwrap().as_bytes()).unwrap();

    page
}

async fn get_tkey() -> (String, String) {
    let input = get_registration_page().await.unwrap();
    let html = scraper::Html::parse_document(&input);

    let key_selector = scraper::Selector::parse(r#"input[name="s_key"]"#)
        .ok()
        .unwrap();

    let key = html
        .select(&key_selector)
        .next()
        .unwrap()
        .value()
        .attr("value")
        .unwrap()
        .to_owned();

    let tkey_selector = scraper::Selector::parse(r#"input[name="s_tkey"]"#)
        .ok()
        .unwrap();

    let tkey = html
        .select(&tkey_selector)
        .next()
        .unwrap()
        .value()
        .attr("value")
        .unwrap()
        .to_owned();

    (key, tkey)
}

async fn register_fitstagram() -> String {
    let (key, tkey) = get_tkey().await;
    let payload = format!("script_name=zadani_registrace_act&apid=281143&zid=58233&s_key={key}&s_tkey={tkey}&prihlasit=Zaregistrovat+se+na+toto+zad%C3%A1n%C3%AD");

    let client = reqwest::Client::new();
    let request = client
        .post("https://www.vut.cz/studis/student.phtml")
        .header(
            reqwest::header::COOKIE,
            include_str!("../session-cookie").trim(),
        )
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .header(
            reqwest::header::REFERER,
            "https://www.vut.cz/studis/student.phtml",
        )
        .header(reqwest::header::ORIGIN, "https://www.vut.cz")
        .header(reqwest::header::CONNECTION, "keep-alive")
        .header(reqwest::header::PRAGMA, "no-cache")
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.5")
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "no-cors")
        .header("Sec-Fetch-Site", "same-origin")
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0",
        )
        .body(payload)
        .build()
        .unwrap();

    let response = client.execute(request).await.unwrap().text().await.unwrap();

    let mut output_file = std::fs::File::create("./output.html").unwrap();
    output_file.write(response.as_bytes()).unwrap();

    response
}

async fn get_autoreg_result() -> Option<String> {
    let input = register_fitstagram().await;
    let alert_selector = scraper::Selector::parse(r#"div[class="alert-text"]>div"#)
        .ok()
        .unwrap();

    let html = scraper::Html::parse_document(&input);

    html.select(&alert_selector)
        .next()?
        .text()
        .next()
        .map(|s| s.to_owned())
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
            if let Some(alert) = get_autoreg_result().await {
                if alert.as_str() == "Vybrané zadání nebylo možné zaregistrovat. V tomto časovém okamžiku není registrace zadání povolena." {
                    tokio::time::sleep(REFRESH_DURATION).await;
                    continue;
                } else {
                    eprintln!("alert: {alert}");
                }
            }

            eprintln!("registration page returned something, check the logs");
            kentus_channel
                .say(
                    &ctx.http,
                    "<@604953070308163604> registration page returned something, check the logs",
                )
                .await
                .unwrap();

            break;
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        match msg.content.as_str() {
            "!kentus-test" => {
                msg.reply(&ctx.http, "I'm fine").await.unwrap();
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
