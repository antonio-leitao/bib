use std::cmp::min;
use std::io::{self, Stdout, Write};
use sublime_fuzzy::best_match;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

const SUBDUED: termion::color::Rgb = color::Rgb(83, 110, 122);
const MUTED: termion::color::Rgb = color::Rgb(46, 60, 68);
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
fn view_help(stdout: &mut RawTerminal<Stdout>) {
    let mut help: String = String::from("");
    let help_options: Vec<(&str, &str)> = vec![
        ("enter", "select"),
        ("↑/↓", "move"),
        ("←/→", "next"),
        ("/", "filter"),
        ("esc", "quit"),
    ];
    for (key, summary) in help_options.iter() {
        help.push_str(&format!(
            "{}{} {}{}{}{}",
            color::Fg(SUBDUED),
            key,
            color::Fg(MUTED),
            summary,
            if help_options.last() == Some(&(key, summary)) {
                ""
            } else {
                " • "
            },
            color::Fg(color::Reset),
        ));
    }
    write!(stdout, "\n    {}", help).unwrap();
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
struct Model<T>
where
    T: Item + Clone,
{
    state: State,
    cursor: usize,
    items: Option<Vec<T>>,
    selected: Option<T>,
    library: Vec<T>,
    stdout: RawTerminal<Stdout>,
    query: String,
    pager: Pager,
    width: u16,
    action: String,
    color: String,
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
    ) -> Self {
        let items = apply_filter(&query, &library);
        let pager = Pager::new(&items, 30);
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
                view_help(&mut self.stdout)
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
                view_help(&mut self.stdout)
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
                self.select();
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
    fn select(&mut self) {
        self.selected = match &self.items {
            Some(papers) => Some(papers[self.cursor].clone()),
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

pub fn run_ui<T: Item + Clone>(
    action: String,
    color: String,
    items: Vec<T>,
    initial_query: String,
    blank_slate: bool,
) -> Option<T> {
    //temporary solution
    let stdin = io::stdin();
    let stdout = io::stdout().into_raw_mode().unwrap();
    let mut query_value = String::new();
    if !blank_slate {
        query_value.push_str(&initial_query);
    }
    let mut model = Model::new(action, color, items, stdout, query_value);
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