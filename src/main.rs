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
    layout::{Constraint, Layout, Rect, Flex},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget, Borders, Wrap, Cell, Row, Table, Padding},
    prelude::{Alignment},
    DefaultTerminal, Frame,
};

/// Single day of weather forecast
#[derive(Serialize, Deserialize, Debug)]
struct NoaaPeriod {
    number: i32,
    name: String,
    detailedForecast: String
}

/// Contains the next 7 days of morning/night weather
#[derive(Serialize, Deserialize, Debug)]
struct NoaaProperties {
    periods: Vec<NoaaPeriod>
}

/// Forecast from NOAA
#[derive(Serialize, Deserialize, Debug)]
struct NoaaForecast {
    properties: NoaaProperties
}

/// Daily forecast data
#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoTimeAndCode {
    time: Vec<String>,
    weather_code: Vec<i32>,
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    apparent_temperature_max: Vec<f32>,
    apparent_temperature_min: Vec<f32>,
    precipitation_probability_mean: Vec<i32>
}

/// Today's weather data
#[derive(Serialize, Deserialize, Debug, Default)]
struct CurrentWeatherData {
    temperature_2m: f32,
    apparent_temperature: f32,
    weather_code: i32
}

/// Combination forecast including daily and today
#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoRawForecast {
    daily: OpenMeteoTimeAndCode,
    current: CurrentWeatherData
}

/// Single day/weather condition 
#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoPeriod {
    date: String,
    weather: String
}

/// Final, reformatted forecast with daily and current weather
#[derive(Serialize, Deserialize, Debug, Default)]
struct OpenMeteoForecast {
    periods: Vec<OpenMeteoPeriod>,
    current: CurrentWeatherData
}


fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}


#[derive(Debug, Default)]
pub struct App {
    openMeteoForecast: OpenMeteoForecast,
    todaysWeatherDescription: String,
    exit: bool
}


impl App {
    /// Runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal, forecast: OpenMeteoForecast, today: String) -> io::Result<()> {
        self.openMeteoForecast = forecast;
        self.todaysWeatherDescription = today;
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        use Constraint::{Fill, Length, Min, Percentage, Ratio};

        let vertical = Layout::vertical([Percentage(50), Percentage(50)]);
        let [today_area, forecast_area] = vertical.areas(frame.area());
        
        let weee = Layout::horizontal([Ratio(1,3), Ratio(1,3), Ratio(1,3)]);
        let [current, icon, today] = weee.areas(today_area);

        let current_weather = Layout::vertical([Ratio(1,2), Ratio(1,2)]);
        let [mut quick_stats, description] = current_weather.areas(current);


        frame.render_widget(Block::bordered().title("Today's Weather"), today_area);
        frame.render_widget(Block::bordered().title("Upcoming Week"), forecast_area);
        frame.render_widget(Block::bordered().title("Remaining Day"), today);

        frame.render_widget(
            Paragraph::new(self.todaysWeatherDescription.clone()).wrap(Wrap { trim: true }).alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Full Description")
                        .padding(Padding::uniform(1))
                )
                , description);

        let stats_widths = [
            Constraint::Length(20),
            Constraint::Length(50),
        ];

        let stats_rows = [
            Row::new(vec![
                Cell::from("Current Temp:"),//.style(styles.text_style),
                Cell::from(format!("{:15}\u{00B0}", self.openMeteoForecast.current.temperature_2m)),//.style(styles.text_style),
            ]),
            Row::new(vec![
                Cell::from("Feels Like:"),//.style(styles.text_style),
                Cell::from(format!("{:15}\u{00B0}", self.openMeteoForecast.current.apparent_temperature)),//.style(styles.text_style),
            ]),
            Row::new(vec![
                Cell::from("Weather Summary:"),//.style(styles.text_style),
                Cell::from(format!("{:50}", self.openMeteoForecast.periods[0].weather)),//.style(styles.text_style),
            ])
        ];


        let table = Table::new(stats_rows, stats_widths).column_spacing(1)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Right Now")
                );



        frame.render_widget(table, quick_stats);

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



fn getWeatherFromCode(code: String) -> String { 
    let weather_codes = readWeatherCodesFile();

    return weather_codes[code].to_string();
}




/// Get the forecast for the next 7 days as well as today's weather conditions
/// Using this API: <https://api.open-meteo.com/v1/forecast>
async fn getOpenMeteoWeather(client: &Client) -> Result<OpenMeteoForecast, Error> {
    let latitude = env::var("LATITUDE").unwrap();
    let longitude = env::var("LONGITUDE").unwrap();
    let mut timezone = env::var("TIMEZONE").unwrap().to_string();
    timezone = encode(&timezone).to_string();
    
    let mut url1 = format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=temperature_2m_max,temperature_2m_min,apparent_temperature_max,apparent_temperature_min,weather_code,precipitation_probability_mean&current=temperature_2m,apparent_temperature,weather_code&timezone={}&wind_speed_unit=mph&temperature_unit=fahrenheit&precipitation_unit=inch", latitude.to_string(), longitude.to_string(), timezone);
    let mut url = format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=weather_code&current=weather_code,is_day&timezone={}&forecast_days=14&wind_speed_unit=mph&temperature_unit=fahrenheit&precipitation_unit=inch", latitude.to_string(), longitude.to_string(), timezone);

    let response = client
        .get(url1)
        .send()
        .await?;

    let json = response.json::<OpenMeteoRawForecast>().await?;

    let mut periods: Vec<OpenMeteoPeriod> = Vec::new();
    let mut count: usize = 0;
    for i in &json.daily.time {
        periods.push(OpenMeteoPeriod{date: i.to_string(), weather: getWeatherFromCode(json.daily.weather_code[count].to_string())});
        count += 1;
    }

    return Ok(OpenMeteoForecast{periods: periods, current: json.current});
}


/// Read in the JSON file storing weather codes and associated condition
/// Taken from: <https://open-meteo.com/en/docs#weather_variable_documentation>
fn readWeatherCodesFile() -> Value {
    let data = fs::read_to_string("./weather-codes.json").expect("Error reading in weather codes file");
    let json: serde_json::Value = serde_json::from_str(&data).expect("JSON was malformed");

    //println!("{:?}", json["82"]);
    return json;
}


/// Get the morning/night weather for the next 7 days (including today)
/// Using this API: <https://api.weather.gov/>
async fn getNoaaWeatherPeriods(client: &Client) -> Result<Vec<NoaaPeriod>, Error> {
    let state = env::var("STATE").unwrap();
    let zone = env::var("ZONE").unwrap();
    let url = format!("https://api.weather.gov/zones/{}/{}/forecast", state.to_string(), zone.to_string());
    
    let response = client
        .get(url)
        .send()
        .await?;
    
    // Equivalent to:   let json: Forecast = response.json().await?;
    let json = response.json::<NoaaForecast>().await?;
    
    let periods: Vec<NoaaPeriod> = json.properties.periods;
    return Ok(periods);
}






#[tokio::main]
async fn main() -> io::Result<()> {

    dotenv().ok();

    let user_agent = "Mozilla/5.0 (X11; Linux x86_64; rv:139.0) Gecko/20100101 Firefox/139.0";
        
    let client = Client::builder()
        .user_agent(user_agent)
        .build().unwrap();

    
    let noaaPeriods = getNoaaWeatherPeriods(&client).await.unwrap();
    let today = noaaPeriods[0].detailedForecast.clone();
    // Get 14 day forecast as well as today's weather info
    let openMeteoForecast = getOpenMeteoWeather(&client).await.unwrap();
    println!("{:?}", openMeteoForecast.periods[0]);

    
    // Initialize the TUI
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal, openMeteoForecast, today);
    // Restore the terminal before we leave
    ratatui::restore();
    app_result
}
