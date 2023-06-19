# kmpeek
An in-progress 3D KMP editor for Mario Kart Wii, written in Rust using the [bevy](https://github.com/bevyengine/bevy) game engine!  
  
## How to build
Have Rust installed and the repository cloned, then run `cargo run --release` in the directory (remove `--release` if you want debug mode).  
Expect it to take a few minutes to compile for the first time (because it will need to download all the packages and compile them), but after that it should be pretty quick.

## State
Currently it can't actually edit KMP files, though the functionality (e.g. opening, saving a KMP file) is there ready to be implemented. It can so far open KCL files and view them in different camera modes, along with editing the colours of different KCL types and hiding/showing them. 
<img width="1392" alt="Screenshot 2023-06-19 at 18 01 43" src="https://github.com/ThomasAlban/kmpeek/assets/98399119/37f1c7c4-b358-45f8-a863-150270a234c7">
