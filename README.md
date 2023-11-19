# `bib`

<img src='assets/bib_banner.png' width='250px' align="right" style="float:right;margin-left:0pt;margin-top:10pt;"></img>

`bib` is a command line bibliography manager and explorer. Git meets bib.
The main power of `bib` is to allow to add notes connected to each reference.

#### Contents
  - [Installation](#installation)
  - [Setting Up Semantic Scholar](#setting-up-semantic-scholar)
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
# Setting Up Semantic Scholar
> **Note**
> to allow integration with Semantic Scholar you need to have an API key and set it as environment variable.

```bash
# set api key as environemnt variable
SCHOLAR_API=your_smenatic_scholar_api_key
```

# Usage
`bib` allows for adding and importing bib references, both manually and from the internet.
It also allows for associating a PDF file and a file with Notes.
This is all done inside a powerfull `git`-like branch managing system.

## Stacks
`git` has branches, `bib` has stacks.
This allows you to create separated stacks of references (`base`, `to_read` etc.) and manage them separately.
The api is done as to mimic `git` as close as possible

#### Managing Stacks

```
bib stack
```
Lists all stacks, including current stack.

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

#### Swithing stacks

```
bib checkout <STACK>
```
Switches to target stack.

```
bib checkout --new <NEW_NAME>
```
Creates new stack `NEW_STACK` and switches to it.

#### Merging
All Notes and PDFs are mantained through stack `merge` and `yeet` favoring always the pulled branch.
New notes are appended to existing ones and updates pdf files get replaced

```
bib merge <STACK>
```
Pulls target stack into current stack. **Target stack is deleted**.

```
bib yeet <STACK>
```
Pushes current stack into target stack. Current stack is **not** deleted.

#### Forking

```
bib fork <NEW_NAME>
```
Duplicates current stack under new name, switches to new stack.

## Managing references

```
bib add --arxiv <arxiv url>
```
> **Warning**
> Requires Semantic Scholar to be set up.

Retrieves PDF and bibtext from arxiv url.

```
bib add --path <pdf_path>
```
Prompts user for bibtext, copies pdf from provided location.

```
bib add --url <pdf_url>
```
Prompts user for bibtext, attempts to download pdf from provided url.



## Exploration

```
bib search <QUERY>
```
Interactive search over all references in stack. Allows for opening and deleting.
Papers can be furthered filtered in UI.
From here you can add/edit notes attached to each reference.

```
bib search --online <QUERY>
```
> **Warning**
> Requires Semantic Scholar to be set up.

Interactive search over all references online matching query. Allows for adding references to stack.

```
bib peek
```
Shows most recently accessed papers for quit acces to PDF and Notes.

## Managing Notes
You can add and edit notes associated to any existing reference inside the commands `search` and `peek`.


## Export
```text
bib export
```
Export references of current stack into a bifile in the current directory.
