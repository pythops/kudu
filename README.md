<div align="center">
  <h2> Easily manage VMs on Linux </h2>
</div>

![](https://github.com/user-attachments/assets/b3a27059-df89-4d07-ac50-871bc4df3522)

## Prerequisites

- A Linux based OS.
- `qemu` binaries, at least one of those:
  - `qemu-system-x86`
  - `qemu-system-aarch64`
  - `qemu-system-riscv`
- `xorriso` for cloudinit
- uefi firmware package

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

## Usage

```bash
$ kudu
```

## FAQ

##### Q: KVM shows Disabled or Unavailable.

Make sure the `kvm` kernel module is loaded if you are have Intel/AMD processor.

if you run `kudu` as regular user, make sure your user belongs to `kvm` group.

## Contributing

- Strict No LLM.
- Only submit a PR after having a prior issue or discussion.
- Keep PRs small and focused.

## License

GPLv3
