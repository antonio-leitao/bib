fn add(reference: String, arxiv: bool, doi:bool) {
    if arxiv {
        println!("getting bibtex from arxiv");
        add_to_library(reference);
        println!("Adding PDF and scholar id to metadata");
    } else {
        add_to_library(reference);
    }
}

fn add_to_library(bibtex: String) {
    println!("Loading entire bibliography");
    println!("Parsing bibtex");
    println!("Insert (automatically updates entries)");
}
