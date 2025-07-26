use chrono::{Datelike, NaiveDate};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use dirs;
use dotenv;
use include_dir::{Dir, include_dir};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    prelude::Alignment,
    style::{Color, Style, Stylize},
    symbols::Marker,
    text::{Line, Text},
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, GraphType, Padding, Paragraph, Row, Table,
    },
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, fs, io};
use ureq::Agent;
use urlencoding::encode;

static MOON_PHASE_ART_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/moon-phase-art");

/// Daily forecast data
#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoTimeAndCode {
    time: Vec<String>,
    weather_code: Vec<i32>,
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    apparent_temperature_max: Vec<f32>,
    apparent_temperature_min: Vec<f32>,
    precipitation_probability_mean: Vec<i32>,
}

/// Raw hourly data
#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoHourlyData {
    time: Vec<String>,
    weather_code: Vec<i32>,
    temperature_2m: Vec<f32>,
}

/// Today's weather data
#[derive(Serialize, Deserialize, Debug, Default)]
struct CurrentWeatherData {
    temperature_2m: f32,
    apparent_temperature: f32,
    weather_code: i32,
}

/// Combination forecast including daily, hourly, and current
#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoRawForecast {
    daily: OpenMeteoTimeAndCode,
    hourly: OpenMeteoHourlyData,
    current: CurrentWeatherData,
}

/// Single day/weather condition
/// NOTE: Temperature values already include a degree symbol (U+00B0)
#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoPeriod {
    date: String,
    weather: String,
    temperature_max: String,
    temperature_min: String,
    apparent_temperature_max: String,
    apparent_temperature_min: String,
    precipitation_probability: String,
}

/// Forecast data by the hour
#[derive(Serialize, Deserialize, Debug)]
struct OpenMeteoHourly {
    datetime: String,
    temperature: String,
    weather: String,
}

/// Final, reformatted forecast with daily and current weather
#[derive(Serialize, Deserialize, Debug, Default)]
struct OpenMeteoForecast {
    periods: Vec<OpenMeteoPeriod>,
    current: CurrentWeatherData,
    hourly: Vec<OpenMeteoHourly>,
}

/// Moon phase data for a given date
#[derive(Serialize, Deserialize, Debug)]
struct MoonPhase {
    date: String,
    phase: String,
    illumination: String,
}

/// Raw phase data from ViewBits
#[derive(Serialize, Deserialize, Debug)]
struct RawMoonPhaseData {
    phases: Vec<MoonPhase>,
}

/// Create the "Right Now" weather table
fn create_right_now_table(forecast: &OpenMeteoForecast) -> Table {
    let widths = [Constraint::Length(15), Constraint::Fill(1)];

    let rows = [
        Row::new(vec![
            Cell::from("Current Temp:"),
            Cell::from(
                Text::from(format!("{}\u{00B0}", forecast.current.temperature_2m)).right_aligned(),
            ),
        ]),
        Row::new(vec![
            Cell::from("Feels Like:"),
            Cell::from(
                Text::from(format!("{}\u{00B0}", forecast.current.apparent_temperature))
                    .right_aligned(),
            ),
        ]),
        Row::new(vec![
            Cell::from("High:"),
            Cell::from(
                Text::from(format!("{}", forecast.periods[0].temperature_max)).right_aligned(),
            ),
        ]),
        Row::new(vec![
            Cell::from("Low:"),
            Cell::from(
                Text::from(format!("{}", forecast.periods[0].temperature_min)).right_aligned(),
            ),
        ]),
        Row::new(vec![
            Cell::from("Weather Summary:"),
            Cell::from(Text::from(format!("{}", forecast.periods[0].weather)).right_aligned()),
        ]),
        Row::new(vec![
            Cell::from("Chance of Rain:"),
            Cell::from(
                Text::from(format!(
                    "{}%",
                    forecast.periods[0].precipitation_probability
                ))
                .right_aligned(),
            ),
        ]),
    ];

    return Table::new(rows, widths).column_spacing(1).block(
        Block::default()
            .borders(Borders::ALL)
            .padding(Padding::new(1, 1, 2, 1)) //uniform(1))
            .title(Line::from(" Right Now ").light_blue().centered().bold()),
    );
}

/// Renders the scatterplot to show the temperature for the rest of the current day
fn render_temperature_scatterplot(frame: &mut Frame, area: Rect, hourly: &Vec<OpenMeteoHourly>) {
    let mut today_hourly: [(f64, f64); 24] = [(0., 0.); 24];
    let mut count: usize = 0;
    for i in hourly {
        let time_split = i.datetime.split("T");
        let time_pieces = time_split.collect::<Vec<_>>();
        let hour_split = time_pieces[1].split(":");
        let hour_pieces = hour_split.collect::<Vec<_>>();
        let hour_as_float = hour_pieces[0].parse::<f64>().unwrap();
        let mut temp_clone = i.temperature.clone();
        temp_clone.pop();
        let temp_as_float = temp_clone.parse::<f64>().unwrap();
        today_hourly[count] = (hour_as_float, temp_as_float);
        count += 1;
        if count == 24 {
            break;
        }
    }

    let temps: Vec<f64> = today_hourly.iter().map(|(_, temp)| *temp).collect();
    let min_temp = temps.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_temp = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let y_min = (min_temp - 5.0).floor();
    let y_max = (max_temp + 5.0).ceil();

    let step = (y_max - y_min) / 4.0;
    let y_labels = (0..5)
        .map(|i| format!("{:.0}", y_min + i as f64 * step))
        .collect::<Vec<_>>();

    let dataset = Dataset::default()
        .marker(Marker::Dot)
        .graph_type(GraphType::Scatter)
        .style(Style::new().yellow())
        .data(&today_hourly);

    let chart = Chart::new(vec![dataset])
        .block(Block::bordered().title(Line::from(" Today's Temps ").cyan().centered().bold()))
        .y_axis(
            Axis::default()
                .title("Temp (\u{00B0}C)")
                .bounds([y_min, y_max])
                .style(Style::default().fg(Color::Gray))
                .labels(y_labels),
        )
        .x_axis(
            Axis::default()
                .title("Time (HH:MM)")
                .bounds([0., 23.])
                .style(Style::default().fg(Color::Gray))
                .labels(["00:00", "06:00", "12:00", "18:00", "23:00"]),
        )
        .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));

    frame.render_widget(chart, area);
}

fn render_fortnight_scatterplot(
    frame: &mut Frame,
    area: Rect,
    hourly: &Vec<OpenMeteoHourly>,
    daily: &Vec<OpenMeteoPeriod>,
) {
    const DATA_LENGTH: usize = 336;

    let mut fortnight_hourly: [(f64, f64); DATA_LENGTH] = [(0., 0.); DATA_LENGTH];
    let mut count: usize = 0;
    for i in hourly {
        let mut temp_clone = i.temperature.clone();
        temp_clone.pop();
        let temp_as_float = temp_clone.parse::<f64>().unwrap();
        // Scale the x-coordinate to fit within 0..336 for 14 days of hourly data
        let x_position = count as f64;
        fortnight_hourly[count] = (x_position, temp_as_float);
        count += 1;
    }

    let days: Vec<String> = daily
        .iter()
        .map(|l| {
            let parts: Vec<&str> = l.date.split('-').collect();
            if parts.len() == 3 {
                format!("{}-{}", parts[1], parts[2]) // MM-DD
            } else {
                l.date.clone() // fallback in case the format is unexpected
            }
        })
        .collect();

    let x_labels: Vec<Line> = (0..14) // 14 days in total
        .map(|i| {
            let day = &days[i];
            Line::from(day.as_str())
        })
        .collect();

    let temps: Vec<f64> = fortnight_hourly.iter().map(|(_, temp)| *temp).collect();
    let min_temp = temps.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_temp = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let y_min = (min_temp - 5.0).floor();
    let y_max = (max_temp + 5.0).ceil();

    let step = (y_max - y_min) / 4.0;
    let y_labels = (0..5)
        .map(|i| format!("{:.0}", y_min + i as f64 * step))
        .collect::<Vec<_>>();

    let dataset = Dataset::default()
        .marker(Marker::Dot)
        .graph_type(GraphType::Scatter)
        .style(Style::new().yellow())
        .data(&fortnight_hourly);

    let chart = Chart::new(vec![dataset])
        .block(Block::bordered().title(Line::from(" Fortnights's Temps ").cyan().centered().bold()))
        .y_axis(
            Axis::default()
                .title("Temp (\u{00B0}C)")
                .bounds([y_min, y_max])
                .style(Style::default().fg(Color::Gray))
                .labels(y_labels),
        )
        .x_axis(
            Axis::default()
                .title("Days")
                .bounds([0., DATA_LENGTH as f64])
                .style(Style::default().fg(Color::Gray))
                .labels(x_labels),
        )
        .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));

    frame.render_widget(chart, area);
}

/// Creates the cards for the 4-cast section
fn create_weather_card(period: &OpenMeteoPeriod) -> Table {
    let widths = [Constraint::Length(15), Constraint::Fill(1)];

    let rows = [
        Row::new(vec![
            Cell::from("High:"),
            Cell::from(Text::from(format!("{}", period.temperature_max)).right_aligned()),
        ]),
        Row::new(vec![
            Cell::from("Apparent High:"),
            Cell::from(Text::from(format!("{}", period.apparent_temperature_max)).right_aligned()),
        ]),
        Row::new(vec![
            Cell::from("Low:"),
            Cell::from(Text::from(format!("{}", period.temperature_min)).right_aligned()),
        ]),
        Row::new(vec![
            Cell::from("Apparent Low:"),
            Cell::from(Text::from(format!("{}", period.apparent_temperature_min)).right_aligned()),
        ]),
        Row::new(vec![
            Cell::from("Weather:"),
            Cell::from(Text::from(format!("{}", period.weather)).right_aligned()),
        ]),
        Row::new(vec![
            Cell::from("Chance of Rain:"),
            Cell::from(
                Text::from(format!("{}%", period.precipitation_probability)).right_aligned(),
            ),
        ]),
    ];

    let day = get_day_from_date(&period.date);

    return Table::new(rows, widths).column_spacing(1).block(
        Block::default()
            .borders(Borders::ALL)
            .padding(Padding::new(0, 0, 1, 0)) //(2,2,3,0))
            .title(
                Line::from(format!(" ({}) {} ", day, period.date))
                    .centered()
                    .bold(),
            ),
    );
}

/// Returns day (Monday, Tuesday, etc) for given date (YYYY-MM-DD)
fn get_day_from_date(date: &String) -> String {
    let date_pieces: Vec<&str> = date.split('-').collect();
    // Parse the day, month, and year as integers
    let year: i32 = date_pieces[0].parse().expect("Invalid year");
    let month: u32 = date_pieces[1].parse().expect("Invalid month");
    let day: u32 = date_pieces[2].parse().expect("Invalid day");

    // Create a NaiveDate object with the provided input using from_ymd_opt
    match NaiveDate::from_ymd_opt(year, month, day) {
        Some(date) => {
            return date.weekday().to_string();
        }
        None => {
            println!("Invalid date provided. Please ensure the date is valid.");
            return String::from("");
        }
    }
}

/// Application state data
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct App {
    open_meteo_forecast: OpenMeteoForecast,
    todays_weather_description: String,
    moon_phase_art: String,
    exit: bool,
}

/// Main Ratatui app for Raijin
impl App {
    /// Runs the application's main loop until the user quits
    fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        forecast: OpenMeteoForecast,
        moon_phase_art: String,
    ) -> io::Result<()> {
        self.open_meteo_forecast = forecast;
        self.moon_phase_art = moon_phase_art;
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        use Constraint::{Percentage, Ratio};

        // Not the best naming; change when better ideas
        let vertical = Layout::vertical([Percentage(70), Percentage(30)]);
        let [today_area, forecast_area] = vertical.areas(frame.area());

        let horizontal = Layout::horizontal([Ratio(2, 3), Ratio(1, 3)]);
        let [top_left, top_right] = horizontal.areas(today_area);

        let top = Layout::vertical([Percentage(40), Percentage(60)]);
        let [today_info, fortnight_graph] = top.areas(top_left);

        let top2 = Layout::vertical([Ratio(3, 4), Ratio(1, 4)]);
        let [today, logo_area] = top2.areas(top_right);

        let topest = Layout::horizontal([Ratio(1, 2), Ratio(1, 2)]);
        let [quick_stats, mid_top] = topest.areas(today_info);

        let outer_block = Block::bordered()
            .title(Line::from(" 4-cast ").light_magenta().centered().bold())
            .padding(Padding::new(0, 0, 1, 0));
        let inner_block = Block::bordered();
        let inner_area = outer_block.inner(forecast_area);

        let upcoming_weather =
            Layout::horizontal([Ratio(1, 4), Ratio(1, 4), Ratio(1, 4), Ratio(1, 4)]);
        let [slot1, slot2, slot3, slot4] = upcoming_weather.areas(inner_area);

        frame.render_widget(outer_block, forecast_area);
        frame.render_widget(inner_block, inner_area);

        frame.render_widget(Block::bordered(), mid_top);
        frame.render_widget(Block::new(), fortnight_graph);

        // Render the current moon phase for tonight (they store the current moon phase in the
        // third position):
        frame.render_widget(
            Paragraph::new(self.moon_phase_art.clone())
                .alignment(Alignment::Center)
                .block(
                    Block::new().title(
                        Line::from(" Tonight's Moon Phase ")
                            .light_yellow()
                            .centered()
                            .bold(),
                    ),
                ),
            mid_top,
        );

        // Render the logo into the middle of the screen
        let logo = include_str!("./logo.txt");
        frame.render_widget(
            Paragraph::new(logo)
                .alignment(Alignment::Center)
                .style(Style::new().red()),
            logo_area,
        );

        render_fortnight_scatterplot(
            frame,
            fortnight_graph,
            &self.open_meteo_forecast.hourly,
            &self.open_meteo_forecast.periods,
        );

        // Render forecast summary details for right now
        frame.render_widget(
            create_right_now_table(&self.open_meteo_forecast),
            quick_stats,
        );
        render_temperature_scatterplot(frame, today, &self.open_meteo_forecast.hourly);

        // Populate the 4-cast
        for i in 1..5 {
            let mut render_area: Rect = slot1;
            if i == 2 {
                render_area = slot2;
            } else if i == 3 {
                render_area = slot3;
            } else if i == 4 {
                render_area = slot4;
            }
            frame.render_widget(
                create_weather_card(&self.open_meteo_forecast.periods[i]),
                render_area,
            );
        }
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

/// Get the forecast for the next 7 days as well as today's weather conditions
/// Using this API: <https://api.open-meteo.com/v1/forecast>
fn get_open_meteo_weather(
    agent: &Agent,
    weather_codes: serde_json::Value,
) -> Result<OpenMeteoForecast, ureq::Error> {
    let latitude = env::var("LATITUDE").unwrap();
    let longitude = env::var("LONGITUDE").unwrap();
    let mut timezone = env::var("TIMEZONE").unwrap().to_string();
    timezone = encode(&timezone).to_string();

    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=temperature_2m_max,temperature_2m_min,apparent_temperature_max,apparent_temperature_min,weather_code,precipitation_probability_mean&hourly=temperature_2m,weather_code&current=temperature_2m,apparent_temperature,weather_code&timezone={}&forecast_days=14",
        latitude.to_string(),
        longitude.to_string(),
        timezone
    );

    let json = agent
        .get(url)
        .call()?
        .body_mut()
        .read_json::<OpenMeteoRawForecast>()?;

    let mut periods: Vec<OpenMeteoPeriod> = Vec::new();
    let mut count: usize = 0;
    for i in &json.daily.time {
        periods.push(OpenMeteoPeriod {
            date: i.to_string(),
            weather: weather_codes[json.daily.weather_code[count].to_string()].to_string(),
            temperature_max: format!(
                "{}\u{00B0}",
                json.daily.temperature_2m_max[count].to_string()
            ),
            temperature_min: format!(
                "{}\u{00B0}",
                json.daily.temperature_2m_min[count].to_string()
            ),
            apparent_temperature_max: format!(
                "{}\u{00B0}",
                json.daily.apparent_temperature_max[count].to_string()
            ),
            apparent_temperature_min: format!(
                "{}\u{00B0}",
                json.daily.apparent_temperature_min[count].to_string()
            ),
            precipitation_probability: json.daily.precipitation_probability_mean[count].to_string(),
        });
        count += 1;
    }

    let mut hourly: Vec<OpenMeteoHourly> = Vec::new();
    let mut count: usize = 0;
    for i in &json.hourly.time {
        hourly.push(OpenMeteoHourly {
            datetime: i.to_string(),
            temperature: format!("{}\u{00B0}", json.hourly.temperature_2m[count].to_string()),
            weather: weather_codes[json.hourly.weather_code[count].to_string()].to_string(),
        });
        count += 1;
    }

    return Ok(OpenMeteoForecast {
        periods: periods,
        current: json.current,
        hourly: hourly,
    });
}

/// Get the phases of the moon for today and the next 3 days
/// Using this API: <https://api.viewbits.com/v1/moonphase>
fn get_moon_phases(agent: &Agent, date: String) -> Result<Vec<MoonPhase>, ureq::Error> {
    let url = format!("https://api.viewbits.com/v1/moonphase?startdate={}", date);
    let moon_phases = agent
        .get(url)
        .call()?
        .body_mut()
        .read_json::<Vec<MoonPhase>>()?;

    return Ok(moon_phases);
}

fn main() -> io::Result<()> {
    let folder: PathBuf = dirs::home_dir()
        .expect("Could not find home directory")
        .join(".config")
        .join("Raijin");

    let file = folder.join(".env");

    if !folder.exists() {
        fs::create_dir(folder)?;
    }

    if !file.exists() {
        fs::write(
            &file,
            "ZONE=\"TNZ069\"\nSTATE=\"TN\"\nLATITUDE=\"35.9626444\"\nLONGITUDE=\"-83.9167239\"\nTIMEZONE=\"America/New_York\"\n",
        )?;
    }

    let _ = dotenv::from_path(&file).expect("Could not find .env file");

    let data = include_str!("./weather-codes.json");
    let weather_codes: serde_json::Value = serde_json::from_str(&data).expect("JSON was malformed");

    // This is used as part of the thin authentication that the NWS API uses
    // I'm hardcoding it because it doesn't really matter and you won't get blocked even with heavy
    // use (I pinged this thing constantly during development and never hit a limit)
    let user_agent = "Mozilla/5.0 (X11; Linux x86_64; rv:139.0) Gecko/20100101 Firefox/139.0";

    let config = Agent::config_builder()
        .user_agent(user_agent)
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build();

    let agent = Agent::new_with_config(config);

    let open_meteo_forecast = get_open_meteo_weather(&agent, weather_codes).unwrap();
    let all_moon_phases =
        get_moon_phases(&agent, open_meteo_forecast.periods[0].date.clone()).unwrap();

    let thing = MOON_PHASE_ART_DIR
        .get_file(format!("{}.txt", all_moon_phases[3].phase))
        .unwrap();
    let moon_phase_art = thing.contents_utf8().unwrap();

    // Initialize the TUI
    let mut terminal = ratatui::init();
    let app_result = App::default().run(
        &mut terminal,
        open_meteo_forecast,
        moon_phase_art.to_string(),
    );
    // Restore the terminal before we leave
    ratatui::restore();
    app_result
}
