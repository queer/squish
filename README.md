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
- File and directory bind-mounts
- Running without root
- Port bindings

### What doesn't work? aka roadmap

- Networking between containers
- Downloading layers from a remote source
- Automagically✨ unpacking zips and tarballs

### What won't be implemented?

- Persistence of container between daemon reboots
- Getting a shell in a container

## Testing

squish currently only has e2e tests. You can run them by running
`./test/test-e2e.sh`.

## Layers and binaries and images and whatnot

squish doesn't have OCI-style container images. Since what actually runs is a
bunch of bind-mounted-together SDKs, your "image" that you push is just a
binary (or tarball, or ...). When actually running a container, you specify its
layer names + tags in your `squishfile.toml`, as well as optional run + env +
port sections -- and the daemon can put all of it together to figure out what
layers are needed and what command to run. This may seem a bit
counter-intuitive at first, but it's useful for ex. adding a custom `debug`
layer to containers at creation time, ensuring you have the same tools present
in a container no matter what source layers make it up.

## Where did the name come from?

The idea started out as making something like [Flatpak](https://flatpak.org/)
for servers -- although squish has significantly diverged since then -- and so
the original working name was "squishpak," which eventually shortened into
"squish."

## Misc

`http-asm`: https://github.com/poletaevvlad/http-asm