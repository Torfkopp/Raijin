use reqwest::{Client, Error};
use dotenv::dotenv;
use std::env;
use serde_json::Value;
use std::collections::HashMap;


#[derive(Debug)]
struct ForecastPeriod {
    number: u8,
    name: String,
    detailedForecast: String
}



#[tokio::main]
async fn main() -> Result<(), Error> {

    dotenv().ok();

    let STATE = env::var("STATE").unwrap();
    let ZONE = env::var("ZONE").unwrap();

    let url = format!("https://api.weather.gov/zones/{}/{}/forecast", STATE.to_string(), ZONE.to_string());

    let user_agent = "Mozilla/5.0 (X11; Linux x86_64; rv:139.0) Gecko/20100101 Firefox/139.0";
        
    println!("{:?}", STATE);
    println!("{:?}", ZONE);
    println!("{:?}", url);

    let client = Client::builder()
        .user_agent(user_agent)
        .build()?;

    let response = client
        .get(url)
        .send()
        .await?;

    //let json2: Vec<ForecastPeriod> = serde_json.from_str(response.text().await)?;
    let json: Vec<HashMap<String, Value>> = serde_json::from_str(&response.text().await.unwrap()).unwrap();

    //response.json().await.unwrap();
    //let periods = json.properties.clone();
    println!("{:#?}", json);
    //let periods: Vec<ForecastPeriod> = Vec::new();
    //assert!(periods.is_array());
    //for i in periods {
        //periods.push(i);
    //    println!("{:#?}", i);
    //}

    //println!("wooo: {:#?}", json2["properties"]);
    //println!("Status: {}", response.status());
    //println!("Body: {}", response.text().await?);
    //println!("Weee: {:#?}", json["properties"]);

    Ok(())
}
