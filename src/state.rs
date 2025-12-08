use crate::config::Config;
use crate::image_gen::ImageGenerator;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub sensor_value: Arc<RwLock<String>>,
    pub image_gen: Arc<ImageGenerator>,
    pub config: Config,
}
