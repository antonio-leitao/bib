use crate::base::{self, MetaData};
use crate::settings;
use crate::utils::bibfile;
use anyhow::{anyhow, Result};
use shellexpand::tilde;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

fn merge_stacks(from: String, into: String) -> Result<()> {
    if !stack_exists(&from)? {
        return Err(anyhow!("Stack named {} does not exist", from));
    };
    if from == into {
        return Err(anyhow!("Stacks must be different"));
    }
    merge_bibfiles(&from, &into)?;
    merge_pdfs(&from, &into)?;
    merge_notes(&from, &into)?;
    merge_metadata(&from, &into)
}

fn append_notes(file_from: &str, file_into: &str) -> Result<()> {
    // Open the source file for reading
    let mut file_from = File::open(file_from)?;
    // Open the destination file for appending
    let mut file_into = OpenOptions::new()
        .write(true)
        .append(true)
        .open(file_into)?;
    // If the destination file is not empty, append a newline before adding content
    if file_into.metadata()?.len() > 0 {
        writeln!(file_into)?;
    }
    // Read the contents of the source file
    let mut content = Vec::new();
    file_from.read_to_end(&mut content)?;
    // Append the contents to the destination file
    file_into.write_all(&content)?;
    Ok(())
}

fn merge_notes(from: &str, into: &str) -> Result<()> {
    let source = tilde(&format!("~/.bib/{}/notes/", from)).to_string();
    let target = tilde(&format!("~/.bib/{}/notes/", into)).to_string();
    // Iterate through each file in the source directory
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_path = entry.path();

        // Check if the file exists in the target directory
        let target_file_path = Path::new(&target).join(file_path.file_name().unwrap());
        if target_file_path.exists() {
            // File exists in the target directory, append contents
            append_notes(
                &file_path.to_string_lossy(),
                &target_file_path.to_string_lossy(),
            )?;
        } else {
            // File doesn't exist in the target directory, copy it
            fs::copy(&file_path, &target_file_path)?;
        }
    }
    Ok(())
}

fn merge_pdfs(from: &str, into: &str) -> Result<()> {
    let source = tilde(&format!("~/.bib/{}/pdf/", from)).to_string();
    let target = tilde(&format!("~/.bib/{}/pdf/", into)).to_string();
    copy_dir(&source, &target)
}

fn merge_metadata(from: &str, into: &str) -> Result<()> {
    //read both metadatas,
    let from_metadata = base::read_other_metadata(from)?;
    let into_metadata = base::read_other_metadata(into)?;
    let merged_into = merge_metadata_maps(from_metadata, into_metadata);
    base::save(&merged_into, "metadata.bin")
}

fn merge_metadata_maps(
    from: HashMap<String, MetaData>,
    mut into: HashMap<String, MetaData>,
) -> HashMap<String, MetaData> {
    for (key, from_metadata) in from {
        match into.contains_key(&key) {
            false => {
                into.insert(key, from_metadata);
                ()
            }
            true => {
                let into_metadata = into.get_mut(&key).unwrap();
                // Merge pdf and notes, prioritizing non-None values
                if let Some(from_pdf) = from_metadata.pdf {
                    into_metadata.pdf = Some(from_pdf);
                };
                if let Some(from_notes) = from_metadata.notes {
                    into_metadata.notes = Some(from_notes);
                }
                // Merge last_accessed, prioritizing the most recent value (largest u64)
                if let Some(from_last_accessed) = from_metadata.last_accessed {
                    if let Some(into_last_accessed) = into_metadata.last_accessed {
                        into_metadata.last_accessed =
                            Some(from_last_accessed.max(into_last_accessed));
                    } else {
                        into_metadata.last_accessed = Some(from_last_accessed);
                    }
                }
            }
        };
    }
    into
}

fn merge_bibfiles(from: &str, into: &str) -> Result<()> {
    //read both metadatas,
    let mut from_bib = bibfile::read_other_bibliography(from)?;
    let into_bib = bibfile::read_other_bibliography(into)?;
    //into gets put in from because insert overwrites
    for entry in into_bib.into_iter() {
        from_bib.insert(entry);
    }
    bibfile::save_other_bibliography(from_bib, from)
}

// Function to recursively copy the contents of a directory
fn copy_dir(src: &str, dest: &str) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let dest_path = Path::new(dest).join(entry.file_name());

        if entry_path.is_dir() {
            fs::create_dir_all(&dest_path)?;
            copy_dir(&entry_path.to_string_lossy(), &dest_path.to_string_lossy())?;
        } else {
            fs::copy(&entry_path, &dest_path)?;
        }
    }
    Ok(())
}

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

pub fn fork(new_name: String) -> Result<String> {
    if stack_exists(&new_name)? {
        return Err(anyhow!("Stack named {} already exists", new_name));
    }
    let from = settings::current_stack()?;
    let old_path = tilde(&format!("~/.bib/{}", from)).to_string();
    let new_path = tilde(&format!("~/.bib/{}", new_name)).to_string();
    // Create the new stack directory
    fs::create_dir(&new_path)?;
    // Copy the contents of the old stack to the new stack
    copy_dir(&old_path, &new_path)?;
    //change config
    let config = settings::Config { stack: new_name };
    settings::save_config_file(&config)?;
    Ok(from)
}

pub fn merge(from: String) -> Result<()> {
    let into = settings::current_stack()?;
    merge_stacks(from.clone(), into)?;
    delete_stack(from)
}

pub fn yeet(into: String) -> Result<()> {
    let from = settings::current_stack()?;
    merge_stacks(from, into)
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
