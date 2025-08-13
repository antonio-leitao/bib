pub mod macros;

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use url::Url;

pub use macros::*;

pub struct UI;

impl UI {
    pub fn download_progress(total_size: u64, url: &str) -> ProgressBar {
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.blue.bold} {spinner:.blue} [{bar:30}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .expect("Invalid progress template")
                .progress_chars("=> "),
        );

        let domain = Url::parse(url)
            .ok()
            .and_then(|u| u.domain().map(|d| d.to_string()))
            .unwrap_or_else(|| "source".to_string());

        pb.set_prefix(format!("{:>12}", "Downloading"));
        pb.set_message(format!("from {}", domain));
        pb
    }

    pub fn spinner(category: &str, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{prefix:.blue.bold} {spinner:.blue} {msg}")
                .expect("Invalid spinner template")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_prefix(format!("{:>12}", category));
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    }

    pub fn finish_with_message(pb: ProgressBar, completed_category: &str, message: &str) {
        pb.finish_and_clear();
        blog_done!(completed_category, "{}", message);
    }
}

pub fn error_message(err: &str) {
    println!(
        "{}{:>12}{} {}",
        termion::color::Fg(termion::color::Red),
        "Error",
        termion::color::Fg(termion::color::Reset),
        err
    );
}
