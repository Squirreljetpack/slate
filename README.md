# slate

>[!NOTE]
> Fork of https://github.com/znx3p0/vielsprachig, with extra functionality

# Features
- Convert between many different input and output filetypes
- Tera templating with `-t` flag
- Special output modes for generating systemd timers and quadlet files

## Supported formats
The current input options and their inferred extensions are:

| Input       | Output       | Extensions               |
|-------------|--------------|--------------------------|
| Json        | Json         | `.json`                  |
|             | PrettyJson   | `.hjson`                 |
| Yaml        | Yaml         | `.yaml`, `.yml`          |
| Cbor        | Cbor         | `.cb`, `.cbor`           |
| Ron         | Ron          | `.ron`                   |
|             | PrettyRon    | `.hron`                  |
| Toml        | Toml         | `.toml`                  |
| Bson        | Bson         | `.bson`, `.bs`           |
|             | Pickle       | `.pickle`, `.pkl`        |
|             | Bincode      | `.bc`, `.bincode`        |
|             | Postcard     | `.pc`, `.postcard`       |
|             | Flexbuffers  | `.fb`, `.flexbuffers`    |
|             | Systemd      | (use `--to systemd`)     |
|             | Quadlet      | (use `--to quadlet`)     |

