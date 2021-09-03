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
1. Compile the binary by running `cargo build --release` in the main folder of the repo. This creates the binary in the folder `target/release/tere`.
1. Configure your shell to `cd` to the folder which `tere` prints when it exits. It has to be usually done using a function instead of an alias, since the latter only changes the working directory of the subshell.

    Note that to make the `--help` option to work, `tere` prints the help message
    to stderr instead of stdout.

    For bash/zsh, put this into your `.bashrc` or `.zshrc`:

    ```sh
    tere() {
        local result=$(/path/to/tere/target/release/tere "$@")
        [ -n "$result" ] && cd -- "$result"
    }
    ```

    For xonsh v0.10 or newer, put this in your `.xonshrc`:

    ```py
    def _tere(args):
        result = $(/path/to/tere/target/release/tere @(args)).strip()
        if result:
            @(["cd", result])

    aliases["tere"] = _tere
    ```


## User guide

The main way to navigate folders in `tere` is by using the keyboard to move the cursor around, and by typing to search.

### Keyboard shortcuts

`tere` has the following keyboard shortcuts:

| Action | Shortcut(s) |
|:---:|:---:|
|Move cursor up  | <kbd>↑</kbd> or <kbd>Alt</kbd>+<kbd>k</kbd> |
|Move cursor down| <kbd>↓</kbd> or <kbd>Alt</kbd>+<kbd>j</kbd> |
|Enter directory | <kbd>→</kbd> or <kbd>Alt</kbd>+<kbd>l</kbd> |
|Go to parent directory| <kbd>←</kbd> or <kbd>Alt</kbd> + <kbd>↑</kbd> or <kbd>Alt</kbd>+<kbd>h</kbd> |
|Exit `tere`| <kbd>Esc</kbd> or <kbd>Alt</kbd>+<kbd>q</kbd> |
|Go to home directory| <kbd>Ctrl</kbd>+<kbd>Home</kbd> or <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>h</kbd>|
|Go to root directory| <kbd>Ctrl</kbd>+<kbd>r</kbd>|
|Move cursor up   by one screen| <kbd>Page Up</kbd>   or <kbd>Ctrl</kbd>+<kbd>u</kbd> |
|Move cursor down by one screen| <kbd>Page Down</kbd> or <kbd>Ctrl</kbd>+<kbd>d</kbd> |
|Move cursor to the top   | <kbd>Home</kbd> or <kbd>Alt</kbd>+<kbd>g</kbd> |
|Move cursor to the bottom| <kbd>End</kbd>  or <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>g</kbd> |
|Change case sensitivity mode| <kbd>Alt</kbd>+<kbd>c</kbd> |

Shortcuts starting with <kbd>Alt</kbd> should be familiar to Vim users.

### Searching

To search for an item in the current folder, just type some letters. `tere` will
highlight all folders and files that match the search query.

While searching, moving the cursor up / down jumps between only the items that
match the search. The search query, as well as the number of matching items is
shown at the bottom of the screen.

If only one folder matches your current search, `tere` will highlight it, and
change the working directory to that folder. This way you can navigate folders
very quickly.

To stop searching, press <kbd>Esc</kbd> or erase all search characters by
pressing <kbd>Backspace</kbd>.

By default, the searching uses "smart case", meaning that if the query contains
only lowercase letters, case is ignored, but if there are uppercase letters, the
search is case sensitive. This can be changed with the `--ignore-case` and
`--case-sensitive` options, or with the keyboard shortcut
<kbd>Alt</kbd>+<kbd>c</kbd>.

### CLI options

You can adjust the behavior of `tere` by passing the following CLI options to it:

- `--help` or `-h`: Print a short help and all CLI options to stderr
- `--version` or `-V`: Print the version of `tere`
- `--filter-search` / `--no-filter-search`: If this option is set, hide items in the output listing that don't match the current search query.
- `--folders-only` / `--no-folders-only`: With `--folders-only`, don't show files but only folders (and symlinks pointing to folders) in the listing.
- `--smart-case` / `--ignore-case` / `--case-sensitive`: Set the case sensitivity mode. The default mode is smart case.
- `--autocd-timeout` - If only one folder matches the current search query, automatically enter it after this many milliseconds. Can also be set to `off`, which disables this behaviour.

Some options have two or more versions that override each other (for example
`--folders-only` and `--no-folders-only`). This means that whichever is passed
last is applied. This way, you can have one option as the default in your `rc`
file, but you can sometimes manually override that option when running `tere`.

## Prior art

The idea of `tere` is by no means unique. There are actually quite a few CLI
applications that attempt to make folder navigation faster. Below is a listing of
such programs. The purpose of this section is to justify the existence of `tere`
by showing how it is different from all these applications in subtle but
important ways.

If there is a program that should be mentioned here, feel free to open an issue
or pull request about it!

### Terminal file browsers

These programs are designed for basically the same task as `tere`: navigate to a
folder in the terminal and then `cd` to it.

- [Broot](https://dystroy.org/broot/) - Broot is more focused on browsing large directories, and has a more complex UI than `tere`.
- [xplr](https://github.com/sayanarijit/xplr) - Lots of features, fully customizable. Not entirely focused on navigation, has file management features. Navigation by searching requires jumping between typing and pressing arrow keys.
- [deer](https://github.com/Vifon/deer) - zsh only, searching requires extra keystrokes.
- [cdir](https://github.com/EskelinenAntti/cdir) - No Vim-like keyboard navigation. Not a standalone binary.

### Fuzzy matching and history-based navigation

These programs have a very similar goal as `tere`, to speed up filesystem
navigation. However, such programs are not suitable for exploration, as they
require that you visit a folder before you can jump to it. They also differ from
`tere` in  philosophy; `tere` aims to be deterministic, while the results of a
fuzzy match or "frecency"-based query vary over time.

- [z](https://github.com/rupa/z)
- [autojump](https://github.com/wting/autojump)
- [zoxide](https://github.com/ajeetdsouza/zoxide)
- [fasd](https://github.com/clvv/fasd)
- [jump](https://github.com/gsamokovarov/jump)
- [bashmarks](https://github.com/huyng/bashmarks)
- [goto](https://github.com/ankitvad/goto)

### Terminal file managers

There are quite a few terminal file managers, and they can often be used in the
same way as `tere`, for example using the `--choosedir` option of ranger.
However, they have a huge number of other features compared to `tere`, which
usually leads to a more complex UI and a higher learning curve. File managers are
also not entirely focused on navigation, and therefore often require extra
keystrokes to search and navigate folders. File management is not in the scope of
`tere`, so these programs are not directly comparable to it.

- [ranger](https://ranger.github.io/)
- [nnn](https://github.com/jarun/nnn)
- [Midnight Commander](https://midnight-commander.org/)
- [vifm](https://vifm.info/)
- [clifm](https://github.com/leo-arch/clifm) (C)
- [clifm](https://github.com/pasqu4le/clifm) (Haskell)
- [lf](https://github.com/gokcehan/lf)
- [fff](https://github.com/dylanaraps/fff)
- [joshuto](https://github.com/kamiyaa/joshuto)
- [hunter](https://github.com/rabite0/hunter)

### Other similar programs

- [noice](https://git.2f30.org/noice/file/README.html) - Very similar to `tere`, but there is no option to print the current directory on exit. Filtering/searching directory contents requires two extra keystrokes.
- [twilight commander](https://github.com/golmman/twilight-commander) - Main goal seems to be a folder tree browser embedded in other apps. No search. No option to go above the initial working directory.


## License

See the `LICENSE` file.
