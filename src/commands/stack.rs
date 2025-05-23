use crate::base::{load_papers, save_papers, Paper};
use crate::{
    blog,
    stacks::Stack,
    utils::io::{read_config_file, save_config_file},
};
use anyhow::{bail, Result};
use indexmap::IndexMap;
use std::collections::HashMap;
use termion::color::{Fg, Reset, Rgb};

fn count_papers_per_stack(papers: &IndexMap<String, Paper>) -> HashMap<String, usize> {
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
    if !config.stacks.iter().any(|s| s.name == name) {
        bail!(
            "No stack named {}.\nRun bib stack {} new\n to create it",
            name,
            name
        )
    };
    config.stack = name.clone();
    save_config_file(&config)?;
    blog!("Switched", "to stack: {}", name);
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

pub fn rename(old_name: String, new_name: String) -> Result<()> {
    let mut papers = load_papers()?;
    let mut config = read_config_file()?;
    // Check if new stack name is reserved
    if new_name == "all" {
        bail!("Name reserved for base stack")
    }

    //check if its already taken
    if config.stacks.iter().any(|s| s.name == new_name) {
        bail!("Stack {} already exists", new_name);
    }

    // Change name of papers
    for (_, paper) in papers.iter_mut() {
        for stack in paper.stack.iter_mut() {
            if stack.name == old_name {
                stack.name = new_name.clone();
            }
        }
    }

    //change name of stack
    for stack in config.stacks.iter_mut() {
        if stack.name == old_name {
            stack.name = new_name.clone();
        }
    }

    //check if its current stack
    match config.current_stack() {
        None => (),
        Some(mut stack) => {
            if stack.name == old_name {
                stack.name = new_name.clone()
            }
        }
    };
    save_papers(&papers)?;
    save_config_file(&config)?;
    blog!("Renamed", "stack {} to {}", old_name, new_name);
    Ok(())
}
pub fn drop(name: String) -> Result<()> {
    let mut papers = load_papers()?;
    let mut config = read_config_file()?;
    config.stacks.retain(|stack| stack.name != name);
    for (_, paper) in papers.iter_mut() {
        paper.stack.retain(|stack| stack.name != name);
    }

    if config.stack == name {
        config.stack = "all".to_string()
    }
    save_papers(&papers)?;
    save_config_file(&config)?;
    blog!("Dropped", "stack {}", name);
    Ok(())
}

pub fn merge(from: String, into: String) -> Result<()> {
    let mut papers = load_papers()?;
    let config = read_config_file()?;

    // Check if both exist
    if !config.stacks.iter().any(|s| s.name == from) {
        bail! {"Stack name {} does not exist",from}
    };
    if !config.stacks.iter().any(|s| s.name == into) {
        bail! {"Stack name {} does not exist",into}
    };

    // Merge stack
    for (_, paper) in papers.iter_mut() {
        for stack in paper.stack.iter_mut() {
            if stack.name == from {
                stack.name = into.clone()
            }
        }
    }
    save_papers(&papers)?;
    save_config_file(&config)?;
    blog!("Merged", "stack {} into {}", from, into);

    Ok(())
}

pub fn fork(from: String, into: String) -> Result<()> {
    let mut papers = load_papers()?;
    let mut config = read_config_file()?;

    // Check if both exist
    if !config.stacks.iter().any(|s| s.name == from) {
        bail! {"Stack name {} does not exist",from}
    };

    let new_stack = Stack::new(&into, &config.stacks)?;

    for (_, paper) in papers.iter_mut() {
        //if it has from
        if paper.stack.iter().any(|s| s.name == from) {
            paper.stack.push(new_stack.clone())
        }
    }

    config.stacks.push(new_stack);
    if config.stack == from {
        config.stack = into.clone()
    }
    save_papers(&papers)?;
    save_config_file(&config)?;
    blog!("Forked", "stack {} into {}", from, into);
    blog!("Switched", "to stack: {}", into);

    Ok(())
}
