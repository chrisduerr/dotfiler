# Dotfiler
Forked from https://github.com/matthunz/dotfiler
Manage your configuration files easily with templating and backup your dotfiles.

## Installation
  1. ```git clone https://gitlab.com/undeadleech/dotfiler.git```
  2. ```cd``` into the newly-created directory
  3. Run ```cargo build --release```
  4. Copy the executable in ```./target/release``` to your desired location

## Usage
  1. Move your template files, for example xresources, into ```dotfiler-dir/templates```
  2. Make an entry for it inside the config.toml in the ```[templates]``` category using ```templatename = path/to/real/file```
  3. Replace any text inside to be changed with {{ variablename }} inside the template file
  4. Add the variable name under ```[variables]``` in config.toml
  5. Move your dotfiles, for example compton.conf, into ```dotfiler-dir/dotfiles```
  6. Make an entry for it inside the config.toml in the ```[dotfiles]``` category using ```templatename = path/to/real/file```
  7. Run the program and set either ```--templates``` or ```--dotfiles``` if you only want to update partially
