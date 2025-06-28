use reqwest::{Client, Error};
use dotenv::dotenv;
use serde_json::Value;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use urlencoding::encode;
use std::{fs, io, env};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};


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

#[derive(Debug, Default)]
pub struct App {
    exit: bool
}


impl App {
    /// Runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        use Constraint::{Fill, Length, Min};

        let vertical = Layout::vertical([Length(1), Min(0),Length(1)]);
        let [title_area, main_area, status_area] = vertical.areas(frame.area());
        let horizontal = Layout::horizontal([Fill(1); 2]);
        let [left_area, right_area] = horizontal.areas(main_area);

        frame.render_widget(Block::bordered().title("Title Bar"), title_area);
        frame.render_widget(Block::bordered().title("Status Bar"), status_area);
        frame.render_widget(Block::bordered().title("Left"), left_area);
        frame.render_widget(Block::bordered().title("Right"), right_area);
    }

    /// Updates the application's state based on user input
    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
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
async fn main() -> io::Result<()> {

    dotenv().ok();

    let weather_codes = readWeatherCodesFile();

    let user_agent = "Mozilla/5.0 (X11; Linux x86_64; rv:139.0) Gecko/20100101 Firefox/139.0";
        
    let client = Client::builder()
        .user_agent(user_agent)
        .build().unwrap();


    //let periods = getNoaaWeatherPeriods(&client).await.unwrap();
    let periods = getOpenMeteoWeather(&client).await.unwrap();


    for i in &periods {
        println!("{i:?}");
    }

    
    // Initialize the TUI
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    // Restore the terminal before we leave
    ratatui::restore();
    app_result
}
