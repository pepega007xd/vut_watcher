use serenity::{async_trait, model::prelude::*, prelude::*};
use std::{collections::HashSet, error::Error, time::Duration};

const GUILD_ID_PADI_SERVER: u64 = 1035650956383227995;
const GUILD_ID_KENTUS_BLENTUS: u64 = 1200141239975153674;

const PADI_CHANNEL_NAME: &str = "webhooci-hovnaci";
const KENTUS_CHANNEL_NAME: &str = "kentusovy-dristy";

const REFRESH_DURATION: Duration = Duration::from_secs(30);

const IMP_SUBJECT_ID: &str = "281143";

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

async fn get_subjects() -> Option<HashSet<String>> {
    let input = get_registration_page().await?;
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

async fn get_current_keys() -> (String, String) {
    let input = get_registration_page().await.unwrap();

    let html = scraper::Html::parse_document(&input);
    let s_key_selector = scraper::Selector::parse(r#"input[name="s_key"]"#)
        .ok()
        .unwrap();
    let s_tkey_selector = scraper::Selector::parse(r#"input[name="s_tkey"]"#)
        .ok()
        .unwrap();
    (
        html.select(&s_key_selector)
            .next()
            .unwrap()
            .value()
            .attr("value")
            .unwrap()
            .to_owned(),
        html.select(&s_tkey_selector)
            .next()
            .unwrap()
            .value()
            .attr("value")
            .unwrap()
            .to_owned(),
    )
}

async fn get_cvika() -> Vec<(String, String)> {
    let input = get_registration_page().await.unwrap();

    let html = scraper::Html::parse_document(&input);

    let row_selector = scraper::Selector::parse(r#"div[class="den"]"#)
        .ok()
        .unwrap();
    let day_name_selector = scraper::Selector::parse(r#"div[class="popis"]"#)
        .ok()
        .unwrap();

    let buttons_selector = scraper::Selector::parse(r#"input[type="radio"]"#)
        .ok()
        .unwrap();

    let wednesday_row = html
        .select(&row_selector)
        .find(|row| {
            row.select(&day_name_selector)
                .next()
                .unwrap()
                .text()
                .next()
                .unwrap()
                == "St".to_string()
        })
        .unwrap();

    wednesday_row
        .select(&buttons_selector)
        .map(|x| x.value().id().unwrap())
        .filter(|x| x.starts_with(&format!("rj[{IMP_SUBJECT_ID}][")))
        .map(|x| {
            let mut numbers = x
                .trim_start_matches(&format!("rj[{IMP_SUBJECT_ID}]["))
                .split("]-");
            (
                numbers.next().unwrap().to_owned(),
                numbers.next().unwrap().to_owned(),
            )
        })
        .collect::<Vec<_>>()
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

async fn auto_register() {
    let (s_key, s_tkey) = get_current_keys().await;

    let (imp_lesson_type_id, imp_lesson_time_id) = if let Some(ids) = get_cvika().await.get(0) {
        println!("registering with {ids:?}");
        ids.clone()
    } else {
        println!("cvika nejsou :tragika:");
        return;
    };

    let imp_index_string_1 = format!("f_rj[{imp_lesson_type_id}]");
    let imp_index_string_2 = format!("f_rj[{IMP_SUBJECT_ID}][{imp_lesson_type_id}]");

    let params = [
        ("script_name", "registrace_vyucovani_act"),
        ("typ_semestru_id", ""),
        ("kontrola_vsech_rj", ""),
        ("zmena_semestru", "0"),
        ("s_key", s_key.as_str()),
        ("s_tkey", s_tkey.as_str()),
        ("f_rj[400112]", "400112"),
        ("f_rj[400131]", "400131"),
        ("f_rj[400247]", "400247"),
        ("f_rj[400248]", "400248"),
        ("f_rj[400450]", "400450"),
        ("f_rj[400451]", "400451"),
        ("f_rj[400546]", "400546"),
        ("f_rj[400547]", "400547"),
        (imp_index_string_1.as_str(), imp_lesson_type_id.as_str()),
        ("rj[281096][400450]", "334759"),
        ("rj[281096][400451]", "334768"),
        ("rj[281143][400546]", "334816"),
        ("rj[281143][400547]", "334817"),
        (imp_index_string_2.as_str(), imp_lesson_time_id.as_str()),
        ("rj[280933][400112]", "334511"),
        ("rj[280994][400248]", "334609"),
        ("rj[280945][400131]", "334513"),
        ("rj[280994][400247]", "334608"),
        ("potvrdit_volbu_vyucovani", "Potvrdit+registraci+vyučování"),
    ];

    let client = reqwest::Client::new();
    client
        .post("https://www.vut.cz/studis/student.phtml?sn=registrace_vyucovani")
        .header(
            reqwest::header::COOKIE,
            include_str!("../session-cookie").trim(),
        )
        .form(&params)
        .send()
        .await
        .unwrap();

    println!("sent autoregistration request")
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
            tokio::time::sleep(REFRESH_DURATION).await;

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
                info_channel.say(&ctx.http, &message).await.unwrap();
                err_channel.say(&ctx.http, &message).await.unwrap();

                if difference.contains("IMP") {
                    auto_register().await;
                }
            } else {
                eprintln!("⚠️ @pepega007xd Fetching registration page failed.⚠️ \n https://www.vut.cz/studis/student.phtml?sn=registrace_vyucovani");
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
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
            "!kentus-autoreg" => {
                if &msg.author.name == "pepega007xd" {
                    auto_register().await;
                    msg.reply(&ctx.http, "Autoregistration triggered")
                        .await
                        .unwrap();
                }
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
