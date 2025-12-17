use anyhow::{Context, Result};
use chrono::Local;
use image::{ImageOutputFormat, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use regex::Regex;
use rusttype::{point, Font, Scale};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

pub struct ImageGenerator {
    font: Arc<Font<'static>>,
    width: u32,
    height: u32,
    lines: Vec<String>,
    font_size: f32,
    decimal_separator: char,
    sensor_regex: Regex,
    time_regex: Regex,
}

impl ImageGenerator {
    pub fn new(
        font_data: &'static [u8],
        lines: Vec<String>,
        font_size: f32,
        locale: &str,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let font = Font::try_from_bytes(font_data).context("Error constructing Font from data")?;
        let sensor_regex = Regex::new(r"\{sensor\.([\w\.]+)\}").expect("Invalid sensor regex");
        let time_regex = Regex::new(r"\{time:([^}]+)\}").expect("Invalid time regex");
        let decimal_separator = Self::get_decimal_separator(locale);

        Ok(Self {
            font: Arc::new(font),
            width,
            height,
            lines,
            font_size,
            decimal_separator,
            sensor_regex,
            time_regex,
        })
    }

    fn get_decimal_separator(locale: &str) -> char {
        let l = locale.to_lowercase();
        // Common locales that use comma as decimal separator
        // Nordic, Western/Southern/Eastern Europe, Russia, South America, etc.
        let comma_prefixes = [
            "sv", "no", "nb", "nn", "da", "fi", "is", // Nordic
            "de", "nl", "pl", "cs", "sk", "hu", "ro", "bg", "hr", "sr", "sl", "bs",
            "mk", // Central/East EU
            "fr", "es", "pt", "it", "el", "tr", // West/South EU
            "ru", "uk", "be", "kk", // Cyrillic
            "id", "vi", // SE Asia
            "az", "sq", "hy", "ka", // Others
        ];

        if comma_prefixes.iter().any(|&p| l.starts_with(p)) {
            ','
        } else {
            '.'
        }
    }

    fn measure_text_width(&self, text: &str, scale: Scale) -> u32 {
        let width = self
            .font
            .layout(text, scale, point(0.0, 0.0))
            .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
            .last()
            .unwrap_or(0.0);
        width.ceil() as u32
    }

    fn resolve_line(&self, template: &str, sensor_values: &HashMap<String, String>) -> String {
        let mut result = template.to_string();

        // Replace Time
        let now = Local::now();
        result = self
            .time_regex
            .replace_all(&result, |caps: &regex::Captures| {
                let fmt = &caps[1];
                now.format(fmt).to_string()
            })
            .to_string();

        // Replace Sensors
        result = self
            .sensor_regex
            .replace_all(&result, |caps: &regex::Captures| {
                let entity_id = format!("sensor.{}", &caps[1]);
                let val = sensor_values
                    .get(&entity_id)
                    .cloned()
                    .unwrap_or_else(|| "?".to_string());

                // Apply decimal separator if numeric
                if val.parse::<f64>().is_ok() {
                    val.replace('.', &self.decimal_separator.to_string())
                } else {
                    val
                }
            })
            .to_string();

        result
    }

    fn draw_frame(&self, sensor_values: &HashMap<String, String>) -> RgbImage {
        let mut image = RgbImage::new(self.width, self.height);

        // Fill with black
        for pixel in image.pixels_mut() {
            *pixel = Rgb([0, 0, 0]);
        }

        let scale = Scale {
            x: self.font_size,
            y: self.font_size,
        };
        let white = Rgb([255, 255, 255]);
        let line_height = self.font_size as i32;
        let gap = (self.font_size * 0.25) as i32; // 25% gap

        let total_lines = self.lines.len() as i32;
        let total_content_height = total_lines * line_height + (total_lines - 1).max(0) * gap;
        let start_y = (self.height as i32 - total_content_height) / 2;

        for (i, line_template) in self.lines.iter().enumerate() {
            let text = self.resolve_line(line_template, sensor_values);
            let text_width = self.measure_text_width(&text, scale);
            let x = (self.width as i32 - text_width as i32) / 2;
            let y = start_y + i as i32 * (line_height + gap);

            draw_text_mut(&mut image, white, x, y, scale, &self.font, &text);
        }

        image
    }

    pub fn generate_frame(&self, sensor_values: &HashMap<String, String>) -> Result<Vec<u8>> {
        let image = self.draw_frame(sensor_values);

        // Encode to JPEG
        let mut buffer = Cursor::new(Vec::new());
        image.write_to(&mut buffer, ImageOutputFormat::Jpeg(80))?;

        Ok(buffer.into_inner())
    }

    pub fn generate_raw_frame(&self, sensor_values: &HashMap<String, String>) -> Vec<u8> {
        let image = self.draw_frame(sensor_values);
        image.into_raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_generation() {
        let font_data = include_bytes!("../assets/Lato-Regular.ttf");
        let lines = vec![
            "Date: {time:%Y-%m-%d}".to_string(),
            "Temp: {sensor.temp}°C".to_string(),
        ];
        let generator = ImageGenerator::new(font_data, lines, 48.0, "en_US", 640, 360)
            .expect("Failed to create ImageGenerator");

        let mut sensors = HashMap::new();
        sensors.insert("sensor.temp".to_string(), "22.5".to_string());

        let frame = generator
            .generate_frame(&sensors)
            .expect("Failed to generate frame");

        assert!(!frame.is_empty());
        assert_eq!(frame[0], 0xFF);
        assert_eq!(frame[1], 0xD8);
    }

    #[test]
    fn test_resolve_line_locale() {
        let font_data = include_bytes!("../assets/Lato-Regular.ttf");
        let lines = vec![];

        // Test US Locale (Dot)
        let gen_us =
            ImageGenerator::new(font_data, lines.clone(), 48.0, "en_US", 640, 360).unwrap();
        let mut sensors = HashMap::new();
        sensors.insert("sensor.temp".to_string(), "22.5".to_string());
        assert_eq!(gen_us.resolve_line("{sensor.temp}", &sensors), "22.5");

        // Test SV Locale (Comma)
        let gen_sv =
            ImageGenerator::new(font_data, lines.clone(), 48.0, "sv_SE", 640, 360).unwrap();
        assert_eq!(gen_sv.resolve_line("{sensor.temp}", &sensors), "22,5");

        // Test Non-numeric
        sensors.insert("sensor.state".to_string(), "on".to_string());
        assert_eq!(gen_sv.resolve_line("{sensor.state}", &sensors), "on");

        // Test IP (multiple dots, parses as float? "1.2.3.4" -> No)
        sensors.insert("sensor.ip".to_string(), "192.168.1.1".to_string());
        assert_eq!(gen_sv.resolve_line("{sensor.ip}", &sensors), "192.168.1.1");

        // Test simple version number "1.2" parses as float -> "1,2".
        // This is a trade-off. "Version 1.2" might become "Version 1,2".
        // Usually acceptable if LOCALE is set.
        sensors.insert("sensor.ver".to_string(), "1.5".to_string());
        assert_eq!(gen_sv.resolve_line("{sensor.ver}", &sensors), "1,5");
    }
}
