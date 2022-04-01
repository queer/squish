# squish

squish is a novel rootless-only container runtime. The name is never
capitalised, so if it were to come at the start of a sentence, it would still
be written `squish`.

## Why?

squish was born out of frustration with existing container runtimes. They all
have their drawbacks (rootful, lack of port rebinds, cache-unfriendliness, slow
container starts, ...) that lead to usage thereof being a frustrating
experience at best. squish attempts to address these shortcomings.

Preliminary testing shows that squish can get a viable Alpine-rootfs container
up in ~5ms. This is an **initial** figure, and will change over time.

![](https://cdn.mewna.xyz/2021/11/21/vTiW66Bnc2Png.png)

### Things that haven't been implemented but are planned

squish also avoids the typical OCI-style container images. The goal of squish
is that the only "image" you deploy is a binary, and a manifest with the list
of SDKs it uses. At container runtime, the various SDKs are bind-mounted into
the container dynamically. Both the rootfs and all SDK layers are mounted
read-only.

## Roadmap

Feature               | Description                                | State
----------------------|--------------------------------------------|------
Alpine                | Read-only Alpine rootfs                    | ✔️
Networking            | slirp4netns networking + port binds        | ✔️
Mounts                | Bind-mount files and directories ro and rw | ✔️
Rootless              | Containers without root                    | ✔️
Container networking  | Inter-container networking                 | TODO
Cgroups               | Resource limitations etc                   | TODO
Systemd cgroup driver | Set up cgroups via systemd                 | TODO
Layer downloads       | Download layers via HTTP                   | TODO
Seccomp               | Syscall filtering                          | TODO
Dynamic port rebinds  | (Re)bind container ports at runtime        | TODO

### What won't be implemented?

- Persistence of containers between daemon reboots
- Getting a shell in a container

## Local development

1. Set up your environment by running `./setup.sh`
2. Run the daemon with `env RUST_BACKTRACE=1 RUST_LOG=debug cargo run -p daemon`
3. Create a container with `cargo run -p cli -- create test/squishfiles/default.toml`
4. You did it! Read the cli source to learn more commands

## Testing

squish currently only has e2e tests. You can run them by running
`./test/test-e2e.sh`.

## Where did the name come from?

The idea started out as making something like [Flatpak](https://flatpak.org/)
for servers -- although squish has significantly diverged since then -- and so
the original working name was "squishpak," which eventually shortened into
"squish."

## Misc

`http-asm`: https://github.com/poletaevvlad/http-asm