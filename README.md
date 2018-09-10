![travis](https://travis-ci.org/ryochack/peep.svg?branch=master) 

# peep
The CLI text viewer tool that works like `less` command on small pane within the terminal window.

# Demos
## Pane on Terminal Window
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
![Follow Mode](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_follow.gif)
## Highlighting on Follow Mode
![Highlighting on Follow Mode](https://raw.githubusercontent.com/wiki/ryochack/peep/images/demo_follow_hl.gif)

# Usage
```shell
peep [OPTION]... [FILE]
```

## Options
```
-n, --lines LINES   set height of pane
-N, --print-line-number
                    print line numbers
-f, --follow        output appended data as the file grows
-h, --help          show this usage
-v, --version       show version
```

## Commands
### Commands on Normal Mode
```
(num)j         Scroll down
(num)k         Scroll up
(num)d         Scroll down half page
(num)u         Scroll up half page
(num)f         Scroll down a page
(num)b         Scroll up a page
(num)l         Scroll horizontally right
(num)h         Scroll horizontally left
(num)L         Scroll horizontally right half page
(num)H         Scroll horizontally left half page
0              Go to the beggining of line
$              Go to the end of line
g              Go to the beggining of file
G              Go to the end of file
[num]g [num]G  Go to line [num]
/pattern       Search forward in the file for the regex pattern
n              Search next
N              Search previous
q Ctrl-c       Quit
(num)+         Increment screen height
(num)-         Decrement screen height
[num]=         Set screen height to [num]
#              Toggle line number printing
ESC            Cancel
F              Toggle to follow mode
```

### Commands on Follow Mode
```
/pattern       Highlight the regex pattern
q Ctrl-c       Quit
(num)+         Increment screen height
(num)-         Decrement screen height
[num]=         Set screen height to [num]
#              Toggle line number printing
ESC            Cancel
F              Toggle to normal mode
```

# Supported Platforms
- Linux
- MacOS

# License
MIT License.
Please refer to LICENCE file.
