use anyhow::{Context, Result};
use chrono::Local;
use image::{ImageOutputFormat, Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{point, Font, Scale};
use std::io::Cursor;
use std::sync::Arc;

pub struct ImageGenerator {
    font: Arc<Font<'static>>,
    width: u32,
    height: u32,
    date_format: String,
    time_format: String,
}

impl ImageGenerator {
    pub fn new(
        font_data: &'static [u8],
        date_format: String,
        time_format: String,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let font = Font::try_from_bytes(font_data).context("Error constructing Font from data")?;

        Ok(Self {
            font: Arc::new(font),
            width,
            height,
            date_format,
            time_format,
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

    fn draw_frame(&self, sensor_value: &str) -> RgbImage {
        let mut image = RgbImage::new(self.width, self.height);

        // Fill with black
        for pixel in image.pixels_mut() {
            *pixel = Rgb([0, 0, 0]);
        }

        let scale_date_time = Scale { x: 48.0, y: 48.0 };
        let scale_sensor = Scale { x: 60.0, y: 60.0 };
        let white = Rgb([255, 255, 255]);

        let now = Local::now();
        let date_str = now.format(&self.date_format).to_string();
        let time_str = now.format(&self.time_format).to_string();
        let sensor_display = format!("{}°", sensor_value);

        // Calculate positions to center text horizontally
        let date_width = self.measure_text_width(&date_str, scale_date_time);
        let time_width = self.measure_text_width(&time_str, scale_date_time);
        let sensor_width = self.measure_text_width(&sensor_display, scale_sensor);

        let date_x = (self.width.saturating_sub(date_width)) / 2;
        let time_x = (self.width.saturating_sub(time_width)) / 2;
        let sensor_x = (self.width.saturating_sub(sensor_width)) / 2;

        // Vertical centering logic
        // Estimate heights: Date(48) + Time(48) + Sensor(60) + Gaps
        // Gap1 (Date->Time): 10px
        // Gap2 (Time->Sensor): 40px
        // Total block height approx: 48 + 10 + 48 + 40 + 60 = 206px
        let content_height = 206;
        let start_y = (self.height.saturating_sub(content_height)) / 2;

        let date_y = start_y;
        let time_y = date_y + 48 + 10;
        let sensor_y = time_y + 48 + 40;

        // Draw Date
        draw_text_mut(
            &mut image,
            white,
            date_x as i32,
            date_y as i32,
            scale_date_time,
            &self.font,
            &date_str,
        );

        // Draw Time
        draw_text_mut(
            &mut image,
            white,
            time_x as i32,
            time_y as i32,
            scale_date_time,
            &self.font,
            &time_str,
        );

        // Draw Sensor Value
        draw_text_mut(
            &mut image,
            white,
            sensor_x as i32,
            sensor_y as i32,
            scale_sensor,
            &self.font,
            &sensor_display,
        );

        image
    }

    pub fn generate_frame(&self, sensor_value: &str) -> Result<Vec<u8>> {
        let image = self.draw_frame(sensor_value);

        // Encode to JPEG
        let mut buffer = Cursor::new(Vec::new());
        image.write_to(&mut buffer, ImageOutputFormat::Jpeg(80))?;

        Ok(buffer.into_inner())
    }

    pub fn generate_raw_frame(&self, sensor_value: &str) -> Vec<u8> {
        let image = self.draw_frame(sensor_value);
        image.into_raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_generation() {
        let font_data = include_bytes!("../assets/Lato-Regular.ttf");
        let generator = ImageGenerator::new(
            font_data,
            "%Y-%m-%d".to_string(),
            "%H:%M".to_string(),
            640,
            360,
        ).expect("Failed to create ImageGenerator");

        let frame = generator.generate_frame("22.5").expect("Failed to generate frame");
        
        // Check if we got some bytes back
        assert!(!frame.is_empty());
        
        // Basic check for JPEG header (FF D8)
        assert_eq!(frame[0], 0xFF);
        assert_eq!(frame[1], 0xD8);
    }

    #[test]
    fn test_text_measurement() {
        let font_data = include_bytes!("../assets/Lato-Regular.ttf");
        let generator = ImageGenerator::new(
            font_data,
            "%Y-%m-%d".to_string(),
            "%H:%M".to_string(),
            640,
            360,
        ).expect("Failed to create ImageGenerator");

        let scale = Scale { x: 48.0, y: 48.0 };
        let width = generator.measure_text_width("Test", scale);
        assert!(width > 0);
    }
}
