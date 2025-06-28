use reqwest::{Client, Error};
use dotenv::dotenv;
use std::env;
use serde_json::Value;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use urlencoding::encode;
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
struct Period {
    number: i32,
    name: String,
    detailedForecast: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Properties {
    periods: Vec<Period>
}

#[derive(Serialize, Deserialize, Debug)]
struct Forecast {
    properties: Properties
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoTimeAndCode {
    time: Vec<String>,
    weather_code: Vec<i32>
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoForecast {
    daily: OpenMeteoTimeAndCode
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoPeriod {
    date: String,
    weather_code: String
}



async fn getOpenMeteoWeather(client: &Client) -> Result<Vec<OpenMeteoPeriod>, Error> {
    let latitude = env::var("LATITUDE").unwrap();
    let longitude = env::var("LONGITUDE").unwrap();
    let mut timezone = env::var("TIMEZONE").unwrap().to_string();
    timezone = encode(&timezone).to_string();

    let mut url = format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=weather_code&current=weather_code,is_day&timezone={}&forecast_days=14&wind_speed_unit=mph&temperature_unit=fahrenheit&precipitation_unit=inch", latitude.to_string(), longitude.to_string(), timezone);

    let response = client
        .get(url)
        .send()
        .await?;

    let json = response.json::<OpenMeteoForecast>().await?;

    let mut periods: Vec<OpenMeteoPeriod> = Vec::new();
    let mut count: usize = 0;
    for i in &json.daily.time {
        periods.push(OpenMeteoPeriod{date: i.to_string(), weather_code: json.daily.weather_code[count].to_string()});
        count += 1;
    }

    return Ok(periods);
}



fn readWeatherCodesFile() -> Value {
    let data = fs::read_to_string("./weather-codes.json").expect("Error reading in weather codes file");
    let json: serde_json::Value = serde_json::from_str(&data).expect("JSON was malformed");

    //println!("{:?}", json["82"]);
    return json;
}



async fn getNoaaWeatherPeriods(client: &Client) -> Result<Vec<Period>, Error> {
    let state = env::var("STATE").unwrap();
    let zone = env::var("ZONE").unwrap();
    let url = format!("https://api.weather.gov/zones/{}/{}/forecast", state.to_string(), zone.to_string());
    
    let response = client
        .get(url)
        .send()
        .await?;
    
    // Equivalent to:   let json: Forecast = response.json().await?;
    let json = response.json::<Forecast>().await?;
    
    let periods: Vec<Period> = json.properties.periods;
    return Ok(periods);
}



#[tokio::main]
async fn main() -> Result<(), Error> {

    dotenv().ok();

    let weather_codes = readWeatherCodesFile();

    let user_agent = "Mozilla/5.0 (X11; Linux x86_64; rv:139.0) Gecko/20100101 Firefox/139.0";
        
    let client = Client::builder()
        .user_agent(user_agent)
        .build()?;


    //let periods = getNoaaWeatherPeriods(&client).await.unwrap();
    let periods = getOpenMeteoWeather(&client).await.unwrap();


    for i in &periods {
        println!("{i:?}");
    }

    Ok(())
}
