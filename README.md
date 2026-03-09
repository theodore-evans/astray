# Astray

A* pathfinding gone astray. An interactive tool for generating longest non-self-intersecting paths within geometric boundaries, with SVG and DXF export for laser cutting.

## Install

### Pre-built binaries

Download from the latest [Actions build](https://github.com/theodore-evans/astray/actions):

**macOS:**
```bash
gh run download -n astray-macos-arm64 --repo theodore-evans/astray  # Apple Silicon
gh run download -n astray-macos-x64 --repo theodore-evans/astray    # Intel
chmod +x astray-macos-*
xattr -d com.apple.quarantine astray-macos-*  # remove Gatekeeper quarantine
```

**Linux:**
```bash
gh run download -n astray-linux-x64 --repo theodore-evans/astray
chmod +x astray-linux-x64
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
