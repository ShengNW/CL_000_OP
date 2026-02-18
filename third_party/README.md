# Third-Party Sources

This repository intentionally does **not** vendor large third-party codebases.

## OmniParser

OmniParser is required for real sidecar modes. Use it as a git submodule or an external clone:

```bash
# from repo root

git submodule add https://github.com/microsoft/OmniParser.git third_party/OmniParser
# or, if already added:

git submodule update --init --recursive
```

After cloning, apply local compatibility patches:

```bash
# from repo root (OmniParser checked out at third_party/OmniParser)

cd third_party/OmniParser

git apply ../../docs/patches/omniparser-local.patch
```

See `docs/patches/OMNIPARSER_LOCAL_PATCHES.md` for details.
