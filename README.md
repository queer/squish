# squish

squish is a novel rootless-only container runtime. The name is never
capitalised, so if it were to come at the start of a sentence, it would still
be written `squish`.

## Why?

Preliminary testing shows that squish can get a viable Alpine-rootfs container
up in ~5ms. This is an **initial** figure, and will change over time.

### Things that haven't been implemented but are planned

squish also avoids the typical OCI-style container images. The goal of squish
is that the only "image" you deploy is a binary, and a manifest with the list
of SDKs it uses. At container runtime, the various SDKs are bind-mounted into
the container dynamically. Both the rootfs and all SDK layers are mounted
read-only.

## What works?

- **Read-only** Alpine rootfs
- Basic slirp4netns-based networking
- Listing running containers

### What doesn't work?

- Persistence, lol
- Any sort of volumes
- Inter-container networking

## Layers and binaries and images and whatnot

squish doesn't have OCI-style container images. Since what actually runs is a
bunch of bind-mounted-together SDKs, your "image" that you push is just a
binary (or tarball, or ...) that has an accompanying `layerfile.toml`. The
`layerfile` is just a manifest of layers that are to be used at runtime, and
optionally a run-specification of what command to run by default. When actually
running a container, you specify its layer name + tag in your `squishfile.toml`
-- and optionally any other layers you want pulled -- as well as optional run +
env + port sections -- and the daemon can put all of it together to figure out
what layers are needed and what command to run. This may seem a bit
counter-intuitive at first, but it's useful for ex. adding a custom `debug`
layer to containers at creation time, ensuring you have the same tools present
in a container no matter what source layers make it up..;

## Misc

`http-asm`: https://github.com/poletaevvlad/http-asm