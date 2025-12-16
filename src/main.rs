use axum::{body::Body, extract::State, response::Response, routing::get, Router};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::time::sleep;

mod config;

mod ha_client;

mod image_gen;

mod rtsp;

mod state;

use config::Config;

use ha_client::HaClient;

use image_gen::ImageGenerator;

use state::AppState;

#[tokio::main]

async fn main() -> anyhow::Result<()> {
    // Initialize logging (optional but good)

    // env_logger::init(); // Skipped for simplicity, can add later

    let config = Config::from_env()?;

    println!("Starting ha-sensor-streamer...");

    println!("Mode: {}", config.stream_format);

    println!("Connecting to Home Assistant at {}", config.ha_base_url);

    let sensors_to_watch = config.get_required_sensors();
    if sensors_to_watch.is_empty() {
        println!("No sensors configured to watch.");
    } else {
        println!("Watching sensors: {:?}", sensors_to_watch);
    }

    // Shared state for the latest sensor values.
    let sensor_values = Arc::new(RwLock::new(HashMap::new()));

    // Initialize components

    let ha_client = HaClient::new(&config);

    // Embed font at compile time

    let font_data = include_bytes!("../assets/Lato-Regular.ttf");

    let image_gen = Arc::new(ImageGenerator::new(
        font_data,
        config.lines.clone(),
        config.video_width,
        config.video_height,
    )?);

    // 1. Spawn Background Polling Task

    let sensor_values_clone = sensor_values.clone();
    let sensors_list = sensors_to_watch.clone();
    let ha_client_clone = ha_client.clone();

    if !sensors_list.is_empty() {
        tokio::spawn(async move {
            loop {
                for entity_id in &sensors_list {
                    match ha_client_clone.fetch_sensor_state(entity_id).await {
                        Ok(val) => {
                            if let Ok(mut lock) = sensor_values_clone.write() {
                                lock.insert(entity_id.clone(), val);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error fetching sensor state for {}: {}", entity_id, e);
                            // Optionally update state to "Error" or keep old value
                        }
                    }
                }
                sleep(Duration::from_secs(10)).await; // Poll every 10 seconds
            }
        });
    }

    let app_state = AppState {
        sensor_values,

        image_gen,

        config: config.clone(),
    };

    if config.stream_format == "rtsp" {
        // Run RTSP Server (Blocking)

        // Since GStreamer main loop is blocking, we can run it here.

        // But we are in a tokio runtime.

        // Best practice: spawn blocking task or just run it since main is async.

        // rtsp::run_rtsp_server blocks.

        tokio::task::spawn_blocking(move || {
            if let Err(e) = rtsp::run_rtsp_server(&config, app_state) {
                eprintln!("RTSP Server error: {}", e);
            }
        })
        .await?;
    } else {
        // Run MJPEG Server (Axum)

        let app = Router::new()
            .route("/stream", get(mjpeg_stream))
            .with_state(app_state);

        let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

        println!("MJPEG Server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;

        axum::serve(listener, app).await?;
    }

    Ok(())
}

// MJPEG Stream Handler

async fn mjpeg_stream(State(state): State<AppState>) -> Response {
    let fps = state.config.video_fps;

    let stream = async_stream::stream! {

        let mut interval = tokio::time::interval(Duration::from_millis(1000 / fps));



                loop {



                    interval.tick().await;







                    // Get current sensor values

            let val_map = {

                let lock = state.sensor_values.read().unwrap();

                lock.clone()

            };



            // Check if we need to regenerate (if time or sensor changed)

                        // We use a cheap formatting check for time

                        // We only need to regenerate if the *displayed* time changes.

            // Since we don't know the format logic here perfectly without duplicating image_gen logic,

            // we'll just regenerate every second (approx) or if sensor changes.

            // Actually, simplest robust way: Just generate it.

            // If FPS is 5, generating 5 JPEGs/sec of simple text is trivial for Rust.

            // Let's stick to simple generation for now to ensure correctness of custom formats (like seconds).



            // Optimization: If the user wants 30FPS, we should probably optimize.

            // For 5FPS, it's fine.



            match state.image_gen.generate_frame(&val_map) {

                Ok(jpeg_bytes) => {

                    let frame_header = format!(

                        "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",

                        jpeg_bytes.len()

                    );



                    yield Ok::<_, std::io::Error>(axum::body::Bytes::from(frame_header));

                    yield Ok(axum::body::Bytes::from(jpeg_bytes));

                    yield Ok(axum::body::Bytes::from("\r\n"));

                }

                Err(e) => {

                    eprintln!("Error generating frame: {}", e);

                }

            }

        }

    };

    let body = Body::from_stream(stream);

    Response::builder()
        .header("Content-Type", "multipart/x-mixed-replace; boundary=frame")
        .body(body)
        .unwrap()
}
