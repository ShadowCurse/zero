# Zero
Zero is a basic rendering engine

## Usage
Example scene can be found in [bin](/bin)

To launch it run
```bash
$ cargo run --release --bin example_scene
```
To launch headless example run
```bash
$ cargo run --release --features headless --bin example_scene_headless
```

## Libraries Used
Main libraries on which Zero relies on:
- [wgpu](https://github.com/gfx-rs/wgpu/tree/v0.12): modern / low-level / cross-platform graphics library inspired by Vulkan
- [winit](https://github.com/rust-windowing/winit): Cross-platform window creation and management in Rust
- [cgmath](https://github.com/rustgd/cgmath): A linear algebra and mathematics library for computer graphics

## Examples
<img src="./img/zero_v0.0.2.png" width="400">
<img src="./img/zero_v0.0.1.png" width="400">
