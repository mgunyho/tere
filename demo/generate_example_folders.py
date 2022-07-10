#!/usr/bin/env python3

# a quick script to generate folders for the demo

import os
from pathlib import Path
from shutil import rmtree

# dummy that appears as a folder instead of a file in the demo
D = { "dummy": {} }

# basic idea: the final path spells out "tere is pretty nice"
# basic idea: the final path spells out "tere is a fast folder navigator for the terminal"
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
            "fast": {
                # idea: "type-ahead search is faster than ls+cd"
                ".if there is only one match, it is selected automatically": D,
                "folder": {
                    # more type-ahead search
                    ". it is designed to minimize the number of keystrokes": D,
                    "navigator": {
                        # more
                        ".and to make exploration as effortless as possible": D,
                        "for": {
                            ".you can also navigate using the arrow keys": D,
                            "the": {
                                ".or vim-like key bindings": D,
                                "terminal": {
                                    "okay that's the end of the demo": D,
                                    #"you can exit with esc (it's a bit easiear than vim)": D,
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
                            "sip": D,
                            "owl": D,
                            "oil": D,
                            "old": D,
                            "fat": D,
                            "run": D,
                        },
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
                    #"negatives": D,
                },
                "fodder": D,
                "forger": D,
                "boulder": D,
                "wonder": D,
                "collar": D,
                "favor": D,
                "border": D,

                #"convenient and": D,
                #"fast way to": D,
                #"navigate folders - check it out": D,
                #"this": D,
                ##"late": D,
                #"pony": D,
                #"pity": D,
                #"skip": D,
                #"want": D,
                #"mail": D,
                #"navigate": D,
                #"it can be": D,
                #"used to": D,
                #"browse": D,
                #"nice": { "okay that's the end of the demo": {}},
                #"wouldn't": D,
                #"you agree": D,
            },
            #"and you can jump to a folder quickly, like this": D,
            #".you can jump to a folder quickly, like this": D,
            ".you can quickly jump to a folder by typing": D,
            "blast": D,
            "cast": D,
            "last": D,
            "mast": D,
            "past": D,
            "vast": D,

            # things that ~rhyme with 'pretty'
            #"betty": D,
            #"city": D,
            ##"dirty": D,
            #"gritty": D,
            #"jetty": D,
            #"nifty": D,
            #"kitty": D,
            #"patty": D,
            ##"petty": D,
            #"witty": D,
        },
        "little": D,
        "program": D,
        "to": D,
        "help with": 0,
        "navigating folders in": 0,
        #"folder": 0,
        #"navigation in": 0,
        "the terminal": 0,
        #"terminal navigation": 0,
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
