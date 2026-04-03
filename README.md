# <img src="https://pixel-eagle.com/assets/logo-DckYRgGf.svg" alt="Pixel Eagle" height="40"> pixeleagle-cli

CLI for visual regression testing with Pixel Eagle.

## Install

From the install script:

```bash
curl -fsSL https://pixel-eagle.com/install.sh | sh
```

From [GitHub releases](https://github.com/vleue/pixeleagle-cli/releases), download the binary for your platform.

From source:

```bash
cargo install --git https://github.com/vleue/pixeleagle-cli
```

## Usage

```bash
# Create a run
RUN_ID=$(pixeleagle new-run --metadata '{"branch":"main"}')

# Upload screenshots
pixeleagle upload-screenshots $RUN_ID screenshot1.png screenshot2.png

# Compare with a previous run and wait for results
pixeleagle compare-run $RUN_ID --with-run $PREVIOUS_RUN_ID --wait --print-details
```

Set `PIXEL_EAGLE_TOKEN` to your project's token.
