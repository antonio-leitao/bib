use crate::settings;
use anyhow::{anyhow, Result};
use shellexpand::tilde;
use std::fs;

// fn merge() {
//     //yeets and then deletes
// }
// fn yeet(//just yeets
// ) {
// }
// fn yank(//will require multiselect in terms
// ) {
// }
//
// fn fork() {}

fn delete_stack(stack: String) -> Result<()> {
    if !stack_exists(&stack)? {
        return Err(anyhow!("Stack {} does not exist", stack.clone()));
    }
    let path = format!("~/.bib/{}", stack);
    let dir = tilde(&path).to_string();
    fs::remove_dir_all(dir)?;
    Ok(())
}
fn rename_stack(new_name: String) -> Result<String> {
    if stack_exists(&new_name)? {
        return Err(anyhow!("Stack named {} already exists", new_name));
    }
    let old_name = settings::current_stack()?;
    let old_path = tilde(&format!("~/.bib/{}", old_name)).to_string();
    let new_path = tilde(&format!("~/.bib/{}", new_name)).to_string();
    fs::rename(&old_path, &new_path)?;
    //change config
    let config = settings::Config { stack: new_name };
    settings::save_config_file(&config)?;
    Ok(old_name)
}

fn new_stack(name: String, initial: bool) -> Result<()> {
    //if it is not initial and the stack exsits
    if !initial && stack_exists(&name)? {
        return Err(anyhow!("Stack named {} already exists", name));
    }
    let create_dir = |subdir: &str| -> Result<(), std::io::Error> {
        let new_dir = tilde(&format!("~/.bib/{}/{}", name, subdir)).to_string();
        fs::create_dir_all(&new_dir)
    };
    create_dir("notes")?; // create the notes directory
    create_dir("pdf")?; // create the pdf directory
    Ok(())
}

fn stack_exists(stack: &String) -> Result<bool> {
    let stack_list = settings::list_stacks()?;
    Ok(stack_list.contains(stack))
}

fn show_stacks() {
    match (settings::current_stack(), settings::list_stacks()) {
        (Ok(current_stack), Ok(stack_list)) => {
            for stack in stack_list {
                if stack == current_stack {
                    println!("* {}", stack);
                } else {
                    println!("  {}", stack);
                }
            }
        }
        (Ok(_), Err(err)) => println!("{}", err),
        (Err(err), Ok(_)) => println!("{}", err),
        (Err(err1), Err(err2)) => println!("{}\n{}", err1, err2),
    };
}

pub fn checkout(stack: String, new: bool) -> Result<()> {
    //loads config
    if !stack_exists(&stack)? {
        if new {
            new_stack(stack.clone(), false)?;
            //create new stack
        } else {
            return Err(anyhow!("Stack does not exist, create it first"));
        };
    };
    //move to it
    let config = settings::Config { stack };
    settings::save_config_file(&config)
}

pub fn init() -> Result<()> {
    //create default config file
    let config = settings::Config {
        stack: String::from("base"),
    };
    settings::save_config_file(&config)?;
    //create base stack
    new_stack(String::from("base"), true)
}

pub fn stack(stack: String, delete: bool, rename: bool) {
    match (stack.len(), delete, rename) {
        (0, false, false) => show_stacks(),
        (_, false, false) => match new_stack(stack.clone(), false) {
            Ok(()) => println!("Created stack {}", stack),
            Err(err) => println!("Bib error: {}", err),
        },
        (_, true, false) => match delete_stack(stack.clone()) {
            //cannot delete current stack (ever)
            Ok(()) => println!("Successfully deleted {} stack", stack),
            Err(err) => println!("Bib error: {}", err),
        },
        (_, false, true) => match rename_stack(stack.clone()) {
            Ok(old_name) => println!("Stack renamed {} => {}", old_name, stack),
            Err(err) => println!("Bib error: {}", err),
        },
        _ => println!("Wrong usage. Type --help for command usage."),
    }
}
