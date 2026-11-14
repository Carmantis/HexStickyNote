# Running this documentation

These pages are built with [MkDocs](https://www.mkdocs.org/) and the Material theme, set up the same way as HexTime's. Nothing needs installing besides [uv](https://docs.astral.sh/uv/getting-started/installation/):

```bash
cd docs        # the folder with mkdocs.yml
uvx --with mkdocs-material --with mkdocs-awesome-nav mkdocs serve --livereload --open --strict
```

The pages are Markdown files under `docs/docs/`, in the order set by `docs/docs/.nav.yml`. `--strict` stops on a broken link or a link to a heading that does not exist (`validation.anchors` in `mkdocs.yml`), so pages that build have no dead internal links.

## HexTime's pages

The [HexTime](hextime/index.md) section is a copy of the documentation in the HexTime repository, so that both apps are documented in one place. Edit those pages in HexTime, then bring them over:

```bash
scripts/sync-hextime-docs.sh        # HexTime next to this repository, or set HEXTIME_DIR
```

The script replaces `docs/docs/hextime/` with HexTime's `docs/docs/`, leaves out HexTime's own "These pages" and notes the HexTime commit it copied. Commit the result like any other change.
