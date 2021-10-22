use curl::easy::{Easy, List};
use http::StatusCode;
use serde::Deserialize;

use crate::config::Config;

const URL: &str = "https://api.intra.42.fr/";
const BASE_API: &str = "https://api.intra.42.fr/v2/";

#[derive(Deserialize)]
struct Auth {
    access_token: String,
}

#[derive(Deserialize, Clone)]
pub struct User {
    pub id: u32,
    pub login: String,
}

#[derive(Deserialize, Debug)]
pub struct Location {
    pub begin_at: Option<String>,
    pub end_at: Option<String>,
}

fn check_response(easy: &mut Easy) -> Result<(), String> {
    match easy.response_code() {
        Ok(200) => Ok(()),
        Ok(code) => {
            let status = StatusCode::from_u16(code as u16).unwrap();
            Err(format!(
                "{} {}",
                status.as_u16(),
                status.canonical_reason().unwrap()
            ))
        }
        Err(e) => Err(format!("cURL error code: {}", e)),
    }
}

fn add_authorization(easy: &mut Easy, token: &str) -> Result<(), curl::Error> {
    let mut headers = List::new();

    headers.append(format!("Authorization: Bearer {}", token).as_str())?;
    easy.http_headers(headers)?;

    Ok(())
}

fn send_request(easy: &mut Easy, url: &str) -> Result<Vec<u8>, curl::Error> {
    let mut dst = Vec::new();

    easy.url(url)?;

    {
        let mut transfer = easy.transfer();
        transfer.write_function(|data| {
            dst.extend_from_slice(data);
            Ok(data.len())
        })?;
        transfer.perform()?;
    }

    match check_response(easy) {
        Ok(_) => Ok(dst),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

pub fn authenticate(easy: &mut Easy, config: &Config) -> Result<String, curl::Error> {
    let url = format!(
        "{url}oauth/token?grant_type=client_credentials&client_id={uid}&client_secret={secret}",
        url = URL,
        uid = config.client_id,
        secret = config.secret,
    );

    easy.reset();
    easy.post(true)?;

    let response = send_request(easy, &url)?;
    Ok(serde_json::from_slice::<Auth>(&response)
        .unwrap()
        .access_token)
}

pub fn get_user(easy: &mut Easy, token: &str, login: &str) -> Result<User, curl::Error> {
    let url = format!(
        "{url}users?filter[login]={login}",
        url = BASE_API,
        login = login
    );

    easy.reset();
    add_authorization(easy, token)?;

    let response = send_request(easy, &url)?;
    let users = serde_json::from_slice::<Vec<User>>(&response).unwrap();

    if users.is_empty() {
        eprintln!("Bad login: {}", &login);
        std::process::exit(1);
    }

    Ok(users[0].clone())
}

pub fn get_locations(
    easy: &mut Easy,
    token: &str,
    user_id: u32,
    config: &Config,
) -> Result<Vec<Location>, curl::Error> {
    let url = format!(
        "{url}users/{id}/locations?range[begin_at]={start},{end}",
        url = BASE_API,
        id = user_id,
        start = &config.from,
        end = &config.to,
    );

    easy.reset();
    add_authorization(easy, token)?;

    let response = send_request(easy, &url)?;
    let locations = serde_json::from_slice::<Vec<Location>>(&response).unwrap();
    Ok(locations)
}
