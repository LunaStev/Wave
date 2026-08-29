# Wave standard library examples

Each `.wave` file is a small, independent program focused on one standard-library area.
Run one from the repository root with:

```sh
wavec run examples/std/strings.wave
```

The filesystem example creates and removes a file in the current directory. The environment,
I/O, process, and time examples use the host operating-system provider selected by `wavec`.
