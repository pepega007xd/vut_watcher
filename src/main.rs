use std::{error::Error, io::Write, time::Duration};

const REFRESH_DURATION: Duration = Duration::from_secs(30);

async fn get_registration_page() -> Option<String> {
    let client = reqwest::Client::new();
    let page = client
        .get("https://www.vut.cz/studis/student.phtml?script_name=zadani_detail&apid=280933&zid=58330")
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
    let payload = format!("script_name=zadani_registrace_act&apid=280933&zid=58330&s_key={key}&s_tkey={tkey}&prihlasit=Zaregistrovat+se+na+toto+zad%C3%A1n%C3%AD");

    // i have no idea which of these things is actually required,
    // and im too lazy to find out
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

#[tokio::main]
async fn main() {
    loop {
        if let Some(alert) = get_autoreg_result().await {
            if alert.as_str()
                == "Vybrané zadání nebylo možné zaregistrovat. Pokus o překročení kapacity zadání."
            {
                tokio::time::sleep(REFRESH_DURATION).await;
                continue;
            } else {
                eprintln!("alert: {alert}");
            }
        }

        break;
    }
}
