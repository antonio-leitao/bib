use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use termion::color;

pub trait Clean {
    fn clean(&self) -> Self;
}

impl Clean for String {
    fn clean(&self) -> Self {
        let cleaned_str = self
            .chars() // Get iterator over characters
            .filter(|&c| !c.is_control()) // Filter out control characters (e.g., newline, tab)
            .collect(); // Collect the characters back into a String
        cleaned_str
    }
}

pub struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    category: String,
    message: String,
}

impl Spinner {
    pub fn new(category: &str, message: &str) -> Self {
        let running = Arc::new(AtomicBool::new(false));
        let spinner = Spinner {
            running: running.clone(),
            handle: None,
            category: category.to_string(),
            message: message.to_string(),
        };
        spinner
    }

    pub fn start(&mut self) {
        self.running.store(true, Ordering::Relaxed);

        let running = self.running.clone();
        let category = self.category.clone();
        let message = self.message.clone();

        let handle = thread::spawn(move || {
            let spinner_chars = ['|', '/', '-', '\\'];
            let mut idx = 0;

            while running.load(Ordering::Relaxed) {
                print!(
                    "\r{}{:>12}{} {} {}",
                    color::Fg(color::Green),
                    category,
                    color::Fg(color::Reset),
                    message,
                    spinner_chars[idx]
                );
                io::stdout().flush().unwrap();

                idx = (idx + 1) % spinner_chars.len();
                thread::sleep(Duration::from_millis(100));
            }
        });

        self.handle = Some(handle);
    }

    pub fn finish(&mut self, finished_message: Option<&str>) {
        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }

        // Clear the line and print the finished message
        print!("\r{}", termion::clear::CurrentLine);

        let final_message = finished_message.unwrap_or(&self.message);
        println!(
            "{}{:>12}{} {}",
            color::Fg(color::Green),
            self.category.trim_end_matches("ing").to_string() + "ed", // Convert "Extracting" to "Extracted"
            color::Fg(color::Reset),
            final_message
        );
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            self.finish(None);
        }
    }
}

#[macro_export]
macro_rules! blog {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",color::Fg(color::Green), $category,color::Fg(color::Reset), formatted_args);
    }};
}
