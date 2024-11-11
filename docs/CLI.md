# CLI (Command Line Interface) Mode
> [!NOTE]
> CLI Mode is for more advanced users. If you are a regular user looking for a GUI, see [README.md](../README.md)

> [!NOTE]
> The CLI mode is still under development. CLI mode does not have many features and things may change.

This mode is useful when you want to use the tool remotely (no display) and/or you want to automate the tool.

# Commands and Usage
## --help
Usage:
```
./Roblox-assets-extractor --help <command>
```
Outputs a help page showing a list of commands<br>
The `<command>` argument is optional.
When `<command>` is provided, it will show help for that commands.

## --list
Usage:
```
./Roblox-assets-extractor --list <catagory>
```
Will list files within that catagory.
`<catagory>` is not optional
`<catagory>` must be either `music`, `sounds`, `images`, `ktx`, or `rbxm`