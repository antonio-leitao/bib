use crate::settings;
use crate::utils::list::prompt_select;
use crate::utils::ui::Item;
use crate::utils::{bibfile, ui};
use anyhow::{bail, Result};

pub fn open(query: String) -> Result<()> {
    //load items
    let bibliography = bibfile::read_bibliography()?;
    let mut items = bibfile::parse_bibliography(bibliography);
    items.reverse();
    //Filter
    let first_ten: Vec<_> = items.iter().take(10).cloned().collect();
    prompt_select()
}
