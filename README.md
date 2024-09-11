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
Each reference is embedding using an LLM and you can query your entire library using natural language.
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


## Stacks
`git` has branches, `bib` has stacks.
This allows you to create separated stacks of references (`base`, `to_read` etc.) and manage them separately.
The api is done as to mimic `git` as close as possible

- `bib stack` : Lists all stacks including active one. 
- `bib stack <NAME>` : Switches to stack named `NAME`.
- `bib stack <NAME> new` : Creates new empty stack named `NAME`.
- `bib stack <NAME> drop` : Deletes the stack named `NAME`.
- `bib stack <NAME> toggle [QUERY]` : Toggles selected paper in/out of stack `NAME`.
- `bib stack <NAME> rename <NEW NAME>` : Renames stack `NAME` to `NEW NAME`.
- `bib stack <NAME> fork <FROM>` : Creates a new stack `NAME` with all papers from `FROM`.
- `bib stack <NAME> merge <FROM>` : Adds all papers of `FROM` into stack `NAME`.
- `bib unstack` : Work with all references at the same time.


## Adding references
References are always added to the current stack.
Unstack before adding if you dont want to assign them to that stack.
Or toggle the stack from the reference later.

- `bib add <ARXIV URL>` : Automatically adds reference given an arxiv url. This will be extended to included other sources.
- `bib add --pdf <PATH>` :Adds paper given a local pdf path. Prompts user to manually add a bibtex reference. 
- `bib add --web <URL>` :Adds paper given an online pdf url. Prompts user to manually add a bibtex reference. 


## Exploration

- `bib list <LENGTH>` : Prints all references in the stack. Optionally choose list size. 
- `bib open <QUERY>` : Select reference to open.

## Export

- `bib yank <QUERY>` : Copies bibtex of selected reference to clipboard. 
- `bib export <FILENAME>` : Export bibfile to standard output of all references or selected stack.
