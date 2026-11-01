## v0.6.0 - 2026-11-01

> Note: Breaking changes

- Add `firmware` field in the vm.json definition
- Drop unecessary dependencies
- Use `-cpu host` when possible

## v0.5.0 - 2026-10-31

> Note: Breaking changes

- Add VDE network backend
- Add Linux Direct Boot
- Fix loading VMs when encoutering erros loading events file
- Make `qemu-img` a prerequisite package

## v0.4.2 - 2026-10-10

- fix: only use .xz for freebsd downloaded os files

## v0.4.1 - 2026-10-02

- Display the release version in the VM information
- Add support for FreeBSD cloud images

## v0.4.0 - 2026-09-23

- Add graphics config with different drivers
- Add file sharing between host and guest
- Remove templeos
- Add ability to choose storage interface when booting from local file
- Small layout improvements

## v0.3.0 - 2026-09-01

- Resize disks
- Fix rendering crash when drive size can not be read
- Add Bridge/Tap backend network

## v0.2.1 - 2016-08-25

- fix cursor display for remote access section
- fix applying network and port forwarding edit changes

## v0.2 - 2026-08-24

- Support configuring networks with different NIC and backends.
- Update layout

## v0.1 - 2026-08-18

First release 🎉
