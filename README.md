# cohost Static Site Generator

## What is this?

This is a tool to convert a [cohost](https://cohost.org/) user export into a static website. For example: [https://cohost.liglig.art](https://cohost.liglig.art/).

The output is intended to be used by the [Zola](https://www.getzola.org/) static site generator.
See the [`zola`](zola) directory for an empty cohost-themed configuration.

## Usage

1. Edit [`zola/config.toml`](zola/config.toml) to set desired site `base_url`.
2. Run `cohost-static --projects handle1,handle2,handle3 <unzipped-export-dir>`. This will populate `zola/content` and `zola/static`.
3. Run `zola --root zola build`. This will generate the full static site in the `zola/public` directory.

**Note:** If the `--projects` argument isn't provided then all available projects will be used.

## License

Licensed under the **MIT License**. See [`LICENSE.txt`](LICENSE.txt)