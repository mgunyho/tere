## 1.4.0 (unreleased)

- Check if the app is being run for the first time (based on whether the history file exists, and a bit of additional simple logic), and if so, prompt the user to confirm that they have updated their shell configuration (Github #83)
- Footer error message is not cleared when search is updated, only when changing the folder

## 1.3.1 (2022-12-06)

- Fixed a bug where `?` didn't show the help screen on Windows by default

## 1.3.0 (2022-10-15)

- Add option to toggle filter search mode while the app is running, the default shortcut is `Alt-f` (Github #59)
- Added option to sort directory listing by creation and modification date in addition to the name. Can be changed with the `--sort` CLI option and with the default shortcut `Alt-s`. (Thanks @joshrdane, Github #64)
- Added "normal search anywhere" search mode with the `--normal-search-anywhere` or `-N` CLI option.
- The `--no-gap-search` option has been renamed to `--normal-search`. The old option will still work, but it will display a warning
- Home / end (i.e. `CursorTop` / `CursorBottom`) now work also while searching
- Bugfixes related to drawing (Github #65)
   - Fixed last character of rows not being drawn, both in the main screen and help screen (at least on some terminal emulators)
   - Fixed broken bolding in the help screen if the wrapping happens at `/`
   - Fixed broken highlighting if the last character of a symlink is matched in a search
   - Fixed drawing bug when info message is longer than the terminal window width
- Improved scrolling / cursor position behavior in filter search mode
- Fix footer not updating when pressing home / end

## 1.2.0 (2022-09-11)

The biggest new feature is the possibility to map custom keyboard shortcuts, using a syntax like `--map key-combination:action`.

Other improvements:

- Add keyboard mapping to select the folder under the cursor and exit immediately. The default keyboard shortcuts are `Alt-Enter` and `Ctrl-Space`. (Github #39)
- Fix exiting with `ctrl-c` with `--mouse=on` (Github #45)
- Update dependencies
- Lots of small updates to readme

## 1.1.0 (2022-07-15)

- Typing '~' goes to the home folder
- Added fish example to readme

## 1.0.0 (2022-07-15)

Initial release.
