use select::{
    document::Document,
    predicate::*,
};

pub struct SSauspielCookie {
    pub headerval_cookie: reqwest::header::HeaderValue,
}

pub struct SSauspielCredentials {
    pub str_user: String,
    pub str_pass: String,
}

pub fn fetch_html(str_url: &str, headerval_cookie: &reqwest::header::HeaderValue) -> Result<String, failure::Error> {
    let mut resp = reqwest::Client::new()
        .get(str_url)
        .header(reqwest::header::COOKIE, headerval_cookie)
        .send()?;
    if resp.status().is_success() {
        resp.text().map_err(|err| format_err!("error retrieving text from response: {:?}", err))
    } else {
        fetch_html(str_url, headerval_cookie)
    }
}

pub fn token_and_cookie(sauspielcredentials: &SSauspielCredentials) -> Result<SSauspielCookie, failure::Error> {
    let str_url_login = "https://www.sauspiel.de/login".to_owned();
    let resp_cookie = reqwest::get(&str_url_login)?;
    let header_cookie = resp_cookie.headers().get(reqwest::header::SET_COOKIE)
        .ok_or_else(|| format_err!("Expected set-cookie."))?;
    let str_html_token = fetch_html(&str_url_login, header_cookie)?;
    let doc_token = Document::from(&str_html_token as &str);
    // TODO first_if_all_same instead of collecting and asserting
    let vecnode_token = doc_token.find(Attr("name", "authenticity_token"))
        .collect::<Vec<_>>();
    let str_token = vecnode_token
        .first().ok_or_else(|| format_err!("authenticity_token not found"))?
        .attr("value").ok_or_else(|| format_err!("authenticity_token has no value"))?;
    assert!(vecnode_token.iter().all(|node| node.attr("value")==Some(str_token)));
    let resp = reqwest::Client::builder()
        .redirect(reqwest::RedirectPolicy::none())
        .build()?
        .post(&str_url_login)
        .header(reqwest::header::COOKIE, header_cookie) // needed. Otherwise we get HTTP 403
        // the following are copy-pasted from Firefox
        //.header(reqwest::header::ACCEPT, "text/html,application/xhtml+xm…ml;q=0.9,image/webp,*/*;q=0.8")
        //.header(reqwest::header::ACCEPT_ENCODING, "gzip, deflate, br")
        //.header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.5")
        //.header(reqwest::header::CONNECTION, "keep-alive")
        //.header(reqwest::header::CONTENT_LENGTH, "132")
        //.header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        //.header(reqwest::header::HOST, "www.sauspiel.de")
        //.header(reqwest::header::REFERER, "https://www.sauspiel.de/login")
        //.header(reqwest::header::UPGRADE_INSECURE_REQUESTS, "1")
        //.header(reqwest::header::USER_AGENT, "Mozilla/5.0 (X11; Ubuntu; Linu…) Gecko/20100101 Firefox/65.0")
        .form(
            &[
                // TODO: assert that those are the elements of <input>
                ("utf8", "✓"),
                ("login", &sauspielcredentials.str_user),
                ("password", &sauspielcredentials.str_pass),
                ("authenticity_token",str_token),
                ("remember_me", "0"),
            ]
        )
        .send()?;
    let headerval_cookie = resp.headers().get(reqwest::header::SET_COOKIE)
        .map(|str_cookie| str_cookie.to_owned())
        .ok_or_else(|| format_err!("Expected set-cookie"))?;
    //assert_eq!(headerval_cookie, header_cookie); // does not hold
    Ok(SSauspielCookie{ headerval_cookie })
}

