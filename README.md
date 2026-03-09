# Astray

A* pathfinding gone astray. An interactive tool for generating longest non-self-intersecting paths within geometric boundaries, with SVG and DXF export for laser cutting.

## Install

### Quick install (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/theodore-evans/astray/main/install.sh | bash
```

### Pre-built binaries

Go to [Actions](https://github.com/theodore-evans/astray/actions), click the latest successful build, and download the artifact for your platform:

- **astray-macos-arm64** — macOS Apple Silicon (M1/M2/M3/M4)
- **astray-macos-x64** — macOS Intel
- **astray-linux-x64** — Linux

Then unzip and run:

```bash
chmod +x astray-*
./astray-macos-arm64 --shape circle --longest
```

On macOS you may need to remove the quarantine flag:
```bash
xattr -d com.apple.quarantine astray-macos-*
```

### Build from source

Requires [Rust](https://rustup.rs/).

```bash
git clone https://github.com/theodore-evans/astray.git
cd astray
cargo build --release
```

Binary will be at `target/release/astar`.

## Usage

```bash
cargo run -- --shape circle --longest
```

### Shapes

`--shape` accepts: `rect`, `circle`, `hex`, `star`, `donut`, `diamond`, `heart`, `maze`

### Controls

| Key | Action |
|-----|--------|
| **1-7** | Select shape (Rect, Circle, Hex, Star, Donut, Diamond, Heart) |
| **LMB drag** | Draw wall line |
| **Ctrl+LMB drag** | Erase wall line |
| **LMB click** | Toggle single wall cell |
| **RMB** | Set start/goal (alternates) |
| **C** | Clear painted walls |
| **D** | Toggle diagonal movement |
| **L** | Toggle link segments |
| **M** | Toggle markers |
| **W** | Toggle wall visibility |
| **S** | Export SVG + DXF |
| **Q / Esc** | Quit |

### Export

Press **S** to save. Files are written to `output/` as `{shape}-{n}.svg` and `{shape}-{n}.dxf`, auto-incrementing to avoid overwrites.

DXF files have named layers (CUT, ENGRAVE, LINK, MARKER) that can be toggled independently in CAD software like Rhino.

## License

MIT
