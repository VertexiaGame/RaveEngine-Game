use bevy::prelude::*;
use bevy::log::BoxedLayer;
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::io::Write;
use bevy::log::tracing_subscriber::Layer;

pub struct VuisFileLogger {
    pub file: Arc<Mutex<File>>,
}

impl<S: bevy::log::tracing::Subscriber> Layer<S> for VuisFileLogger {
    fn on_event(
        &self,
        event: &bevy::log::tracing::Event<'_>,
        _ctx: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Ok(mut file) = self.file.lock() {
            struct MessageVisitor {
                message: String,
            }
            impl bevy::log::tracing::field::Visit for MessageVisitor {
                fn record_debug(&mut self, field: &bevy::log::tracing::field::Field, value: &dyn std::fmt::Debug) {
                    if field.name() == "message" {
                        self.message = format!("{:?}", value);
                    }
                }
            }
            let mut visitor = MessageVisitor { message: String::new() };
            event.record(&mut visitor);
            
            let level = event.metadata().level();
            let target = event.metadata().target();
            
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
                
            let line = format!("[{}] {} [{}]: {}\n", now, level, target, visitor.message);
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }
}

pub fn vuis_custom_layer(_app: &mut App) -> Option<BoxedLayer> {
    let app_name = std::env::var("VERTIGO_APP").unwrap_or_else(|_| "unknown".to_string());
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let filename = format!("log{}-{}.txt", app_name, timestamp);
    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(filename)
    {
        Some(Box::new(VuisFileLogger {
            file: Arc::new(Mutex::new(file)),
        }))
    } else {
        None
    }
}