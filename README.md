![RaveEngine](repo/rave.png)

# RaveEngine-Game

Welcome to the RaveEngine project, a full rewrite of the previous client built in Godot to transition to Rust. RaveEngine is an engine built with Bevy, a graphics engine that allows us to skip the boring low-level part and get straight to what's important: the studio, the client and the server.

With Rust, we are able to build things we previously considered impossible or too hard with Godot. Many Godot features limited us for what we actually wanted with VERTEXIA. So, with the VERTIGO project (a rewrite of the website to Go), we thought, "why not rewrite the client too?". After all, it was getting hard to move around the client source code: it was as much of a mess as you think it was.

## Building

Building and running RaveEngine is fairly easy.

Head to the Rust website to install Rust and its tools:

https://rust-lang.org/tools/install/

Clone our repo, run `cargo build` (`cargo build` builds as debug, which is faster but may run worse and leave a bigger footprint in the disk; compile with the flag `--release` for a smaller file but longer compilation time), and **copy the `assets` folder you'll find the root of the project and paste it in `/target/[...]/`**. THIS IS A VERY IMPORTANT STEP!

You can now launch the executable!

As of right now, the studio, client and server can be built, but client is not as mature yet. I wanna first stabilize the studio and get it to a "proper" point before finishing out the client. 

## Designer Program

The designer program is no longer being used, in favor of fully switching to egui.
