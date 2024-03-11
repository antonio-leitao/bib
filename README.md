<p align="center">
  <img src='assets/logo.svg' width='250px' align="center"></img>
</p>

<div align="center">
<h3 max-width='200px' align="center"><code>bib</code></h3>
  <p><i>Manage your entire bibliography from the command line<br/>
  Biblatex meets Git<br/>
  Built with Rust</i><br/></p>
  <p>
   <img alt="Static Badge" src="https://img.shields.io/badge/homebrew-black?style=for-the-badge&logo=homebrew&logoColor=white">

  </p>
</div>

#

`bib` is a command line bibliography manager and explorer. Git meets bib.
The main power of `bib` is to allow to create and manage multiple stacks (branches).

#### Contents
  - [Installation](#installation)
  - [Usage](#usage)
    - [Stacks](#stacks)
    - [Managing References](#managing-references)
    - [Exploration](#exploration)
    - [Export](#export)
    
# Installation
```bash
# macOS or Linux
brew tap antonio-leitao/taps
brew install bib
```
# Usage
`bib` allows for adding and importing bib references, both manually and automatically from arXiv.
It also allows for associating a PDF url.
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

- `bib status` : Print basic stack statistics.
- `bib list <LENGTH>` : Prints all references in the stack. Optionally choose list size. 
- `bib open <QUERY>` : Select reference to open.
- `bib search <QUERY>` : Interactive search over all references in stack. Allows for opening and deleting and moving between stacks. Papers can be furthered filtered in UI. Suitable for in depth organization of stacks.


## Export

- `bib export <FILENAME>` : Export references of current stack into a bibfile named `<FILENAME>` defaults to the current directory.
