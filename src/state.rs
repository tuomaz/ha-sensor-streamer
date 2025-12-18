use crate::config::Config;
use crate::image_gen::ImageGenerator;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub sensor_values: Arc<RwLock<HashMap<String, String>>>,
    pub image_gen: Arc<ImageGenerator>,
    pub config: Config,
}
