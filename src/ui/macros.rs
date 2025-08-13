#[macro_export]
macro_rules! blog {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",
            color::Fg(color::Green),
            $category,
            color::Fg(color::Reset),
            formatted_args
        );
    }};
}

#[macro_export]
macro_rules! blog_warning {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",
            color::Fg(color::Yellow),
            $category,
            color::Fg(color::Reset),
            formatted_args
        );
    }};
}

#[macro_export]
macro_rules! blog_working {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",
            color::Fg(color::Blue),
            $category,
            color::Fg(color::Reset),
            formatted_args
        );
    }};
}

#[macro_export]
macro_rules! blog_done {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",
            color::Fg(color::Green),
            $category,
            color::Fg(color::Reset),
            formatted_args
        );
    }};
}

pub use blog;
pub use blog_done;
pub use blog_warning;
pub use blog_working;
