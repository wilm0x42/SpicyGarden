# SpicyGarden

Tool for crowdsourcing data about Minecraft seeds

Now I hear what you're saying:
"Why are you running a whole dang Minecraft server to collect seed data?? Why not extract the
world-generation code and boil it down to the bare necessities and run it on a GPU so you can
see a million seeds in an hour???"

You make a valid point, voice-in-my-head, and that would indeed be a really cool thing, but consider:

That's really hard! And also, the work involved in *maintaining* that code across Minecraft updates
would probably (*probably*) be pretty darn significant if, say for example, some new and exciting
biomes are introduced. But after an update is released, I want cool seeds to explore, featuring
the new biomes, *immediately*. Or at least, as soon as possible.

So rather than build some code that would take all that effort to update alongside Minecraft,
the idea of SpicyGarden is that we basically offload the work of "supporting new features" to
the wonderful folks who already do this for modded server jars with each update. This way,
we only need to update a few things here and there to utilize any updated bits in the server API;
Just toss a new server jar into the old setup, and *bam*, it's "updated." :)

# Project structure

## Desktop client application

At the root of this repo, you can find `/src`, `Cargo.toml`, and `build.rs`, which together constitute
the desktop client application for users to run, written in rust.
This application runs several Minecraft servers in parallel, collects data from them via the SpicyGarden
spigot plugin, and sends them to a central server.

Users are given a convenient GUI to configure the following parameters:
* The address of the server with which to communicate regarding seeds and their results
* The client key that will allow the user to access said server
* How many Minecraft servers should be run in parallel

## Spigot plugin

In the `spigot-plugin` folder resides the java code which collects data about a particular seed,
writing that data into a file named `SpicyGardenData.txt` at the root of a server's folder.
After writing this file, the plugin has the server shut down, signaling the desktop client to
grab the data and start the next server.

Because I (wilm0x42) am no java programmer, building is simply handled with `compile.sh`.
In order to build the plugin, you'll need to place a spigot api jar in this folder, so `compile.sh` sees it.

## Gather server

TODO

# Steps for building a package for end-users

TODO -- something something `server_template` blah blah `config.toml`
