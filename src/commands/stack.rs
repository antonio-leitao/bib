use crate::settings;
use crate::utils::{bibfile, fmt};
use anyhow::{bail, Result};
use shellexpand::tilde;
use std::fs;
use std::io::Write;
use termion::color;

fn merge_stacks(from: String, into: String) -> Result<usize> {
    if !stack_exists(&from)? {
        bail!("Stack named {} does not exist", from);
    };
    if !stack_exists(&into)? {
        bail!("Stack named {} does not exist", into);
    };
    if from == into {
        bail!("Stacks must be different");
    }
    merge_bibfiles(&from, &into)
}

fn merge_bibfiles(from: &str, into: &str) -> Result<usize> {
    // Read both metadatas
    let from_bib = bibfile::read_other_bibliography(from)?;
    let mut into_bib = bibfile::read_other_bibliography(into)?;
    // Track the count of elements inserted
    let mut inserted_count = 0;
    // Insert entries from 'from_bib' into 'into_bib'
    for entry in from_bib.into_iter() {
        if into_bib.insert(entry).is_none() {
            inserted_count += 1;
        }
    }
    // Save 'into_bib' after merge
    bibfile::save_other_bibliography(into_bib, into)?;
    // Return the count of elements inserted
    Ok(inserted_count)
}

fn delete_stack(stack: String) -> Result<()> {
    if !stack_exists(&stack)? {
        bail!("Stack {} does not exist", stack.clone());
    }
    let current = settings::current_stack()?;
    if stack == current {
        bail!("Cannot delete active stack");
    }
    let path = format!("~/.bib/{}.bib", stack);
    let dir = tilde(&path).to_string();
    fs::remove_file(dir)?;
    fmt::delete(stack);
    Ok(())
}

fn rename_stack(new_name: String) -> Result<()> {
    if stack_exists(&new_name)? {
        bail!("Stack named {} already exists", new_name);
    }
    let old_name = settings::current_stack()?;
    let old_path = tilde(&format!("~/.bib/{}.bib", old_name)).to_string();
    let new_path = tilde(&format!("~/.bib/{}.bib", new_name)).to_string();
    fs::rename(&old_path, &new_path)?;
    //change config
    let config = settings::Config {
        stack: new_name.clone(),
    };
    settings::save_config_file(&config)?;
    fmt::rename(old_name, new_name);
    Ok(())
}

fn new_stack(name: String, initial: bool) -> Result<()> {
    //if it is not initial and the stack exsits
    if !initial && stack_exists(&name)? {
        bail!("Stack named {} already exists", name);
    }
    let new_bib = tilde(&format!("~/.bib/{}.bib", name)).to_string();
    let mut file = fs::File::create(&new_bib)?;
    file.write_all(b"")?;
    if !initial {
        fmt::new(name);
    }
    Ok(())
}

fn stack_exists(stack: &String) -> Result<bool> {
    let stack_list = settings::list_stacks()?;
    Ok(stack_list.contains(stack))
}

fn show_stacks() -> Result<()> {
    let current_stack = settings::current_stack()?;
    let stack_list = settings::list_stacks()?;

    for stack in stack_list {
        if stack == current_stack {
            println!(
                "* {}{}{}",
                color::Fg(color::Green),
                stack,
                color::Fg(color::Reset)
            );
        } else {
            println!("  {}", stack);
        }
    }
    Ok(())
}

pub fn checkout(into: String, new: bool) -> Result<()> {
    //loads config
    if !stack_exists(&into)? {
        if new {
            new_stack(into.clone(), false)?;
            //create new stack
        } else {
            bail!("Stack does not exist, run 'bib stack {} to create'", into);
        };
    };
    //pretend to move into it
    let from = settings::current_stack()?;
    //actualy move into it
    let config = settings::Config {
        stack: into.clone(),
    };
    settings::save_config_file(&config)?;
    //inform
    fmt::switch(from, into);
    Ok(())
}

pub fn init() -> Result<()> {
    //create default config file
    let config = settings::Config {
        stack: String::from("base"),
    };
    settings::save_config_file(&config)?;
    //create base stack
    new_stack(String::from("base"), true)?;
    fmt::init();
    Ok(())
}

pub fn fork(new_name: String) -> Result<()> {
    if stack_exists(&new_name)? {
        bail!("Stack named {} already exists", new_name);
    }
    let from = settings::current_stack()?;
    let old_path = tilde(&format!("~/.bib/{}.bib", from)).to_string();
    let new_path = tilde(&format!("~/.bib/{}.bib", new_name)).to_string();
    // Copy the contents of the old stack to the new stack
    fs::copy(&old_path, &new_path)?;
    fmt::new(new_name.clone());
    let bib = bibfile::read_other_bibliography(&from)?;
    fmt::yeet(from.clone(), None, new_name.clone(), bib.len());
    //change config
    let config = settings::Config {
        stack: new_name.clone(),
    };
    settings::save_config_file(&config)?;
    fmt::switch(from, new_name);
    Ok(())
}

pub fn merge(from: String) -> Result<()> {
    let into = settings::current_stack()?;
    let number = merge_stacks(from.clone(), into.clone())?;
    fmt::merge(from.clone(), into, number);
    delete_stack(from)
}

pub fn yeet(into: String) -> Result<()> {
    let from = settings::current_stack()?;
    let number = merge_stacks(from.clone(), into.clone())?;
    fmt::yeet(from, None, into, number);
    Ok(())
}

pub fn yank(from: String) -> Result<()> {
    let into = settings::current_stack()?;
    let number = merge_stacks(from.clone(), into.clone())?;
    fmt::yank(from, None, into, number);
    Ok(())
}

pub fn stack(stack: String, delete: bool, rename: bool) -> Result<()> {
    match (stack.len(), delete, rename) {
        (0, false, false) => show_stacks(),
        (_, false, false) => new_stack(stack, false),
        (_, true, false) => delete_stack(stack),
        (_, false, true) => rename_stack(stack),
        _ => bail!("Wrong usage. Type --help for command usage."),
    }
}
