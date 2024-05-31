# kmpeek

An in-progress 3D KMP editor for Mario Kart Wii. The goal is for it to be the ultimate KMP editor for Mario Kart Wii, taking inspiration from the good parts of others (such as [Lorenzi's](https://github.com/hlorenzi/kmp-editor), [KMP Cloud](https://wiki.tockdom.com/wiki/KMP_Cloud)). It is currently unusable as a KMP editor but hopefully will have its first release soon!

<img width="1412" alt="image" src="https://github.com/ThomasAlban/kmpeek/assets/98399119/ee13fe41-3acb-4912-82eb-7e187417220b">

## Alpha Release

These are the things I still need to do before an alpha release:
- [ ] Finish checkpoint editor
- [ ] Finish Table viewer
- [ ] Route editor
- [ ] Edit tools (such as drawing lines of points)
- [ ] Undo/Redo
- [ ] Saving back to KMP format
- [ ] Auto updater

If you want to help test KMPeek when these things are done (before the main first release), talk to me on discord! @thomasalban


## Contributing

Contributing to this project is very much welcome! The project is written in Rust using the [bevy](https://github.com/bevyengine/bevy) game engine. If you want to contribute, talk to me on discord or open an issue so we can discuss it. I will also try my best to open issues where I highlight things I'd like help with. Note: since this project is in the early stages of development, my commits are often disorganised and lots of the fundamental code structure is often changed.

## How to build

Have Rust installed and the repository cloned, then run `cargo run -r` in the directory (remove `-r` if you want debug mode rather than release mode).  
Expect it to take a few minutes to compile for the first time (because it will need to download all the packages and compile them), but after that it should be pretty quick.
