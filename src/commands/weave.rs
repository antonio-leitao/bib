pub fn weave(query: Option<String>) {
    match query {
        Some(query) => {
            println!("'loom weave' was used, query is: {query:?}")
        }
        None => {
            println!("No query Provided")
        }
    }
}
