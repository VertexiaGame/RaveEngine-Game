use std::panic;
use std::sync::Mutex;
use std::backtrace::Backtrace;
use std::time::{Instant, Duration};

static PANIC_INFO: Mutex<Option<(String, String)>> = Mutex::new(None);

pub fn init_crash_handler() {
    panic::set_hook(Box::new(|info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let location = if let Some(loc) = info.location() {
            format!("at {}:{}:{}", loc.file(), loc.line(), loc.column())
        } else {
            "unknown location".to_string()
        };

        let bt = Backtrace::force_capture();
        let bt_str = format!("{}", bt);

        let full_message = format!("Panic: {} {}\n\nStack Trace:\n{}", message, location, bt_str);

        if let Ok(mut guard) = PANIC_INFO.lock() {
            *guard = Some((message, full_message));
        }
    }));
}

pub fn run_safe<F>(mut f: F)
where
    F: FnMut() + std::panic::UnwindSafe,
{
    init_crash_handler();

    let mut restart_count = 0;
    let mut last_restart = Instant::now();

    loop {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(&mut f));

        if result.is_ok() {
            break;
        }

        let now = Instant::now();
        if now.duration_since(last_restart) > Duration::from_secs(10) {
            restart_count = 0;
        }
        last_restart = now;

        if restart_count < 3 {
            restart_count += 1;
            bevy::log::warn!("VERTEXIA has encountered a fatal error! Attempting to recover and restart the executable (Attempt {}/3)...", restart_count);
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }

        let mut stack_trace = "No stack trace available.".to_string();
        if let Ok(guard) = PANIC_INFO.lock() {
            if let Some((_, ref full)) = *guard {
                stack_trace = full.clone();
            }
        }

        rfd::MessageDialog::new()
            .set_title("VERTEXIA has crashed!")
            .set_description(&format!("VERTEXIA has crashed! Please report this to the developers:\n\n{}", stack_trace))
            .set_level(rfd::MessageLevel::Error)
            .show();
        
        break;
    }
}