#!/usr/bin/env python3

# a quick script to generate folders for the demo

import os
from pathlib import Path
from shutil import rmtree

# dummy that appears as a folder instead of a file in the demo
D = { "dummy": {} }

# basic idea: the final path spells out "tere is a fast folder navigator for the terminal"
TREE = {
    "Documents": D,
    "Downloads": D,
    "Pictures": D,
    "Secrets": D,
    "tere": {
        # idea: "tere is a program to help navigating folders in the terminal"
        ".tere": D,
        "is a": {
            # idea: "demonstrate type-ahead search"
            ".you can quickly jump to a folder by typing": D,
            "fast": {
                # idea: "type-ahead search is fast"
                ".if there is only one match, it is selected automatically": D,
                "folder": {
                    # more type-ahead search
                    ".tere is designed to minimize the number of keystrokes": D,
                    "navigator": {
                        # more
                        ".and to make exploration as effortless as possible": D,
                        "for": {
                            # arrow key demo
                            ".you can also navigate using the arrow keys": D,
                            "the": {
                                # vim keybindings demo
                                ".or vim-like key bindings": D,
                                "terminal": {
                                    "okay that's the end of the demo": D,
                                },
                                "triangle": D,
                                "tolerate": D,
                                "theorist": D,
                                "teenager": D,
                                "threaten": D,
                                "talented": D,
                                "twilight": D,
                            },
                            "map": D,
                            "kid": D,
                            "owl": D,
                            "bat": D,
                            "run": D,
                            "use": D,
                            "van": D,
                            "win": D,
                        },
                        "fog": D,
                        "far": D,
                        "few": D,
                        "set": D,
                        "bet": D,
                        "run": D,
                        "eat": D,
                        "low": D,
                        "bad": D,
                        "put": D,
                        "hit": D,
                    },
                    "naughtier": D,
                    "narrator": D,
                    "narrator": D,
                    "notorious": D,
                    "nightmare": D,
                    "neighbour": D,
                    "nonsense": D,
                    "necklaces": D,
                    "notebooks": D,
                    "national": D,
                    "nominate": D,
                },
                "fodder": D,
                "forger": D,
                "boulder": D,
                "wonder": D,
                "collar": D,
                "favor": D,
                "border": D,
            },
            "blast": D,
            "cast": D,
            "last": D,
            "mast": D,
            "past": D,
            "vast": D,
        },
        "little": D,
        "program": D,
        "to": D,
        "help with": 0,
        "navigating folders in": 0,
        "the terminal": 0,
    },
}


def generate_tree(cur_path: Path, subtree: dict):
    for k, v in subtree.items():
        if not v:
            # v is empty, treat k as a filename
            final_path = cur_path / k
            print(str(final_path))
            final_path.touch()

        else:
            subpath = cur_path / k
            print(str(subpath) + "/")
            subpath.mkdir()

            generate_tree(cur_path / k, v)

root_path = Path("/tmp/tere-demo")
if root_path.exists():
    print(f"rm {str(root_path)}")
    rmtree(str(root_path))
root_path.mkdir()
generate_tree(root_path, TREE)
