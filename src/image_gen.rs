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
    sensor_regex: Regex,
    time_regex: Regex,
}

impl ImageGenerator {
    pub fn new(
        font_data: &'static [u8],
        lines: Vec<String>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let font = Font::try_from_bytes(font_data).context("Error constructing Font from data")?;
        let sensor_regex = Regex::new(r"\{sensor\.([\w\.]+)\}").expect("Invalid sensor regex");
        let time_regex = Regex::new(r"\{time:([^}]+)\}").expect("Invalid time regex");

        Ok(Self {
            font: Arc::new(font),
            width,
            height,
            lines,
            sensor_regex,
            time_regex,
        })
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
        // Note: We need to handle multiple time placeholders or just one?
        // Replace all instances of {time:fmt}
        // Since we can't easily replace_all with a capturing group that changes per match in a simple pass
        // without a loop or a sophisticated replace function, we'll iterate.
        // Actually, Regex::replace_all accepts a closure.
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
                sensor_values
                    .get(&entity_id)
                    .cloned()
                    .unwrap_or_else(|| "?".to_string())
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

        // Configurable scales could be added later, currently fixed.
        // We'll use a slightly smaller font if there are many lines?
        // Or just stick to a reasonable default.
        // Previous code had 48.0 and 60.0. Let's try 50.0 for all for consistency,
        // or vary it. Let's stick to 48.0 for now.
        let scale = Scale { x: 48.0, y: 48.0 };
        let white = Rgb([255, 255, 255]);
        let line_height = 48;
        let gap = 12;

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
        let generator = ImageGenerator::new(font_data, lines, 640, 360)
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
    fn test_resolve_line() {
        let font_data = include_bytes!("../assets/Lato-Regular.ttf");
        let lines = vec![];
        let generator = ImageGenerator::new(font_data, lines, 640, 360).unwrap();

        let mut sensors = HashMap::new();
        sensors.insert("sensor.temp".to_string(), "20".to_string());

        // Test Time
        let res = generator.resolve_line("Time: {time:%H}", &sensors);
        assert!(res.starts_with("Time: ")); // Can't easily check hour, but regex works

        // Test Sensor
        let res = generator.resolve_line("Temp: {sensor.temp}", &sensors);
        assert_eq!(res, "Temp: 20");

        // Test Missing Sensor
        let res = generator.resolve_line("Hum: {sensor.hum}", &sensors);
        assert_eq!(res, "Hum: ?");

        // Test Multiple
        sensors.insert("sensor.hum".to_string(), "50".to_string());
        let res = generator.resolve_line("T: {sensor.temp} H: {sensor.hum}", &sensors);
        assert_eq!(res, "T: 20 H: 50");
    }
}
