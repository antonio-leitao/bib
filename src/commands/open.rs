use crate::utils::ui::Item;
use crate::utils::{bibfile, list};
use anyhow::Result;
use sublime_fuzzy::best_match;

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
pub fn open(query: String) -> Result<()> {
    //load items
    let bibliography = bibfile::read_bibliography()?;
    let mut items = bibfile::parse_bibliography(bibliography);
    if query.is_empty() {
        items.reverse()
    } else {
        match apply_filter(&query, &items) {
            Some(elems) => items = elems,
            None => {
                println!("\tNo itens match the query");
                return Ok(());
            }
        };
    }
    //Filter
    let first_ten: Vec<_> = items.iter().take(10).cloned().collect();
    match list::prompt_select(&first_ten)? {
        Some(index) => println!("{}", items[index].title),
        None => (),
    };
    Ok(())
}
