<div align="center">
  <h1>Raijin</h1>

  <p>
    A free, simple weather TUI that pulls data without the need for an API key, account, or subscription. Weather data is from [NWS](https://api.weather.gov/) and [OpenMeteo](https://open-meteo.com/en/docs). Moon phase data is from [ViewBits](https://viewbits.com/docs/moon-phase-api-documentation).
  </p>

</div>

<div align="center">
  <img src="screenshot.png" alt="A screenshot of the application"/>
  <p>
  <sub>
  (NOTE: I'm using WezTerm with the "Gruvbox Dark (Gogh)" theme. Yours may look slightly different)
  </sub>
  </p>
</div>

<br>

## Usage

First, you'll need to get some data about your location (namely, your latitude and longitude)
- Navigate to the [NWS](https://www.weather.gov/) website
- Type in your location in the top left search bar and click `Go`
- Once the page has loaded, look up at the URL search bar at the top of your browser and jot down the latitude and longitude for this location
- Then, scroll down to the `Additional Forecasts and Information` section
- Find and click the link that says `ZONE AREA FORECAST FOR <COUNTY>, <STATE>`
- In the URL search bar at the top of your browser, you should now see a zoneId at the end of that URL. It will be in the form of `<STATE>Z123` (e.g. TNZ069 which is for Knoxville, TN; in Knox county). Jot this down

Next, you need to figure out what timezone you're in and its IANA name
- Navigate to the [AddEvent](https://www.addevent.com/c/documentation/tools/time-zone-lookup) website to look this up for free
- Type in your location using the `CITY, STATE` format (e.g. Knoxville, TN) and hit `Enter`
- Once a timezone pops up, click the green `Copy` button for that result to copy the timezone to your clipboard

Now that we have the 5 pieces of data we need (latitude, longitude, 2-letter state code, weather zone ID, and timezone), let's put them into an environment file
- Create a copy or rename the `.env.sample` file to `.env`
- Fill in the appropriate fields with the data you collected (make sure they have double-quotes around them like in the example)

<br>

## Develop
When editing the logo.txt or any of the moon phases, make sure every line has the exact same length (even if there are just blank lines). This will ensure that it can be centered and manipulated properly by Ratatui.

<br>

## Why "Raijin"?
I went googling around for mythological god names related to weather/storms. "Raijin" sounded the coolest
