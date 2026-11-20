# Install Dictionary

Set up PrimitiveOS using the [Launcher installation guide](https://github.com/PrimitiveOS/kindle-primitive-launcher/blob/main/docs/INSTALL.md). Then open **Store**, find **Dictionary**, and tap **Get**.

## Build an ARMHF package

Keep the Tools and Build checkouts beside this repository.

```sh
cargo zigbuild --offline --locked --release -p dictionary-kindle --target armv7-unknown-linux-musleabihf
sh scripts/stage.sh armhf
python3 ../doannt-kindle-build/scripts/package.py --id primitive-dictionary --dir dictionary \
  --version 0.1.1 --arch armhf --stage build/stage --license LICENSE \
  --docs-license LICENSE-DOCS --install INSTALL.md --out dist
```
