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
