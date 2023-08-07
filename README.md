# kmpeek

An in-progress 3D KMP editor for Mario Kart Wii, written in Rust using the [bevy](https://github.com/bevyengine/bevy) game engine. It is currently unusable as a KMP editor but hopefully will have its first release soon!

## How to build

Have Rust installed and the repository cloned, then run `cargo run -r` in the directory (remove `-r` if you want debug mode rather than release mode).  
Expect it to take a few minutes to compile for the first time (because it will need to download all the packages and compile them), but after that it should be pretty quick.

## State

So far, it can open a kmp file (which will load a course.kcl in the same directory if found, if not you can manually open a kcl file), which will currently only show item paths. You can edit the positions of the item paths by dragging the numbers in the 'Edit' panel. You can also save the kmp file which will save these changes. 
<img width="1392" alt="image" src="https://github.com/ThomasAlban/kmpeek/assets/98399119/70a265d7-6d6d-4719-a97e-3191297ec7a4">

