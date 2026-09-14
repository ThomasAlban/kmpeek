![logo_banner](https://github.com/ThomasAlban/kmpeek/blob/main/assets/logos/logo_banner.svg)

An in-progress 3D KMP editor for Mario Kart Wii. The goal is for it to be the ultimate KMP editor for Mario Kart Wii, taking inspiration from the good parts of others (such as [Lorenzi's](https://github.com/hlorenzi/kmp-editor), [KMP Cloud](https://wiki.tockdom.com/wiki/KMP_Cloud)). It is currently unusable as a KMP editor but hopefully will have its first release soon!

<img width="1412" alt="image" src="https://github.com/ThomasAlban/kmpeek/assets/98399119/ee13fe41-3acb-4912-82eb-7e187417220b">

## Alpha Release

These are the things I still need to do before an alpha release. Some of these are pretty quick and easy but others more difficult!

- [x] Finish checkpoint editor
- [x] Finish Table viewer
- [x] Route editor
- [ ] Saving back to KMP format
- [ ] Object search by id/name

If you want to help test KMPeek when these things are done (before the main first release), talk to me on discord! @thomasalban

## Future Wishlist

These are things that I want to get to in subsequent updates, once an initial version is released.

- Auto updater
- Undo/Redo
- Edit tools (such as drawing lines of points)
- Eline control handling - converting it to a setting on enemy points
- Custom object settings
- Export/Import CSV from tables
- Camera Preview
- Copy/paste
- Object KCL viewer
- Ability to open/edit SZS files directly
- Course model/object model viewer???

## Contributing

Contributing to this project is very much welcome! The project is written in Rust using the [bevy](https://github.com/bevyengine/bevy) game engine. If you want to contribute, talk to me on discord or open an issue so we can discuss it. I will also try my best to open issues where I highlight things I'd like help with. Note: since this project is in the early stages of development, my commits are often disorganised and lots of the fundamental code structure is often changed.

## How to build

Have Rust installed and the repository cloned, then run `cargo run -r` in the directory (remove `-r` if you want debug mode rather than release mode).
Expect it to take a few minutes to compile for the first time (because it will need to download all the packages and compile them), but after that it should be pretty quick.
