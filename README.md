<div align="center">
  <h2> Easily manage VMs on Linux </h2>
</div>

![](https://github.com/user-attachments/assets/2331028e-741c-4689-9777-fb74fd72b345)

## Description

`kudu` is a TUI for creating and managing VMs on Linux. It is an alternative to GUIs like `virt-manager` or `GNOME boxes` and as opposed to these, it does not rely on libvirt

## Prerequisites

- A Linux based OS.
- `qemu` binaries, at least one of those:
  - `qemu-system-x86`
  - `qemu-system-aarch64`
  - `qemu-system-riscv`
- `xorriso` for cloudinit
- `passt`
- uefi firmware package (Optional for x86_64)

  |             | Debian/Ubuntu    | Arch (btw)   | Fedora       |
  | ----------- | ---------------- | ------------ | ------------ |
  | **x86_64**  | ovmf             | edk2-ovmf    | edk2-ovmf    |
  | **aarch64** | qemu-efi-aarch64 | edk2-aarch64 | edk2-aarch64 |
  | **riscv64** | qemu-efi-riscv64 | edk2-riscv64 | -            |

## Installation

### Binary release

You can download the pre-built binaries from the release page [release page](https://github.com/pythops/kudu/releases)

### Build from source

```shell
git clone https://github.com/pythops/kudu
cd kudu
cargo build --release
```

### On Arch Linux

```bash
pacman -S kudu
```

## Usage

```bash
$ kudu
```

## FAQ

##### Q: KVM shows Disabled or Unavailable.

Make sure the `kvm` kernel module is loaded if you have Intel/AMD processor.

if you run `kudu` as regular user, make sure your user belongs to `kvm` group. otherwise run `kudu` with sudo.

`kudu` still runs fine even if kvm is not available.

## Contributing

- Strict No LLM.
- Only submit a PR after having a prior issue or discussion.
- Keep PRs small and focused.

## License

GPLv3
