This folder has some stuff related to the demo video (as well as the demo video itself).

Instructions for recording the demo video (using `xonsh`):
1. Run the script to generate the example folders in `/tmp`:
    ```shell
    ./generate_example_folders.py
    ```
1. Set up the prompt with the following command:
    ```python
    #$PROMPT = "$ {BOLD_GREEN}{cwd}{RESET} > "
    $PROMPT = "$ "
    ```
1. Resize your terminal to 64 x 12 characters
1. To show keystrokes on the screen, run (in another therminal window) `screenkey -g $(slop) --key-mode=composed --font="Ubuntu mono bold"` (tested using screenkey 1.5, the version from apt is outdated). `slop` is used to select the window geometry.
1. (Re)move your tere history file so that it doesn't mess up the video
1. Navigate to `/tmp/tere-demo`
1. Clear the screen with ctrl+L
1. Record the video with Peek. Check the settings: record as gif, disable mouse cursor, 25 fps
1. Minimize the gif with `gifsicle -O3 -o filename-optimized.gif filename.gif`.
