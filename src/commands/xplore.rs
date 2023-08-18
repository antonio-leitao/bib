use crate::utils::{parse_bibliography, Paper};
use biblatex::Bibliography;
use std::cmp::min;
use std::error::Error;
use std::fs;
use std::io::{self, Stdout, Write};
use sublime_fuzzy::best_match;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
fn open_file() -> Result<String, Box<dyn Error>> {
    let file_path = "./bibliography.bib"; // Replace with the actual path to your file

    // Read the file contents into a string
    let file_contents = fs::read_to_string(file_path)?;
    Ok(file_contents)
}

fn read_bibliography() -> Vec<Paper> {
    let src = open_file().expect("Could not read bibliography file");
    let bibliography = Bibliography::parse(&src).unwrap_or_else(|err| {
        eprintln!("Error parsing BibTeX file: {}", err);
        std::process::exit(1);
    });
    parse_bibliography(bibliography)
}

fn apply_filter(query: &str, bibliography: &Vec<Paper>) -> Option<Vec<Paper>> {
    if query.is_empty() {
        return Some(bibliography.clone());
    }
    let mut matching_papers: Vec<(Paper, isize)> = bibliography
        .iter()
        .map(|paper| {
            let match_result = best_match(query, &paper.slug);
            let score = match match_result {
                Some(matched) => matched.score(),
                None => 0,
            };
            (paper.clone(), score)
        })
        .filter(|(_, score)| *score > 0)
        .collect();

    matching_papers.sort_by(|(_, score_a), (_, score_b)| {
        score_b
            .partial_cmp(score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let filtered_papers: Vec<Paper> = matching_papers
        .into_iter()
        .map(|(paper, _)| paper)
        .collect();

    if filtered_papers.is_empty() {
        None
    } else {
        Some(filtered_papers)
    }
}

// TERMION STUFF
fn view_filter(query: &String, disabled: bool, stdout: &mut RawTerminal<Stdout>) {
    if disabled {
        writeln!(
            stdout,
            "    {}Filter: {}{}\n\r",
            color::Fg(color::Rgb(83, 110, 122)),
            query,
            color::Fg(color::Reset)
        )
        .unwrap()
    } else {
        writeln!(stdout, "    Filter: {}\n\r", query).unwrap()
    }
}
fn format_paper(paper: &Paper, disabled: bool, cursor: bool) -> String {
    if disabled {
        format!(
            "{}  {} | {} | {} [Notes]{}\r",
            color::Fg(color::Rgb(83, 110, 122)),
            paper.year,
            paper.author,
            paper.title,
            color::Fg(color::Reset),
        )
    } else if cursor {
        format!(
            "* {} {}|{} {} {}|{} {} {}[Notes]{}\r",
            paper.year,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            paper.author,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            paper.title,
            color::Fg(color::Yellow),
            color::Fg(color::Reset),
        )
    } else {
        format!(
            "  {} {}|{} {} {}|{} {} {}[Notes]{}\r",
            paper.year,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            paper.author,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            paper.title,
            color::Fg(color::Yellow),
            color::Fg(color::Reset),
        )
    }
}

fn view_list(
    papers: &Option<Vec<Paper>>,
    pager: &Pager,
    disabled: bool,
    cursor: usize,
    stdout: &mut RawTerminal<Stdout>,
) {
    let start = pager.per_page * pager.current_page;
    match papers {
        None => writeln!(stdout, "\n\n\t\t No papers found :( \n\n").unwrap(),
        Some(papers) => {
            let end = min(start + pager.per_page, papers.len());
            for i in start..end {
                let paper = &papers[i];
                writeln!(stdout, "{}", format_paper(paper, disabled, i == cursor)).unwrap();
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
                color::Fg(color::Rgb(83, 110, 122)),
                pager.current_page + 1,
                pager.n_pages,
                color::Fg(color::Reset),
            )
            .unwrap();
        }
        if page == pager.current_page {
            write!(
                stdout,
                "{}*{}",
                color::Fg(color::Rgb(83, 110, 122)),
                color::Fg(color::Reset)
            )
            .unwrap();
        } else {
            write!(
                stdout,
                "{}•{}",
                color::Fg(color::Rgb(46, 60, 68)),
                color::Fg(color::Reset)
            )
            .unwrap();
        }
    }
    writeln!(stdout).unwrap();
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
    fn new(papers: &Option<Vec<Paper>>, per_page: usize) -> Pager {
        let n_pages: usize;
        match papers {
            Some(papers) => {
                n_pages = (papers.len() + per_page - 1) / per_page;
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
struct Model {
    state: State,
    cursor: usize,
    papers: Option<Vec<Paper>>,
    bibliography: Vec<Paper>,
    stdout: RawTerminal<Stdout>,
    query: String,
    pager: Pager,
}
impl Model {
    fn new(bibliography: Vec<Paper>, stdout: RawTerminal<Stdout>, query: String) -> Model {
        let papers = apply_filter(&query, &bibliography);
        let pager = Pager::new(&papers, 30);
        Model {
            state: State::Browsing,
            cursor: 0,
            papers,
            bibliography,
            stdout,
            query,
            pager,
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
                view_filter(&self.query, false, &mut self.stdout);
                view_list(
                    &self.papers,
                    &self.pager,
                    true,
                    self.cursor,
                    &mut self.stdout,
                );
                if self.pager.n_pages > 1 {
                    view_pager(&self.pager, &mut self.stdout);
                }
            }
            State::Browsing => {
                view_filter(&self.query, true, &mut self.stdout);
                view_list(
                    &self.papers,
                    &self.pager,
                    false,
                    self.cursor,
                    &mut self.stdout,
                );
                if self.pager.n_pages > 1 {
                    view_pager(&self.pager, &mut self.stdout);
                }
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
        let max_index = match &self.papers {
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
        let max_index = match &self.papers {
            Some(papers) => papers.len() - 1,
            None => 0,
        };
        self.cursor = min(max_index, self.cursor + self.pager.per_page);
        self.update_pager();
    }
    fn update_writing(&mut self, c: Result<Key, std::io::Error>) {
        match c.unwrap() {
            Key::Esc | Key::Ctrl('c') => self.state = State::Quit,
            Key::Char(c) => {
                if c == '\n' {
                    self.papers = apply_filter(&self.query, &self.bibliography);
                    self.update_pager();
                    self.cursor = 0;
                    self.state = State::Browsing
                } else {
                    self.query.push(c);
                }
            }
            Key::Backspace => {
                if self.query.len() > 0 {
                    self.query.pop();
                }
            }
            _ => {}
        }
    }
    fn update_pager(&mut self) {
        match &self.papers {
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

pub fn execute(query: Option<String>) {
    //read paper list
    let bibliography = read_bibliography();
    let initial_query = query.unwrap_or(String::from(""));
    //temporary solution
    let stdin = io::stdin();
    let stdout = io::stdout().into_raw_mode().unwrap();
    let mut model = Model::new(bibliography, stdout, initial_query);
    model.init();
    //every time a key is clicked
    for c in stdin.keys() {
        model.update(c);
        model.view();
        if model.state == State::Quit {
            break;
        }
    }
    write!(model.stdout, "{}", termion::cursor::Show).unwrap();
}
