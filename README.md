 [![crates.io](https://img.shields.io/crates/v/peep.svg)](https://crates.io/crates/peep)
 ![test](https://github.com/ryochack/peep/workflows/test/badge.svg)

# peep
The CLI text viewer tool that works like `less` command on small pane within the terminal window.

# Demos
## Pane on Terminal Window
peep can view text file freely.

![Pane on Terminal Window](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo.gif)
## Read from Pipe
![Pipe Input](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_pipe.gif)
## Print Line Number
![Print Line Number](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_linenumber.gif)
## Resize Pane
![Resize Pane](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_resize.gif)
## Incremental Regex Search
![Incremental Regex Search](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_incsearch.gif)
## Wide Width Character Support
![Wide Width Character Support](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_wide_width_chars.gif)
## Follow Mode
peep has the follow mode that can monitor file updates and read them continuously like `tail -f` or `less +F`.  
Also, peep can switch between the normal mode and follow mode with `F` command.

![Follow Mode](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_follow.gif)
## Highlighting on Follow Mode
peep can highlight the regex word on the follow mode.

![Highlighting on Follow Mode](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_follow_hl.gif)
## Text Line Wrapping

![Text Line Wrapping](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_wrapping.gif)

# Installation
```shell
cargo install peep
```

If you don't have Rust toolchains, please refer to [The Rust Programming Language](https://www.rust-lang.org/).

Or, you can download peep binary file from [GitHub peep Releases](https://github.com/ryochack/peep/releases) :)

# Usage
```shell
peep [OPTION]... [FILE]
```

## Options
```
-n, --lines LINES        set height of pane
-s, --start START        set start line of data at startup
-t, --tab-width WIDTH    set tab width
-N, --print-line-number  print line numbers
-f, --follow             output appended data as the file grows
-h, --help               show this usage
-v, --version            show version
```

## Commands
**Format**  

```
KEY-BIND            OPERATION
```

**Example 1**  

```
0 Ctr-a             Go to the beggining of line
```
Type `0` OR `Ctrl-a`, then `Go to the beggining of line`.

**Example 2**  

```
(num)+              Increment screen height
```
`(num)` means that entering a number is optional.  
If you omit the number input, the number will be processed as 1.

**Example 3**  

```
[num]=              Set screen height to [num]
```
`[num]` means that entering a number is mandatory.


### Commands on Normal Mode
```
(num)j Ctr-j Ctr-n  Scroll down
(num)k Ctr-k Ctr-p  Scroll up
(num)d Ctr-d        Scroll down half page
(num)u Ctr-u        Scroll up half page
(num)f Ctr-f SPACE  Scroll down a page
(num)b Ctr-b        Scroll up a page
(num)l              Scroll horizontally right
(num)h              Scroll horizontally left
(num)L              Scroll horizontally right half page
(num)H              Scroll horizontally left half page
0 Ctr-a             Go to the beggining of line
$ Ctr-e             Go to the end of line
g                   Go to the beggining of file
G                   Go to the end of file
[num]g [num]G       Go to line [num]
/pattern            Search forward in the file for the regex pattern
n                   Search next
N                   Search previous
q Ctr-c             Quit
(num)+              Increment screen height
(num)-              Decrement screen height
[num]=              Set screen height to [num]
#                   Toggle line number printing
!                   Toggle line wrapping
ESC                 Cancel
F                   Toggle to follow mode
```

### Commands on Follow Mode
```
/pattern            Highlight the regex pattern
q Ctr-c             Quit
(num)+              Increment screen height
(num)-              Decrement screen height
[num]=              Set screen height to [num]
#                   Toggle line number printing
!                   Toggle line wrapping
ESC                 Cancel
F                   Toggle to normal mode
```

# Supported Platforms
- Linux
- MacOS

# License
MIT License.
Please refer to LICENSE file.
