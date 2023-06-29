# tere - a faster alternative to cd + ls


`tere` is a terminal file explorer. It is a faster alternative to using `cd`
and `ls` to browse folders in your terminal. `tere` only really does one thing: it
provides a TUI for efficiently navigating to a folder, and then prints the path
to that folder when you exit. By configuring your shell to `cd` to the printed
folder, you can move around in your filesystem very quickly.

![A gif showing what using tere looks like](./demo/tere-demo-2022-07-10-2027-e2-O3.gif)

Note that `tere` is not a file _manager_, it
can only be used to browse folders, not to create, rename or delete them.

`tere` aims to be minimal and simple. It should be obvious how to use it.
Navigating the file system should be efficient and require as few keystrokes as
possible. A great source of inspiration for `tere` is the "type-ahead search"
functionality found in many GUI file managers.

"Tere" means "hello" in Estonian. It also feels nice to type.

## Setup

To use `tere` for changing directories, you need to install it, and then
configure your shell to `cd` to the folder `tere` prints when it exits. Here's
how to do it:

### Step 1: Obtain a copy of `tere`

This can be done in various ways:

- Download the [latest release](https://github.com/mgunyho/tere-rs/releases).
- Install `tere` with [Homebrew](https://brew.sh) by running `brew install tere`.
- Install `tere` with [Nix](https://nixos.org/) by running `nix-env -i tere`.
- Install `tere` with [Cargo](https://www.rust-lang.org/tools/install) by running `cargo install tere`.
- Install `tere` with [Pacman](https://wiki.archlinux.org/title/pacman) by running `pacman -S tere`.
- Install `tere` with [Scoop](https://scoop.sh) by running `scoop install tere`.
- Build from source, see [below](#hacking).

### Step 2: Configure your shell to `cd` using `tere`

`tere` only prints a folder when it exits. To make your shell actually `cd` to this folder, you have to define a function or alias, since the working directory cannot be changed by a subprocess. See instructions for your shell below.

<details>
<summary>Bash/Zsh</summary>

Put this in your `.bashrc` or `.zshrc`:

```sh
tere() {
    local result=$(command tere "$@")
    [ -n "$result" ] && cd -- "$result"
}
```
</details>

<details>
<summary>fish</summary>

Put this in your `config.fish`:

```sh
function tere
    set --local result (command tere $argv)
    [ -n "$result" ] && cd -- "$result"
end
```
</details>

<details>
<summary>Xonsh</summary>

Put this in your `.xonshrc` (Xonsh v0.10. or newer is required):

```py
def _tere(args):
    result = $(tere @(args)).strip()
    if result:
        cd @(result)

aliases["tere"] = _tere
```
</details>

<details>
<summary>PowerShell</summary>

Put this in your `$PROFILE`:

```powershell
function Invoke-Tere() {
    $result = . (Get-Command -CommandType Application tere) $args
    if ($result) {
        Set-Location $result
    }
}
Set-Alias tere Invoke-Tere
```
</details>

<details>
<summary>Windows Command Prompt (CMD)</summary>

Put this in a batch script file called `tere.bat` in a folder included in your `PATH` environment variable such as `C:\Windows`:

```batch
@echo off

rem set the location/path of the tere executable here...
SET TereEXE=C:\path\to\tere.exe

FOR /F "tokens=*" %%a in ('%TereEXE% %*') do SET OUTPUT=%%a
IF ["%OUTPUT%"] == [""] goto :EOF
cd %OUTPUT%
```
Note that if you want to make `tere` work with *both* PowerShell and CMD, you should *not* put `tere.exe` to a location that is in your `PATH`, because then the `.exe` will be run instead of the `.bat`. Place `tere.exe` somewhere that is not in your `PATH`, and use the full path to the exe in both the `.bat` file and in the PowerShell `$PROFILE`.
</details>

<details>
<summary>Nushell</summary>

Put this in your `config.nu`:

```nushell
def-env tere [] {
    let result = (^tere)
    if ($result | str length) > 0 {
        cd $result
    }
}
```
</details>

If `tere` is not in your `PATH`, use an absolute path to the tere binary in your shell config file. For example, for Bash/Zsh, you would need to replace `local result=$(command tere "$@")` with `local result=$(/path/to/tere "$@")`, or for PowerShell, replace `(Get-Command -CommandType Application tere)` with `C:\path\to\tere.exe`.

If instructions for your shell are missing, feel free to send a pull request that includes them!

### Step 3: That's it

The next time you open a new shell, the command `tere` should work. You can of course rename the shell function/alias to whatever you like. The shell configuration also acts as a config file for `tere`, just add the options you want (see `tere --help`).

### Supported platforms

`tere` works on Linux, Windows and macOS. For Linux and Windows, binaries are provided in the [releases](https://github.com/mgunyho/tere-rs/releases). For Mac, you can install using Homebrew or Cargo, or build from source.

If you get libc errors on Linux, try the `musl` version.

## User guide

### Basic navigation

You can navigate folders in `tere` by moving the cursor around and by typing to search. By default, the cursor can be moved up or down using the arrow keys, and pressing <kbd>Enter</kbd> or the right arrow <kbd>→</kbd> to enter the highlighted folder. You can move to the parent folder by pressing <kbd>Enter</kbd> on the parent folder item `..`, or with the left arrow <kbd>←</kbd>. Once you have navigated to the folder you want, exit `tere` by perssing <kbd>Esc</kbd>. If you have configured your shell correctly, your shell's current working directory should now be set to that folder.

### Keyboard shortcuts

`tere` has the following keyboard shortcuts by default:

| Description | Default shortcut(s) | Action name |
|:---:|:---:|:--:|
|Enter directory under cursor | <kbd>Enter</kbd> or <kbd>→</kbd> or <kbd>Alt</kbd>-<kbd>↓</kbd> or <kbd>Alt</kbd>-<kbd>l</kbd> or if not searching, <kbd>Space</kbd> | `ChangeDir` |
|Go to parent directory| <kbd>←</kbd> or <kbd>Alt</kbd>-<kbd>↑</kbd> or <kbd>Alt</kbd>-<kbd>h</kbd> or if not searching, <kbd>Backspace</kbd> or <kbd>-</kbd> | `ChangeDirParent` |
|Go to home directory| <kbd>~</kbd> or <kbd>Ctrl</kbd>-<kbd>Home</kbd> or <kbd>Ctrl</kbd>-<kbd>Alt</kbd>-<kbd>h</kbd>| `ChangeDirHome` |
|Go to root directory| <kbd>/</kbd> or <kbd>Alt</kbd>-<kbd>r</kbd>| `ChangeDirRoot` |
|Move cursor up  | <kbd>↑</kbd> or <kbd>Alt</kbd>-<kbd>k</kbd> | `CursorUp` |
|Move cursor down| <kbd>↓</kbd> or <kbd>Alt</kbd>-<kbd>j</kbd> | `CursorDown` |
|Move cursor up   by one screen| <kbd>Page Up</kbd>   or <kbd>Ctrl</kbd>-<kbd>u</kbd> or <kbd>Alt</kbd>-<kbd>u</kbd> | `CursorUpScreen` |
|Move cursor down by one screen| <kbd>Page Down</kbd> or <kbd>Ctrl</kbd>-<kbd>d</kbd> or <kbd>Alt</kbd>-<kbd>d</kbd> | `CursorDownScreen` |
|Move cursor to the top   | <kbd>Home</kbd> or <kbd>Alt</kbd>-<kbd>g</kbd> | `CursorTop` |
|Move cursor to the bottom| <kbd>End</kbd>  or <kbd>Alt</kbd>-<kbd>Shift</kbd>-<kbd>g</kbd> | `CursorBottom` |
|Erase a character from the search | <kbd>Backspace</kbd> if searching | `EraseSearchChar` |
|Clear the search | <kbd>Esc</kbd> if searching | `ClearSearch` |
|Toggle filter search| <kbd>Alt</kbd>-<kbd>f</kbd> | `ChangeFilterSearchMode` |
|Change case sensitivity mode| <kbd>Alt</kbd>-<kbd>c</kbd> | `ChangeCaseSensitiveMode` |
|Change gap search mode| <kbd>Ctrl</kbd>-<kbd>f</kbd> | `ChangeGapSearchMode` |
|Change sorting mode| <kbd>Alt</kbd>-<kbd>s</kbd> | `ChangeSortMode` |
|Refresh current directory| <kbd>Ctrl</kbd>-<kbd>r</kbd>| `RefreshListing` |
|Show help screen| <kbd>?</kbd> | `Help` |
|Exit `tere`| <kbd>Esc</kbd> or <kbd>Alt</kbd>-<kbd>q</kbd> | `Exit` |
|Enter directory and exit `tere`| <kbd>Alt</kbd>-<kbd>Enter</kbd> or <kbd>Ctrl</kbd>-<kbd>Space</kbd> | `ChangeDirAndExit` |
|Exit `tere` without changing directory| <kbd>Ctrl</kbd>-<kbd>c</kbd> | `ExitWithoutCd` |

Some of the shortcuts starting with <kbd>Alt</kbd> should be familiar to Vim users.

#### Customizing keyboard shortcuts

All of the keyboard shortcuts listed above can be customized using the `--map` (or `-m`) CLI option. Keyboard mappings can be either of the form `--map key-combination:action` or `--map key-combination:context:action`, where `key-combination` is a key combination, such as `ctrl-x`, `action` is a valid action name (for example `Exit` or `ChangeDir`, see the table above or `--help` for a full list of actions), and the optional `context` specifies the context in which the mappling applies (for example `Searching` and `NotSearching`, see `--help`). To remove a mapping, use `--map key-combination:None`. Multiple mappings can be made by providing `--map` multiple times, or by using a comma-separated list of mappings: `--map combination1:action1,combination2:action2`.

For further details and examples, see the output of `--help`.

### Searching

To search for an item in the current folder, just type some letters. `tere` will incrementally highlight all folders and files that match the search query.

While searching, moving the cursor up or down jumps between only the items that match the search. The search query, as well as the number of matching items is shown at the bottom of the screen.

If only one folder matches your current search, `tere` will highlight it, and change the working directory to that folder. This way you can navigate folders very quickly.

To stop searching, press <kbd>Esc</kbd> or erase all search characters by pressing <kbd>Backspace</kbd>.

By default, the searching uses "smart case", meaning that if the query contains only lowercase letters, case is ignored, but if there are uppercase letters, the search is case sensitive. This can be changed with the `--ignore-case` and `--case-sensitive` options, or with the keyboard shortcut <kbd>Alt</kbd>-<kbd>c</kbd> by default.

Additionally, in the default search mode, "gap search" (sometimes also known as fuzzy search) is enabled. This means that the search matches any folder or file name as long as it starts with the same character as the search query, and contains the rest of the query characters, even if there are other characters between them. For example, searching for `dt` would match both `DeskTop` and `DocumenTs`. With the `--gap-search-anywhere` option, the first character of the query doesn't have to match the first character of a folder/file name. The gap search can be disabled with the `--normal-search` and `--normal-search-anywhere` options, which only allow matching consecutive characters, either from the start or anywhere within the folder/file name, respsectively. The gap search behavior can also be changed with the keyboard shortcut <kbd>Ctrl</kbd>-<kbd>f</kbd> by default. See the output of the `--help` option for further details.

### Mouse navigation

Although `tere` is mainly keyboard-focused, it is also possible to navigate using the mouse. To maximize compatibility, mouse support is off by default, and has to be enabled with the option `--mouse=on`. With the mouse enabled, you can change to a folder by clicking on it, and move to the parent folder by right-clicking.

### CLI options

You can adjust the behavior of `tere` by passing the following CLI options to it:

- `--help` or `-h`: Print a short help and all CLI options. Note that the output goes to stderr, to not interfere with `cd` ing in the shell functions defined during the setup.
- `--version` or `-V`: Print the version of `tere`. This also goes to stderr.
- `--filter-search` or `-f` / `--no-filter-search` or `-F`: If `--filter-search` is set, show only items that match the current search query in the listing. Otherwise all items are shown in the listing while searching (this is the default behavior).
- `--folders-only` or `-d` / `--no-folders-only` or `-D`: With `--folders-only`, don't show files but only folders (and symlinks pointing to folders) in the listing.
- `--smart-case` or `-S` / `--ignore-case` or `-i` / `--case-sensitive` or `-s`: Set the case sensitivity mode. The default mode is smart case, which is case insensitive if the query contains only lowercase letters and case sensitive otherwise.
- `--gap-search` or `-g` / `--gap-search-anywhere` or `-G` / `--normal-search` or `-n` / `--normal-search-anywhere` or `-N`: Configure whether to allow matches with gaps in them (see above).
- `--sort name` / `created` / `modified`: Change the sorting order of the listing.
- `--autocd-timeout` - If the current search matches only one folder, automatically change to that folder after this many milliseconds. Can also be set to `off`, which disables this behaviour.
- `--history-file`: To make browsing more convenient, `tere` saves a history of folders you have visited to this file in JSON format. It should be an absolute path. Defaults to `$CACHE_DIR/tere/history.json`, where `$CACHE_DIR` is `$XDG_CACHE_HOME` or `~/.cache`. Set to the empty string `''` to disable saving the history. Note that the history reveals parts of your folder structure if it can be read by someone else.
- `--mouse=on` or `--mouse=off`: Enable or disable navigating with the mouse. If enabled, you can left-click to enter folders and right-click to go to the parent folder. Off by default.

Some options have two or more versions that override each other (for example `--folders-only` and `--no-folders-only`). For such options, whichever is passed last wins. This way, you can have one option as the default in your shell's `rc` file, but you can sometimes manually override that option when running `tere`.

## Similar projects

The idea of `tere` is by no means unique. There are actually quite a few CLI
applications that attempt to make folder navigation faster. Below is a
non-exhaustive list of such programs. The purpose of this section is to justify
the existence of `tere` by showing how it is different from all these
applications in subtle but important ways.

If there is a program that should be mentioned here, feel free to open an issue
or pull request about it!

### Terminal file browsers

These programs are designed for basically the same task as `tere`: navigate to a
folder in the terminal and then `cd` to it.

- [Broot](https://dystroy.org/broot/) - Broot is more focused on browsing large directories, and has a more complex UI than `tere`.
- [xplr](https://github.com/sayanarijit/xplr) - Lots of features, fully customizable. Not entirely focused on navigation, has file management features. Navigation by searching requires jumping between typing and pressing arrow keys.
- [deer](https://github.com/Vifon/deer) - zsh only, searching requires extra keystrokes.
- [cdir](https://github.com/EskelinenAntti/cdir) - Basically exactly the same idea as `tere`, but in written in Python. Doesn't have Vim-like keyboard navigation, and it's not a standalone binary.
- [llama](https://github.com/antonmedv/llama) - Very similar to `tere`, written in Go.
- [sdn](https://git.janouch.name/p/sdn) - Also very similar to `tere`, even in terms of the UI as well. Type-ahead search mode is not the default, searching requires a couple of extra keystrokes.

### Fuzzy matching and history-based navigation

These programs have a very similar goal as `tere`, to speed up filesystem
navigation. However, these kinds of programs are not well suited for
exploration, as they require that you visit a folder before you can jump to it.
They also differ from `tere` in philosophy; `tere` aims to be deterministic,
while the results of a fuzzy match or "frecency"-based query vary depending on
your previous queries.

- [z](https://github.com/rupa/z)
- [autojump](https://github.com/wting/autojump)
- [zoxide](https://github.com/ajeetdsouza/zoxide)
- [fasd](https://github.com/clvv/fasd)
- [jump](https://github.com/gsamokovarov/jump)
- [bashmarks](https://github.com/huyng/bashmarks)
- [goto](https://github.com/ankitvad/goto)
- [fzf](https://github.com/junegunn/fzf)
- [skim](https://github.com/lotabout/skim)

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

## Hacking

To compile `tere` from source, follow the standard procedure:

1. [Install the Rust toolchain](https://www.rust-lang.org/tools/install)
1. `git clone git@github.com:mgunyho/tere.git`
1. `cd tere`
1. Run `cargo build` (`--release` for the release version)

This will place the `tere` binary in the folder `target/debug`, or `target/release` if you used `--release`.

New features should go on the `develop` branch before they are released, and they should be mentioned in `CHANGELOG.md`.

To set up cross-compilation for other platforms (e.g. when making a release), run (on Ubuntu):
```shell
# Support for linux without dependence on glibc
rustup target add x86_64-unknown-linux-musl

# Windows support
sudo apt install gcc-mingw-w64
rustup target add x86_64-pc-windows-gnu

# ARM (raspberry pi) support
sudo apt install gcc-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-gnu

# NOTE: macOS is not available
```
Then, the `build-release.sh` script should work.

For further details, see the [`rustup` guide](https://rust-lang.github.io/rustup/cross-compilation.html), and the [`rustc` platform support page](https://doc.rust-lang.org/nightly/rustc/platform-support.html), and consult your favourite search engine for help on cross-compilation.

### Making a new release

Here's a checklist of things to do for a new release.

- Run `cargo test` and verify that all tests pass
- Update version in `Cargo.toml`
- Run `cargo build` so that `Cargo.lock` is also updated, and make a commit with the updated versions.
- Update the release date in `CHANGELOG.md` and commit it
- `git checkout master && git merge --no-ff develop`. The commit title should be "Version X.Y.Z" and the commit message should contain the changelog.
- `git tag vX.Y.Z`
- `git push && git push --tags`. Also make sure that the latest version of `develop` is pushed.
- `sh ./build-release.sh` to build the binaries. They are zipped and placed in the folder `release/`.
- Upload binaries to github and copy-paste the changelog from the commit message
- `cargo publish` to upload to crates.io


## License

Copyright 2023 András Márton Gunyhó. Licensed under the EUPL, see the `LICENSE` file.
