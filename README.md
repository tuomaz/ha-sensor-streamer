# Home Assistant Sensor MJPEG Streamer (`ha-sensor-streamer`)

`ha-sensor-streamer` is a lightweight Rust application that connects to Home Assistant, fetches the state of a specified sensor, and streams it as an MJPEG video feed. This allows you to integrate sensor data and the current time into systems that consume video streams, such as [Frigate](https://frigate.video/).

The application is highly configurable, allowing you to customize the displayed date and time formats, and easily deployable via Docker.

## Features

*   **Home Assistant Integration**: Connects to HA via its REST API using a long-lived access token.
    *   **Polling**: Updates the sensor state every 10 seconds.
*   **Dynamic Overlay**: Overlays current date, time, and a chosen sensor's value onto a black background.
    *   **Sensor Display**: Automatically appends a degree symbol (°) to the sensor value, making it ideal for temperature sensors.
*   **Customizable Display**: Date and time formats are configurable via environment variables.
*   **MJPEG Stream**: Serves a standard MJPEG video stream (e.g., `http://localhost:3000/stream`).
*   **Native RTSP Stream**: Serves a low-latency H.264 RTSP stream using GStreamer.
    *   Uses `x264enc` with `speed-preset=ultrafast` and `tune=zerolatency` for minimal delay.
*   **Dockerized**: Provided `Dockerfile` and `docker-compose.yml` for easy deployment.
*   **Built with Rust**: High performance, memory safety, and minimal resource usage.

## Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing purposes.

### Prerequisites

*   [Rust toolchain](https://www.rust-lang.org/tools/install) (if building from source)
*   [Docker](https://docs.docker.com/get-docker/) (for containerized deployment)
*   A Home Assistant instance with a long-lived access token.

### Installation

#### Build from Source

1.  Clone the repository:
    ```bash
    git clone https://github.com/tuomaz/ha-sensor-streamer.git
    cd ha-sensor-streamer
    ```
2.  Build the project in release mode:
    ```bash
    cargo build --release
    ```
    The executable will be located at `./target/release/ha-sensor-streamer`.

#### Using Docker (Recommended for Deployment)

The easiest way to run `ha-sensor-streamer` is using its Docker image from the GitHub Container Registry.

```bash
# Pull the latest image
docker pull ghcr.io/tuomaz/ha-sensor-streamer:main
```

## Usage

### Configuration

The application is configured using environment variables:

*   `HA_BASE_URL` (Required): The base URL of your Home Assistant instance (e.g., `http://192.168.1.100:8123`).
*   `HA_LONG_LIVED_TOKEN` (Required): Your Home Assistant long-lived access token.
*   `SENSOR_ENTITY_ID` (Required): The entity ID of the sensor you want to display (e.g., `sensor.ute_kombinerad`).
*   `PORT` (Optional): The port the application listens on (default: `8080`).
*   `DATE_FORMAT` (Optional): The format string for the displayed date (default: `%Y-%m-%d`).
*   `TIME_FORMAT` (Optional): The format string for the displayed time (default: `%H.%M`).
    *   For `DATE_FORMAT` and `TIME_FORMAT` specifiers, refer to the [chrono::strftime documentation](https://docs.rs/chrono/latest/chrono/format/strftime/index.html#specifiers).
*   `VIDEO_WIDTH` (Optional): The width of the video stream (default: `640`).
*   `VIDEO_HEIGHT` (Optional): The height of the video stream (default: `360`).
*   `TZ` (Optional): Set the timezone for the displayed time (e.g., `Europe/Berlin`, `America/New_York`).
    *   Requires `tzdata` package installed in the Docker image. Refer to `/usr/share/zoneinfo` for available timezones.
*   `VIDEO_FPS` (Optional): The frame rate of the video (default: `5`). Increasing this helps with RTSP stability.
*   `STREAM_FORMAT` (Optional): The streaming mode. Options: `mjpeg` (default) or `rtsp`.
    *   `mjpeg`: Serves an HTTP MJPEG stream at `/stream`.
    *   `rtsp`: Serves a native H.264 RTSP stream at `rtsp://0.0.0.0:PORT/stream`.

### Running Locally

1.  Set the required environment variables in your shell:
    ```bash
    export HA_BASE_URL="http://YOUR_HA_IP:8123"
    export HA_LONG_LIVED_TOKEN="YOUR_LONG_LIVED_TOKEN"
    export SENSOR_ENTITY_ID="sensor.ute_kombinerad"
    # Optional:
    export PORT="8080"
    export DATE_FORMAT="%Y-%m-%d"
    export TIME_FORMAT="%H.%M"
    ```
2.  Run the compiled executable:
    ```bash
    ./target/release/ha-sensor-streamer
    ```

### Running with Docker Compose

A `docker-compose.yml` file is provided for easy setup:

1.  Copy the example `docker-compose.yml` file:
    ```bash
    cp docker-compose.yml docker-compose.local.yml # Or rename to just docker-compose.yml
    ```
2.  Edit the `docker-compose.local.yml` file and replace the placeholder values for `HA_BASE_URL`, `HA_LONG_LIVED_TOKEN`, and `SENSOR_ENTITY_ID` with your actual Home Assistant details.
3.  Start the service:
    ```bash
    docker compose -f docker-compose.local.yml up -d
    ```

### Viewing the Stream

Once the application is running (locally or via Docker), you can view the stream:

*   **MJPEG Stream**: Open `http://localhost:8080/stream` (if configured, adjust port if changed).
*   **RTSP Stream**: Open `rtsp://localhost:8080/stream` (if configured, adjust port if changed).
*   **In VLC Media Player**: Go to `Media > Open Network Stream...` and enter the URL.

### Native H.264 RTSP Integration (Recommended)

The most efficient way to use this application is to enable the native H.264 RTSP server. This avoids the need for transcoding MJPEG to H.264 in go2rtc/ffmpeg, saving CPU resources.

1.  **Enable RTSP Mode**: Set the environment variable `STREAM_FORMAT=rtsp`.
2.  **Configure go2rtc**: Add the stream directly to `go2rtc.yaml`.

    ```yaml
    streams:
      ha_dashboard:
        - rtsp://YOUR_DOCKER_HOST_IP:8080/stream
    ```

3.  **Configure Frigate**: Consume the stream from go2rtc.

    ```yaml
    cameras:
      ha_dashboard:
        ffmpeg:
          inputs:
            - path: rtsp://127.0.0.1:8554/ha_dashboard
              input_args: preset-rtsp-restream
              roles:
                - detect
        detect:
          width: 640
          height: 360
          fps: 5 # Match your app's VIDEO_FPS (default 5)
    ```

### Direct Frigate Integration (MJPEG without go2rtc)

To add this as a camera in your `frigate.yml` without using `go2rtc` for transcoding, use the following configuration. This method consumes the raw MJPEG stream directly.

```yaml
cameras:
  ha_dashboard:
    ffmpeg:
      inputs:
        - path: http://YOUR_DOCKER_HOST_IP:8080/stream
          input_args: -avoid_negative_ts make_zero -fflags nobuffer -flags low_delay -strict experimental -f mjpeg
          roles:
            - detect
    detect:
      width: 640
      height: 360
      fps: 1
      objects:
        track: [] # Disable object tracking for this camera
```

### Transcoding MJPEG to H.264 via go2rtc

If you prefer to use the MJPEG output but need H.264 for Frigate/HomeKit, you can use `go2rtc` to transcode it.

#### 1. Add `ha-sensor-streamer` to `go2rtc.yaml`

Modify your `go2rtc.yaml` (or the `go2rtc` section if embedded in `frigate.yml`) to include the stream with H.264 transcoding and pixel format conversion:

```yaml
streams:
  # ... other streams ...
  ha_dashboard:
    # 'ffmpeg:' triggers transcoding.
    # '#input=mjpeg' hints ffmpeg about the source format.
    # '#video=h264' forces output to H.264.
    # '#vf=format=yuv420p' converts pixel format for Frigate compatibility.
    # '#raw=-re' ensures ffmpeg reads input at native framerate (important for slow streams).
    - "ffmpeg:http://YOUR_DOCKER_HOST_IP:8080/stream#input=mjpeg#video=h264#vf=format=yuv420p#raw=-re"
    # Optional: Add hardware acceleration if your system supports it and you have it configured in go2rtc
    # Example for Intel VAAPI: - "ffmpeg:http://YOUR_DOCKER_HOST_IP:8080/stream#input=mjpeg#video=h264#vf=format=yuv420p#raw=-re#hardware=vaapi"
```

#### 2. Update `frigate.yml` to use the `go2rtc` stream

Once `go2rtc` is providing the H.264 stream, configure Frigate to pick it up from `go2rtc`. This uses standard RTSP arguments for better compatibility.

```yaml
cameras:
  ha_dashboard:
    ffmpeg:
      inputs:
        # Connect to the transcoded stream via go2rtc's RTSP endpoint
        - path: rtsp://127.0.0.1:8554/ha_dashboard?mp4
          input_args: preset-rtsp-restream # Use Frigate's built-in preset for go2rtc streams
          roles:
            - detect # Required for live view decoding
    detect:
      width: 640 # Must match your VIDEO_WIDTH
      height: 360 # Must match your VIDEO_HEIGHT
      fps: 1 # Match your app's FPS
      objects:
        track: [] # Disable object tracking for this camera
```
## Contributing

Feel free to open issues or submit pull requests on the [GitHub repository](https://github.com/tuomaz/ha-sensor-streamer).

## Acknowledgements

This project was developed with significant assistance from Google Gemini.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

The Lato font used in this project is licensed under the [SIL Open Font License, Version 1.1](LICENSE-FONT).
