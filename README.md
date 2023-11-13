# Bib

Command line bibliography management. Git meets bib.

# Stack

```
bib stack
```
Lists all stacks, marking current one.

```
bib stack <NAME>
```
Creates new empty stack.

```
bib stack --delete <STACK>
```
Deletes target stack.

```
bib stack --rename <NEW_NAME>
```
Renames current stack to `new_name`.

# Checkout 

```
bib checkout <STACK>
```
Switches to target stack.

```
bib checkout --new <NEW_NAME>
```
Creates new stack `NEW_STACK` and swicthes to it.

# Exploring

```
bib search <QUERY>
```
Interactive search over all references in stack. Allows for opening and deleting.

```
bib search --online <QUERY>
```
Interactive search over all references online matching query. Allows for adding references to stack.

# Merging

```
bib merge <STACK>
```
Pulls target stack into current branch. Target stack is deleted.

```
bib yeet <STACK>
```
Pushes current branch into target stack. Current stack is **not** deleted.

```
bib yank <STACK>
```
Brings references from target stack into current branch.

```
bib fork <NEW_NAME>
```
Duplicates current stack under new name.

# Add

```
bib add --arxiv <arxiv url>
```
Retrieves PDF and bibtext from arxiv url.

```
bib add --path <pdf_path>
```
Prompts user for bibtext, copies pdf from provided location.

```
bib add --url <pdf_url>
```
Prompts user for bibtext, attempts to download pdf from provided url.


