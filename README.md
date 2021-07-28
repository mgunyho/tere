# tere - a faster alternative to cd + ls


`tere` is a terminal file explorer. It is a faster alternative to `cd`ing and
`ls`ing. It only really does one thing: it allows you to navigate to a folder
efficiently using a TUI, and then prints the path to that folder when you exit.

`tere` aims to be minimal and simple. It should be obvious how to use it.
Navigating the file system should be efficient and require as few keystrokes as
possible. A great source of inspiration for `tere` is the "type-ahead search"
functionality found in many GUI file managers.

"Tere" means "hello" in Estonian. It also feels nice to type.

## Setup

1. Clone the repo
1. [Install the Rust toolchain](https://www.rust-lang.org/tools/install)
1. Compile the binary by running `cargo build --release` in the main folder of the repo. This creates the binary in the folder `target/release/tere-rs`.
1. Configure your shell to `cd` to the folder which `tere` prints when it exits. It has to be usually done using a function instead of an alias, since the latter only changes the working directory of the subshell.

	For bash/zsh, put this into your `.bashrc` or `.zshrc`:

	```sh
	tere() {
		local output=$(/path/to/tere/target/release/tere-rs)
		[ -n "$output" ] && cd -- "$output"
	}
	```

	For xonsh, put this in your `.xonshrc`:

	```py
	def _tere():
		@(["cd", $(/path/to/tere/target/release/tere-rs).strip()])

	aliases["tere"] = _tere
	```
	Note that xonsh v0.10 or newer is required for `tere` to work.

## User guide

The main way to navigate folders in `tere` is by using the keyboard to move the cursor around, and by typing to search.

### Keyboard shortcuts

`tere` has the following keyboard shortcuts:

| Action | Shortcut(s) |
|:---:|:---:|
|Move cursor up  | <kbd>↑</kbd> or <kbd>Alt</kbd>+<kbd>k</kbd> |
|Move cursor down| <kbd>↓</kbd> or <kbd>Alt</kbd>+<kbd>j</kbd> |
|Enter directory | <kbd>→</kbd> or <kbd>Alt</kbd>+<kbd>l</kbd> |
| Go to parent directory | <kbd>←</kbd> or <kbd>Alt</kbd> + <kbd>↑</kbd> or <kbd>Alt</kbd>+<kbd>h</kbd> |
|Exit `tere`| <kbd>Esc</kbd> or <kbd>Alt</kbd>+<kbd>q</kbd> |
|Go to home directory| <kbd>Ctrl</kbd>+<kbd>Home</kbd> or <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>h</kbd>|
|Move cursor up   by one screen| <kbd>Page Up</kbd>   or <kbd>Ctrl</kbd>+<kbd>u</kbd> |
|Move cursor down by one screen| <kbd>Page Down</kbd> or <kbd>Ctrl</kbd>+<kbd>d</kbd> |
|Move cursor to the top   | <kbd>Home</kbd> or <kbd>Alt</kbd>+<kbd>g</kbd> |
|Move cursor to the bottom| <kbd>End</kbd>  or <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>g</kbd> |

The shortcuts starting with <kbd>Alt</kbd> should be familiar to Vim users.

### Searching

To search for an item in the current folder, just type some letters. `tere` will
highlight all folders and files that match the search string. Currently the
search is case-sensitive.

While searching, moving the cursor up / down jumps between only the items that
match the search. The search string, as well as the number of matching items is
shown at the bottom of the screen.

If only one folder matches your current search, `tere` will highlight it, and
change the working directory to that folder. This way you can navigate folders
very quickly.

To stop searching, press <kbd>Esc</kbd> or erase all search characters by
pressing <kbd>Backspace</kbd>.



## License

See the `LICENSE` file.
