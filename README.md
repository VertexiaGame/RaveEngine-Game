![RaveEngine](repo/rave.png)

# RaveEngine-Game

Welcome to the RaveEngine project, a full rewrite of the previous client built in Godot to transition to Rust. RaveEngine is an engine built with Bevy, a code-only game engine that allows us to skip the boring low-level part and get straight to what's important: the studio, the client and the server.

With Rust, we are able to build things we previously considered impossible or too hard with Godot. Many Godot features limited us for what we actually wanted with VERTEXIA. So, with the VERTIGO project (a rewrite of the website to Go), we thought, "why not rewrite the client too?". After all, it was getting hard to move around the client source code: it was as much of a mess as you think it was.

## Building

Building and running RaveEngine is fairly easy.

Head to the Rust website to install Rust and its tools:

https://rust-lang.org/tools/install/

Clone our repo, run `cargo build` (`cargo build` builds as debug, which is faster but may run worse and leave a bigger footprint in the disk; compile with the flag `--release` for a smaller file but longer compilation time), and **copy the `assets` folder you'll find the root of the project and paste it in `/target/[...]/`**. THIS IS A VERY IMPORTANT STEP!

You can now launch the executable!

## AI assistance disclosure

If I were to rate this on a scale of 1-5 where "1" is the least vibecoded and "5" is the most vibecoded, I'd say 2/2.5. I mostly used it for tests and debugging, as well as some visual things I couldn't figure. This sped up development considerably, and helped me find numerous amount of bottlenecks and bugs in my code. 

Though, our stance on AI is clear: all forms of AI are not allowed in our platform: one thing is using it as a tool to help and speed up development, the other is to steal a writer's job, or an artist, or a photographer.

