# kmpeek

An in-progress 3D KMP editor for Mario Kart Wii, written in Rust using the [bevy](https://github.com/bevyengine/bevy) game engine!

## How to build

Have Rust installed and the repository cloned, then run `cargo run --release` in the directory (remove `--release` if you want debug mode).  
Expect it to take a few minutes to compile for the first time (because it will need to download all the packages and compile them), but after that it should be pretty quick.

## State

Currently it can't actually edit KMP files, though the functionality (e.g. opening, saving a KMP file) is there ready to be implemented. It can so far open a KMP file (which will load a course.kcl file in the same directory if avaliable), and I currently have it displaying item paths as a test. It also has multiple camera modes and customisation options for the view.
<img width="1392" alt="Screenshot 2023-06-22 at 09 17 50" src="https://github.com/ThomasAlban/kmpeek/assets/98399119/45583174-614b-4729-b343-79ff7908ccde">
