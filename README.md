# kmpeek

An in-progress 3D KMP editor for Mario Kart Wii, written in Rust using the [bevy](https://github.com/bevyengine/bevy) game engine. The goal is for it to be the ultimate KMP editor for Mario Kart Wii, taking inspiration from the good parts of others (such as [Lorenzi's](https://github.com/hlorenzi/kmp-editor), [KMP Cloud](https://wiki.tockdom.com/wiki/KMP_Cloud)). It is currently unusable as a KMP editor but hopefully will have its first release soon!

## How to build

Have Rust installed and the repository cloned, then run `cargo run -r` in the directory (remove `-r` if you want debug mode rather than release mode).  
Expect it to take a few minutes to compile for the first time (because it will need to download all the packages and compile them), but after that it should be pretty quick.

## State

So far, it can open a kmp file (which will load a course.kcl in the same directory if found, if not you can manually open a kcl file), which will currently only show item paths. You can edit the positions of the item points by clicking and dragging them, or changing the numbers in the 'Edit' pane. You can also save your changes.
<img width="1392" alt="image" src="https://github.com/ThomasAlban/kmpeek/assets/98399119/70a265d7-6d6d-4719-a97e-3191297ec7a4">

