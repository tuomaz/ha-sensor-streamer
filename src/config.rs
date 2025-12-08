use anyhow::Result;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub ha_base_url: String,
    pub ha_token: String,
    pub sensor_entity_id: String,
    pub port: u16,
    pub date_format: String,
    pub time_format: String,
    pub video_width: u32,
    pub video_height: u32,
    pub video_fps: u64,
    pub stream_format: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let ha_base_url = env::var("HA_BASE_URL").expect("HA_BASE_URL must be set");
        let ha_token = env::var("HA_LONG_LIVED_TOKEN").expect("HA_LONG_LIVED_TOKEN must be set");
        let sensor_entity_id =
            env::var("SENSOR_ENTITY_ID").unwrap_or_else(|_| "sensor.ute_kombinerad".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .expect("PORT must be a number");
        let date_format = env::var("DATE_FORMAT").unwrap_or_else(|_| "%Y-%m-%d".to_string());
        let time_format = env::var("TIME_FORMAT").unwrap_or_else(|_| "%H.%M".to_string());
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

        // Ensure base URL doesn't end with slash for cleaner path joining
        let ha_base_url = if ha_base_url.ends_with('/') {
            ha_base_url[..ha_base_url.len() - 1].to_string()
        } else {
            ha_base_url
        };

        Ok(Config {
            ha_base_url,
            ha_token,
            sensor_entity_id,
            port,
            date_format,
            time_format,
            video_width,
            video_height,
            video_fps,
            stream_format,
        })
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

        let config = Config::from_env().unwrap();

        assert_eq!(config.ha_base_url, "http://localhost:8123");
        assert_eq!(config.ha_token, "test_token");
        assert_eq!(config.sensor_entity_id, "sensor.ute_kombinerad");
        assert_eq!(config.port, 8080);
        assert_eq!(config.video_width, 640);
        assert_eq!(config.video_height, 360);
        assert_eq!(config.video_fps, 5);
        assert_eq!(config.stream_format, "mjpeg");
        
        // Cleanup
        env::remove_var("HA_BASE_URL");
        env::remove_var("HA_LONG_LIVED_TOKEN");
    }

    #[test]
    #[serial]
    fn test_config_from_env_custom() {
        env::set_var("HA_BASE_URL", "http://homeassistant.local/"); // Test trailing slash removal
        env::set_var("HA_LONG_LIVED_TOKEN", "another_token");
        env::set_var("SENSOR_ENTITY_ID", "sensor.temp");
        env::set_var("PORT", "9090");
        env::set_var("DATE_FORMAT", "%d-%m-%Y");
        env::set_var("TIME_FORMAT", "%H:%M");
        env::set_var("VIDEO_WIDTH", "1280");
        env::set_var("VIDEO_HEIGHT", "720");
        env::set_var("VIDEO_FPS", "10");
        env::set_var("STREAM_FORMAT", "RTSP"); // Test case insensitivity

        let config = Config::from_env().unwrap();

        assert_eq!(config.ha_base_url, "http://homeassistant.local"); // Slash removed
        assert_eq!(config.ha_token, "another_token");
        assert_eq!(config.sensor_entity_id, "sensor.temp");
        assert_eq!(config.port, 9090);
        assert_eq!(config.date_format, "%d-%m-%Y");
        assert_eq!(config.time_format, "%H:%M");
        assert_eq!(config.video_width, 1280);
        assert_eq!(config.video_height, 720);
        assert_eq!(config.video_fps, 10);
        assert_eq!(config.stream_format, "rtsp"); // Lowercase

        // Cleanup
        env::remove_var("HA_BASE_URL");
        env::remove_var("HA_LONG_LIVED_TOKEN");
        env::remove_var("SENSOR_ENTITY_ID");
        env::remove_var("PORT");
        env::remove_var("DATE_FORMAT");
        env::remove_var("TIME_FORMAT");
        env::remove_var("VIDEO_WIDTH");
        env::remove_var("VIDEO_HEIGHT");
        env::remove_var("VIDEO_FPS");
        env::remove_var("STREAM_FORMAT");
    }
}
