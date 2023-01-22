# DOTZ

Dotz is a simple tool to manage dotfiles write in rust.

![dotz](docs/images/dotz_image.jpg)

# Index

- [Installation](#installation)
- [Usage](#usage)
  - [Example of folder structure](#example-of-folder-structure)

# Installation

Easy to install with cargo just use the following command:

```bash
cargo install dotz
```

or if you want to update dotz is the same command.

# Usage

For use dotz you need to transfer all your dotfiles that you want manage to a folder and just run.

> ! Note: The folder must follow the same hierarchy as the one you want them to be installed in.

## Example of folder structure:

```bash
dotfiles/
├── .config/
│   ├── alacritty/
│   │   └── alacritty.yml
│   └── bspwm/
│       └── bspwmrc
├── .zshrc
├── .vimrc

Home/
├── .config/
│   ├── alacritty/
│   │   └── alacritty.yml
│   └── bspwm/
│       └── bspwmrc
├── .zshrc
└── .vimrc
```

And then run the following command:

```bash
dotz [path to dotfiles folder]
```

Also you can specify the path where you want to install the dotfiles:

```bash
dotz [path to dotfiles folder] [path to install dotfiles]
```

Or you can use repo command for install the dotfiles from a repository of github (You need to have git installed).

```bash
dotz repo [github repository url]
```

the default path where the repository will be cloned is "$HOME/.dotfiles" but you can change it with the following command:

```bash
dotz repo [github repository url] [path to dotfiles folder]
```

> Node: In this case the folder need to be empty or not exist (dotz will create the folder).

If you want to install the dotfiles in a different path you can use the following command:

```bash
dotz repo [github repository url] [path to dotfiles folder] [path to install dotfiles]
```

for all the commands you can use the following options:

```
-h, --help        Show help message
-f, --force       Force overwrite of existing files
-s, --static      Create static files
-v, --version     Show version
--verbose         Show verbose output
```
