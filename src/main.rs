use reqwest::{Client, Error};
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {

    dotenv().ok();

    let STATE = env::var("STATE");
    let ZONE = env::var("ZONE");


    println!("{:?}", STATE);
    println!("{:?}", ZONE);

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:139.0) Gecko/20100101 Firefox/139.0")
        .build()?;


    println!("{:?}", format!("https://api.weather.gov/zones/{:?}/{:?}/forecast", Some(STATE.clone()), Some(ZONE.clone())));
    let response = client
        .get(format!("https://api.weather.gov/zones/{:?}/{:?}/forecast", STATE, ZONE))
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = response.json().await.unwrap();

    //println!("Status: {}", response.status());
    //println!("Body: {}", response.text().await?);
    println!("Weee: {}", json["properties"]);

    Ok(())
}
