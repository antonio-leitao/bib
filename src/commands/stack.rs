use crate::base::{load_papers, Paper};
use crate::{
    blog,
    stacks::Stack,
    utils::io::{read_config_file, save_config_file},
};
use anyhow::{bail, Result};
use std::collections::{BTreeMap, HashMap};
use termion::color::{Fg, Reset, Rgb};

fn count_papers_per_stack(papers: &BTreeMap<String, Paper>) -> HashMap<String, usize> {
    let mut stack_counts = HashMap::new();
    for paper in papers.values() {
        for stack in &paper.stack {
            *stack_counts.entry(stack.name.clone()).or_insert(0) += 1;
        }
    }
    stack_counts
}

pub fn list() -> Result<()> {
    let config = read_config_file()?;
    let current_stack = config.current_stack();
    let papers = load_papers()?;
    let paper_per_stack: HashMap<_, _> = count_papers_per_stack(&papers);

    match &current_stack {
        None => println!("No stack selected. Available stacks"),
        Some(stack) => println!("Currently in stack: {}", stack),
    };

    for stack in config.stacks {
        let prefix = if Some(&stack) == current_stack.as_ref() {
            "* "
        } else {
            "  "
        };
        let paper_count = paper_per_stack.get(&stack.name).unwrap_or(&0);
        println!(
            "{}{:>5} {}• {} papers{}",
            prefix,
            stack.name,
            Fg(Rgb(83, 110, 122)),
            paper_count,
            Fg(Reset),
        );
    }

    Ok(())
}

pub fn new(name: String) -> Result<()> {
    let mut config = read_config_file()?;
    let new_stack = Stack::new(&name, &config.stacks)?;
    config.stacks.push(new_stack.clone());
    save_config_file(&config)?;
    blog!("Created", "new stack: {}", new_stack);
    Ok(())
}
pub fn switch(name: String) -> Result<()> {
    let mut config = read_config_file()?;
    let mut found = false;
    let mut new_stack = Stack {
        name: "default".to_string(),
        color: "default".to_string(),
    };
    config.stacks.iter().for_each(|s| {
        if s.name == name {
            found = true;
            new_stack = s.clone()
        };
    });
    if !found {
        bail!(
            "No stack named {}.\nRun bib stack {} new\n to create it",
            name,
            name
        )
    }
    config.stack = name;
    save_config_file(&config)?;
    blog!("Switched", "to stack: {}", new_stack);
    Ok(())
}

pub fn unstack() -> Result<()> {
    let mut config = read_config_file()?;
    if config.current_stack().is_none() {
        bail!("Already unstacked")
    }
    config.stack = String::from("all");
    save_config_file(&config)?;
    blog!("Unstacked", "working on all papers");
    Ok(())
}

pub fn rename(old_name: String, new_name: String) {
    println!("Renaming {} to {}", old_name, new_name)
}
pub fn delete(name: String) {
    println!("Deleting stack {}", name)
}
pub fn merge(from: String, into: String) {
    println!("Merging stack {} into stack {}", from, into)
}
pub fn fork(from: String, into: String) {
    println!("Forking stack {} to new stack {}", from, into)
}
