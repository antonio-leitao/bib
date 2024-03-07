# `bib`

<img src='assets/bib_banner.png' width='250px' align="right" style="float:right;margin-left:0pt;margin-top:10pt;"></img>

`bib` is a command line bibliography manager and explorer. Git meets bib.
The main power of `bib` is to allow to add notes connected to each reference.

#### Contents
  - [Installation](#installation)
  - [Usage](#usage)
    - [Stacks](#stacks)
    - [Managing References](#managing-references)
    - [Managing Notes](#managing-notes)
    - [Exploration](#exploration)
    - [Export](#export)
    
# Installation
```bash
# macOS or Linux
brew tap antonio-leitao/taps
brew install bib
```
# Usage
`bib` allows for adding and importing bib references, both manually and from the internet.
It also allows for associating a PDF file and a file with Notes.
This is all done inside a powerfull `git`-like branch managing system.
To get started run:

```text
bib init
```

## Stacks
`git` has branches, `bib` has stacks.
This allows you to create separated stacks of references (`base`, `to_read` etc.) and manage them separately.
The api is done as to mimic `git` as close as possible

#### Managing Stacks

- `bib stack` : Lists all stacks including active one. 
- `bib stack <NAME>` : Creates new empty stack named `NAME`.
- `bib stack --delete <STACK>` : Deletes target stacked named `STACK`, cannot delete active one.
- `bib stack --rename <NAME>` : Renames active stack to `NAME`. 

#### Switching stacks

- `bib checkout <STACK>` : Switches to target stack.
- `bib checkout --new <STACK>` : Creates new stack `NEW_STACK` and switches to it.

#### Merging

Merging is always a non-simmetric operation.
A merge/pull/push of stack A into B means to add all references that exist in A into B.
Duplicate entries are updated.

- `bib yank <STACK>` : Pulls from target stack **Target stack is not deleted**. 
- `bib yeet <STACK>` : Pushes current stack into target stack. Current stack is **not** deleted.
- `bib merge <STACK>` : Pulls target stack into current stack. **Target stack is deleted**.
> [!CAUTION]
> Merge is a yank that deletes the target.
- `bib fork <NEW_STACK>` : Duplicates active stack under new name, switches to new stack.

## Managing references

- `bib add` : Prompts user to manually add a bibtex reference. Optionally a `url` to allow `bib` to open/download the pdf.
- `bib add <ARXIV URL>` : Automatically adds reference given an arxiv url. This will be extended to included other sources.


## Exploration

- `bib search <QUERY>` : Interactive search over all references in stack. Allows for opening and deleting. Papers can be furthered filtered in UI.


## Export

- `bib search <FILENAME>` : Export references of current stack into a bibfile named `<FILENAME>` defaults to the current directory.
