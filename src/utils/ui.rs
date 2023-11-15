use std::cmp::min;
use std::io::{self, Stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use sublime_fuzzy::best_match;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

const SUBDUED: termion::color::Rgb = color::Rgb(83, 110, 122);
const MUTED: termion::color::Rgb = color::Rgb(46, 60, 68);

pub enum Action<T: Item> {
    Open(T),
    Add(T),
    Notes(T),
    Remove(T),
}

pub trait Item {
    fn display(&self, max_width: u16) -> String;
    fn disabled(&self, max_width: u16) -> String;
    //searchable string
    fn slug(&self) -> String;
}

fn place_at_indices<T>(original: &mut [T], indices: &mut [usize]) {
    for i in 0..indices.len() {
        while i != indices[i] {
            let new_i = indices[i];
            indices.swap(i, new_i);
            original.swap(i, new_i);
        }
    }
}

fn argsort<T: Ord>(data: &[T]) -> Vec<usize> {
    let mut indices = (0..data.len()).collect::<Vec<_>>();
    indices.sort_by_key(|&i| &data[i]);
    indices
}

fn apply_filter<T: Item + Clone>(query: &str, items: &Vec<T>) -> Option<Vec<T>> {
    if query.is_empty() {
        return Some(items.clone());
    }
    //indexes where there is a match
    let mut indices: Vec<usize> = Vec::new();
    //score of match
    let mut scores: Vec<isize> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        match best_match(query, &item.slug()) {
            Some(matched) => {
                indices.push(i);
                scores.push(matched.score())
            }
            None => continue,
        };
    }
    //indexes that would sort the scores
    let mut order = argsort(&scores);
    //place the indices in that order
    place_at_indices(&mut indices, &mut order);
    let filtered_items: Vec<T> = indices
        .into_iter()
        .filter_map(|index| items.get(index).cloned())
        .collect();
    if filtered_items.is_empty() {
        None
    } else {
        Some(filtered_items)
    }
}

fn display_item<T: Item>(item: &T, disabled: bool, cursor: bool, max_width: u16) -> String {
    if disabled {
        format!("{}\r", item.disabled(max_width - 2))
    } else if cursor {
        format!("* {}\r", item.display(max_width - 2))
    } else {
        format!("  {}\r", item.display(max_width - 2))
    }
}

fn view_list<T: Item>(
    items: &Option<Vec<T>>,
    pager: &Pager,
    disabled: bool,
    cursor: usize,
    stdout: &mut RawTerminal<Stdout>,
    width: u16,
) {
    let start = pager.per_page * pager.current_page;
    match items {
        None => writeln!(stdout, "\n\n\t\t No items found :( \n\n\r").unwrap(),
        Some(items) => {
            let end = min(start + pager.per_page, items.len());
            for i in start..end {
                let item = &items[i];
                writeln!(
                    stdout,
                    "{}",
                    display_item(item, disabled, i == cursor, width)
                )
                .unwrap();
            }
        }
    }
}

fn view_pager(pager: &Pager, stdout: &mut RawTerminal<Stdout>) {
    for page in 0..pager.n_pages {
        if page == 0 {
            write!(
                stdout,
                "\n    {}{}/{}{}  ",
                color::Fg(SUBDUED),
                pager.current_page + 1,
                pager.n_pages,
                color::Fg(color::Reset),
            )
            .unwrap();
        }
        if page == pager.current_page {
            write!(stdout, "{}*{}", color::Fg(SUBDUED), color::Fg(color::Reset)).unwrap();
        } else {
            write!(stdout, "{}•{}", color::Fg(MUTED), color::Fg(color::Reset)).unwrap();
        }
    }
    writeln!(stdout, "\r").unwrap();
}

#[derive(PartialEq)]
enum State {
    Browsing,
    Writing,
    Quit,
}
struct Pager {
    current_page: usize,
    n_pages: usize,
    per_page: usize,
}
impl Pager {
    fn new<T: Item + Clone>(items: &Option<Vec<T>>, per_page: usize) -> Pager {
        let n_pages: usize;
        match items {
            Some(items) => {
                n_pages = (items.len() + per_page - 1) / per_page;
            }
            None => {
                n_pages = 0;
            }
        };

        Pager {
            current_page: 0,
            n_pages,
            per_page,
        }
    }
}
fn unwrap_color(color_name: &str) -> Box<dyn color::Color> {
    let color_enum: Box<dyn color::Color> = match color_name.to_lowercase().as_str() {
        "red" => Box::new(color::Red),
        "blue" => Box::new(color::Blue),
        "green" => Box::new(color::Green),
        "cyan" => Box::new(color::Cyan),
        _ => Box::new(color::Yellow),
    };
    color_enum
}
fn load_options(allow_add: bool, allow_notes: bool, allow_delete: bool) -> Vec<(String, String)> {
    let mut help_options = vec![("enter".to_string(), "open".to_string())];
    if allow_add {
        help_options.push(("a".to_string(), "add".to_string()))
    }
    if allow_notes {
        help_options.push(("n".to_string(), "notes".to_string()))
    }
    if allow_delete {
        help_options.push(("d".to_string(), "delete".to_string()))
    }
    help_options.extend(vec![
        ("↑/↓".to_string(), "move".to_string()),
        ("←/→".to_string(), "next".to_string()),
        ("/".to_string(), "filter".to_string()),
        ("esc".to_string(), "quit".to_string()),
    ]);
    help_options
}
struct Model<T>
where
    T: Item + Clone,
{
    state: State,
    cursor: usize,
    items: Option<Vec<T>>,
    selected: Option<Action<T>>,
    library: Vec<T>,
    stdout: RawTerminal<Stdout>,
    query: String,
    pager: Pager,
    width: u16,
    action: String,
    color: String,
    help_options: Vec<(String, String)>,
    allow_add: bool,
    allow_notes: bool,
    allow_delete: bool,
}
impl<T> Model<T>
where
    T: Item + Clone,
{
    fn new(
        action: String,
        color: String,
        library: Vec<T>,
        stdout: RawTerminal<Stdout>,
        query: String,
        allow_add: bool,
        allow_notes: bool,
        allow_delete: bool,
    ) -> Self {
        let items = apply_filter(&query, &library);
        let pager = Pager::new(&items, 30);
        let help_options = load_options(allow_add, allow_notes, allow_delete);
        let (width, _) = termion::terminal_size().unwrap();
        Model {
            state: State::Browsing,
            cursor: 0,
            items,
            selected: None,
            library,
            stdout,
            query,
            pager,
            width,
            action,
            color,
            help_options,
            allow_add,
            allow_notes,
            allow_delete,
        }
    }
    fn init(&mut self) {
        //initial model load
        write!(
            self.stdout,
            "{}{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1),
            termion::cursor::Hide
        )
        .unwrap();
        self.view()
    }
    fn view_header(&mut self) {
        let mut output: String = String::from("");
        //Action
        output.push_str(&format!(
            "{}{}{}",
            color::Fg(&*unwrap_color(&self.color)),
            self.action,
            color::Fg(color::Reset),
        ));
        //Filter
        if !self.query.is_empty() {
            let filter = format!(
                "{} • {}{}{}",
                color::Fg(SUBDUED),
                color::Fg(MUTED),
                self.query,
                color::Fg(color::Reset)
            );
            output.push_str(&filter)
        }
        //Projects
        let n_items = match &self.items {
            Some(items) => items.len(),
            None => 0,
        };
        let banner = format!(
            "{} • {}{} items{}",
            color::Fg(SUBDUED),
            color::Fg(MUTED),
            n_items,
            color::Fg(color::Reset)
        );
        output.push_str(&banner);
        writeln!(self.stdout, "    {}\n\r", output).unwrap()
    }
    fn view_filter(&mut self) {
        writeln!(
            self.stdout,
            "    {}Filter:{} {}\n\r",
            color::Fg(SUBDUED),
            color::Fg(color::Reset),
            self.query
        )
        .unwrap()
    }
    fn view_help(&mut self) {
        let mut help: String = String::from("");
        for (key, summary) in self.help_options.iter() {
            help.push_str(&format!(
                "{}{} {}{}{}{}",
                color::Fg(SUBDUED),
                key,
                color::Fg(MUTED),
                summary,
                if self.help_options.last() == Some(&(key.to_string(), summary.to_string())) {
                    ""
                } else {
                    " • "
                },
                color::Fg(color::Reset),
            ));
        }
        write!(self.stdout, "\n    {}", help).unwrap();
    }

    fn view(&mut self) {
        //clear screen
        write!(
            self.stdout,
            "{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::All
        )
        .unwrap();

        match self.state {
            State::Writing => {
                self.view_filter();
                // view_header(&self.title, &mut self.stdout);
                // view_filter(&self.query, false, &mut self.stdout);
                view_list(
                    &self.items,
                    &self.pager,
                    true,
                    self.cursor,
                    &mut self.stdout,
                    self.width,
                );
                if self.pager.n_pages > 1 {
                    view_pager(&self.pager, &mut self.stdout);
                }
                self.view_help()
            }
            State::Browsing => {
                self.view_header();
                // view_header(&self.title, &mut self.stdout);
                // view_filter(&self.query, true, &mut self.stdout);
                view_list(
                    &self.items,
                    &self.pager,
                    false,
                    self.cursor,
                    &mut self.stdout,
                    self.width,
                );
                if self.pager.n_pages > 1 {
                    view_pager(&self.pager, &mut self.stdout);
                }
                self.view_help()
            }
            State::Quit => {
                write!(self.stdout, "").unwrap();
            }
        }
        self.stdout.flush().unwrap();
    }
    fn update(&mut self, c: Result<Key, std::io::Error>) {
        match self.state {
            State::Browsing => self.update_browsing(c),
            State::Writing => self.update_writing(c),
            _ => self.update_browsing(c),
        }
    }
    fn update_browsing(&mut self, c: Result<Key, std::io::Error>) {
        match c.unwrap() {
            Key::Up | Key::Char('k') => {
                self.cursor_up();
            }
            Key::Down | Key::Char('j') => {
                self.cursor_down();
            }
            Key::Left | Key::Char('h') => {
                self.cursor_left();
            }
            Key::Right | Key::Char('l') => {
                self.cursor_right();
            }
            Key::Char('\n') => {
                self.open();
            }
            Key::Char('a') => {
                if self.allow_add {
                    self.add();
                }
            }
            Key::Char('n') => {
                if self.allow_notes {
                    self.notes();
                }
            }
            Key::Char('d') => {
                if self.allow_delete {
                    self.remove();
                }
            }
            Key::Char('/') => self.state = State::Writing,
            Key::Char('q') | Key::Esc | Key::Ctrl('c') => self.state = State::Quit,
            _ => {}
        }
    }
    fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1
        }
        self.update_pager()
    }
    fn cursor_down(&mut self) {
        let max_index = match &self.items {
            Some(papers) => papers.len() - 1,
            None => 0,
        };
        self.cursor = min(max_index, self.cursor + 1);
        self.update_pager()
    }
    fn cursor_left(&mut self) {
        if self.cursor >= self.pager.per_page {
            self.cursor -= self.pager.per_page;
        } else {
            self.cursor = 0;
        }
        self.update_pager();
    }
    fn cursor_right(&mut self) {
        let max_index = match &self.items {
            Some(papers) => papers.len() - 1,
            None => 0,
        };
        self.cursor = min(max_index, self.cursor + self.pager.per_page);
        self.update_pager();
    }
    fn select(&mut self) -> Option<T> {
        match &self.items {
            Some(papers) => Some(papers[self.cursor].clone()),
            None => None,
        }
    }
    fn open(&mut self) {
        self.selected = match self.select() {
            Some(paper) => Some(Action::Open(paper)),
            None => None,
        };
        self.state = State::Quit;
    }
    fn add(&mut self) {
        self.selected = match self.select() {
            Some(paper) => Some(Action::Add(paper)),
            None => None,
        };
        self.state = State::Quit;
    }
    fn notes(&mut self) {
        self.selected = match self.select() {
            Some(paper) => Some(Action::Notes(paper)),
            None => None,
        };
        self.state = State::Quit;
    }
    fn remove(&mut self) {
        self.selected = match self.select() {
            Some(paper) => Some(Action::Remove(paper)),
            None => None,
        };
        self.state = State::Quit;
    }
    fn update_writing(&mut self, c: Result<Key, std::io::Error>) {
        match c.unwrap() {
            Key::Esc | Key::Ctrl('c') => self.state = State::Quit,
            Key::Char(c) => {
                if c == '\n' {
                    self.cursor = 0;
                    self.state = State::Browsing
                } else {
                    self.query.push(c);
                    self.items = apply_filter(&self.query, &self.library);
                    self.update_pager();
                }
            }
            Key::Backspace => {
                if self.query.len() > 0 {
                    self.query.pop();
                    self.items = apply_filter(&self.query, &self.library);
                    self.update_pager();
                }
            }
            _ => {}
        }
    }
    fn update_pager(&mut self) {
        match &self.items {
            Some(papers) => {
                self.pager.n_pages = (papers.len() + self.pager.per_page - 1) / self.pager.per_page;
                self.pager.current_page = self.cursor / self.pager.per_page;
            }
            None => {
                self.pager.n_pages = 0;
                self.pager.current_page = 0;
            }
        };
    }
}

pub fn display_list<T: Item + Clone>(
    action: String,
    color: String,
    items: Vec<T>,
    initial_query: String,
    blank_slate: bool,
    allow_add: bool,
    allow_notes: bool,
    allow_delete: bool,
) -> Option<Action<T>> {
    //temporary solution
    let stdin = io::stdin();
    let stdout = io::stdout().into_raw_mode().unwrap();
    let mut query_value = String::new();
    if !blank_slate {
        query_value.push_str(&initial_query);
    }
    let mut model = Model::new(
        action,
        color,
        items,
        stdout,
        query_value,
        allow_add,
        allow_notes,
        allow_delete,
    );
    model.init();
    //every time a key is clicked
    for c in stdin.keys() {
        model.update(c);
        model.view();
        if model.state == State::Quit {
            break;
        }
    }
    //clean screen
    write!(model.stdout, "{}", termion::cursor::Show).unwrap();
    model.selected
}

pub struct Spinner {
    running: Arc<AtomicBool>,
    text: String,
}

impl Spinner {
    pub fn new(text: String) -> Spinner {
        Spinner {
            running: Arc::new(AtomicBool::new(false)),
            text,
        }
    }

    pub fn start(&self) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }
        let text = self.text.clone();
        self.running.store(true, Ordering::SeqCst);
        let running_clone = Arc::clone(&self.running);

        thread::spawn(move || {
            let spin_chars: Vec<char> = vec!['/', '-', '\\', '|'];
            let mut spin_idx = 0;

            while running_clone.load(Ordering::SeqCst) {
                print!("{}... {}\r", text, spin_chars[spin_idx]);
                spin_idx = (spin_idx + 1) % spin_chars.len();
                std::io::stdout().flush().expect("Failed to flush stdout");
                thread::sleep(Duration::from_millis(100));
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        println!("{}... Done", self.text); // Clear the spinner character
        std::io::stdout().flush().expect("Failed to flush stdout");
    }
}
