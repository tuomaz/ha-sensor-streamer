use anyhow::Result;
use regex::Regex;
use std::collections::HashSet;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub ha_base_url: String,
    pub ha_token: String,
    pub port: u16,
    pub video_width: u32,
    pub video_height: u32,
    pub video_fps: u64,
    pub stream_format: String,
    pub lines: Vec<String>,
    pub font_size: f32,
    pub locale: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let ha_base_url = env::var("HA_BASE_URL").expect("HA_BASE_URL must be set");
        let ha_token = env::var("HA_LONG_LIVED_TOKEN").expect("HA_LONG_LIVED_TOKEN must be set");
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .expect("PORT must be a number");
        let video_width = env::var("VIDEO_WIDTH")
            .unwrap_or_else(|_| "640".to_string())
            .parse()
            .expect("VIDEO_WIDTH must be a number");
        let video_height = env::var("VIDEO_HEIGHT")
            .unwrap_or_else(|_| "360".to_string())
            .parse()
            .expect("VIDEO_HEIGHT must be a number");
        let video_fps = env::var("VIDEO_FPS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .expect("VIDEO_FPS must be a number");
        let stream_format = env::var("STREAM_FORMAT")
            .unwrap_or_else(|_| "mjpeg".to_string())
            .to_lowercase();
        let font_size = env::var("FONT_SIZE")
            .unwrap_or_else(|_| "48.0".to_string())
            .parse()
            .expect("FONT_SIZE must be a number");
        let locale = env::var("LOCALE").unwrap_or_else(|_| "en_US".to_string());

        // Ensure base URL doesn't end with slash for cleaner path joining
        let ha_base_url = if ha_base_url.ends_with('/') {
            ha_base_url[..ha_base_url.len() - 1].to_string()
        } else {
            ha_base_url
        };

        // Parse Lines
        let mut lines = Vec::new();
        let mut has_line_config = false;

        for i in 1..=4 {
            if let Ok(line) = env::var(format!("LINE_{}", i)) {
                if !line.is_empty() {
                    lines.push(line);
                    has_line_config = true;
                }
            }
        }

        // Fallback to old config if no lines are defined
        if !has_line_config {
            let date_format = env::var("DATE_FORMAT").unwrap_or_else(|_| "%Y-%m-%d".to_string());
            let time_format = env::var("TIME_FORMAT").unwrap_or_else(|_| "%H.%M".to_string());
            let sensor_entity_id = env::var("SENSOR_ENTITY_ID")
                .unwrap_or_else(|_| "sensor.ute_kombinerad".to_string());

            lines.push(format!("{{time:{}}}", date_format));
            lines.push(format!("{{time:{}}}", time_format));
            lines.push(format!("{{sensor.{}}}°", sensor_entity_id));
        }

        Ok(Config {
            ha_base_url,
            ha_token,
            port,
            video_width,
            video_height,
            video_fps,
            stream_format,
            lines,
            font_size,
            locale,
        })
    }

    /// Extracts unique sensor entity IDs from the configured lines.
    pub fn get_required_sensors(&self) -> Vec<String> {
        let re = Regex::new(r"\{sensor\.([\w\.]+)\}").expect("Invalid regex");
        let mut sensors = HashSet::new();

        for line in &self.lines {
            for cap in re.captures_iter(line) {
                if let Some(match_str) = cap.get(1) {
                    sensors.insert(format!("sensor.{}", match_str.as_str()));
                }
            }
        }

        let mut result: Vec<String> = sensors.into_iter().collect();
        result.sort(); // Sort for deterministic output
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_config_from_env_defaults() {
        // Set required env vars
        env::set_var("HA_BASE_URL", "http://localhost:8123");
        env::set_var("HA_LONG_LIVED_TOKEN", "test_token");

        // Clear optional ones to test defaults
        env::remove_var("SENSOR_ENTITY_ID");
        env::remove_var("PORT");
        env::remove_var("DATE_FORMAT");
        env::remove_var("TIME_FORMAT");
        env::remove_var("VIDEO_WIDTH");
        env::remove_var("VIDEO_HEIGHT");
        env::remove_var("VIDEO_FPS");
        env::remove_var("STREAM_FORMAT");
        env::remove_var("FONT_SIZE");
        env::remove_var("LOCALE");
        for i in 1..=4 {
            env::remove_var(format!("LINE_{}", i));
        }

        let config = Config::from_env().unwrap();

        assert_eq!(config.ha_base_url, "http://localhost:8123");
        assert_eq!(config.ha_token, "test_token");
        assert_eq!(config.port, 8080);
        assert_eq!(config.video_width, 640);
        assert_eq!(config.video_height, 360);
        assert_eq!(config.video_fps, 5);
        assert_eq!(config.stream_format, "mjpeg");
        assert_eq!(config.font_size, 48.0);
        assert_eq!(config.locale, "en_US");

        // Check fallback lines
        assert_eq!(config.lines.len(), 3);
        assert_eq!(config.lines[0], "{time:%Y-%m-%d}");
        assert_eq!(config.lines[1], "{time:%H.%M}");
        assert_eq!(config.lines[2], "{sensor.sensor.ute_kombinerad}°");

        // Cleanup
        env::remove_var("HA_BASE_URL");
        env::remove_var("HA_LONG_LIVED_TOKEN");
    }

    #[test]
    #[serial]
    fn test_config_lines() {
        env::set_var("HA_BASE_URL", "http://localhost:8123");
        env::set_var("HA_LONG_LIVED_TOKEN", "test_token");

        // Set line config
        env::set_var("LINE_1", "Hello World");
        env::set_var("LINE_2", "Temp: {sensor.temp}°C");
        env::set_var("LINE_3", "{time:%H:%M:%S}");
        env::set_var("FONT_SIZE", "64");
        env::set_var("LOCALE", "sv_SE");

        let config = Config::from_env().unwrap();

        assert_eq!(config.lines.len(), 3);
        assert_eq!(config.lines[0], "Hello World");
        assert_eq!(config.lines[1], "Temp: {sensor.temp}°C");
        assert_eq!(config.lines[2], "{time:%H:%M:%S}");
        assert_eq!(config.font_size, 64.0);
        assert_eq!(config.locale, "sv_SE");

        let sensors = config.get_required_sensors();
        assert_eq!(sensors.len(), 1);
        assert_eq!(sensors[0], "sensor.temp");

        // Cleanup
        env::remove_var("HA_BASE_URL");
        env::remove_var("HA_LONG_LIVED_TOKEN");
        env::remove_var("LINE_1");
        env::remove_var("LINE_2");
        env::remove_var("LINE_3");
        env::remove_var("FONT_SIZE");
        env::remove_var("LOCALE");
    }
}
