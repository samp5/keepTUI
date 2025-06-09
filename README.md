
`keep` is a TUI for to-do lists.

Loosely inspired by Google Keep, or rather, what I use Google Keep for: small daily to-dos structuring what I want to accomplish each week.

### Features
- **VIM-ish** - Simple vim emulator to bring vim motions(ish) to editing your to-do lists
- **Local notes** - I often found I wanted separate lists for different projects, e.g. in my `keepTUI` project directory
    - When run with `-l` or `--local`, `keep` will first look in the current working directory for a `.keep` folder, and, if found, will display and edit this data.
#### In progress:
- [ ] Fuzzy search over note content and titles
- [ ] Note tags and collections

### Shell completions
Generate shell completion information with `keep --generate-completions=<SHELL>` for `fish`, `bash`, or `zsh` 

```bash
$ keep --generate-completions=fish | source
```

### Configuration
Use `keep --dump-config` to get a sample configuration with all keys set to their default values. Colors can be set with hex values (`#FFFFFF`), ANSI terminal color indices, or common color names (`red`). Configuration keys are generally self-explanatory. 

Default values are used for any key not specified by the configuration file.
