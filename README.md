# nvmetcfg - NVMe Target Configuration CLI for Linux
nvmetcfg is a tool to configure the NVMe Target (`nvmet`) subsystem for Linux.

- Simple CLI commands with friendly help texts and error messages.
- Show and configure Ports, Subsystems and Namespaces.
- Strict checks to ensure only valid configuration gets applied.
- Save the state in a simple yaml file and restore it later.
- Full integration tests and unit tests ensure it functions as advertised.

## Usage
nvmetcfg primarily provides the `nvmet` CLI tool.
If you provide no arguments, you will see the main usage and subcommands:
```
NVMe-oF Target Configuration CLI

Usage: nvmet <COMMAND>

Commands:
  port       NVMe-oF Target Port Commands
  subsystem  NVMe-oF Target Subsystem Commands
  namespace  NVMe-oF Target Subsystem Namespace Commands
  state      NVMe-oF Target Subsystem State Management Commands
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help

```

This should be familiar to anyone who has used `clap`-based CLIs before.

Each of the subcommands contains a usage as well. When in doubt, run `help <subcommands>`.
Don't know what arguments to provide to `port add`? Run `help port add`.

Keep in mind that you *need* at least the `nvmet` module loaded.
Given that this tool is modifying the kernel sysfs, manipulating the state requires running as `root`.


Alternatively, this project also provides a library for integration into other projects.
In this case, consider the `nvmet` binary source code in `src/bin/nvmet/` as the example.

## TCP example
This is a simple example showing:
- Creation of a 1G file `/tmp/test.img`.
- Setup of `/dev/loop0` to point to `/tmp/test.img`
- Creation of a simple single-namespace subsystem `nqn.2023-11.sh.tty:example-test-loop` which is backed by `/dev/loop0`.
- Creation of a NVMe over TCP port which binds to all IPv4 addresses on port `4420`, the standard port.
- Saving the current nvmet subsystem state, clearing it and restoring it.

```console
# whoami
root
# modprobe nvmet 
# fallocate -l 1G /tmp/test.img
# losetup /dev/loop0 /tmp/test.img
# nvmet subsystem add nqn.2023-11.sh.tty:example-test-loop
# nvmet namespace add nqn.2023-11.sh.tty:example-test-loop 1 /dev/loop0
# nvmet subsystem show
Configured subsystems: 1
Subsystem: nqn.2023-11.sh.tty:example-test-loop
	Allow Any Host: true
	Number of Namespaces: 1
	Namespaces: 1
# nvmet namespace show nqn.2023-11.sh.tty:example-test-loop
Number of Namespaces: 1
Namespace 1:
	Enabled: true
	Device Path: /dev/loop0
	Device UUID: 75db752f-9c96-4e3b-ae08-e0feebe08138
	Device NGUID: 00000000-0000-0000-0000-000000000000
# nvmet port add 1 tcp 0.0.0.0:4420
# nvmet port add-subsystem 1 nqn.2023-11.sh.tty:example-test-loop
# nvmet port show
Configured ports: 1
Port 1:
	Type: Tcp(0.0.0.0:4420)
	Subsystems: 1
		nqn.2023-11.sh.tty:example-test-loop

# mkdir -p /etc/nvmetcfg
# nvmet state save /etc/nvmetcfg/state.yaml
Sucessfully written current state to file.
# nvmet state clear
Sucessfully cleared configuration: 2 state changes.
# nvmet state restore /etc/nvmetcfg/state.yaml
Sucessfully applied saved state: 2 state changes.
# nvmet state restore /etc/nvmetcfg/state.yaml
No changes made: System state has no changes compared to saved state.
```

Obviously, the `show` commands are not necessary for functionality, only for visual verification.
If any of the commands fail, error messages will be printed.

For an example of the config file, check out [examples/tcp.yaml](examples/tcp.yaml).
It should match what you'd get if running this, other than the random serial number.

## Installation
### NixOS
On NixOS, you can install the tool by adding `github:vifino/nvmetcfg` as a flake.
It provides the well-known system-specific `packages` attribute, specifically `.packages.${system}.nvmetcfg`.
Currently, the flake supports x86, x86_64 and aarch64 systems.

Alternatively, with NixOS or plain Nix on Linux, you can just temporarily run `nvmet` for testing:
```shell
modprobe nvmet
nix run github:vifino/nvmetcfg help
```

Of course, automatically loading the kernel modules is the way to go for actual usage:
```nix
{
  boot.kernelModules = [ "nvmet" ];
}
```

### From Source
Just the usual lifecycle of Cargo-based Rust projects:
```console
$ cargo build --release
```

Running it is simple:
```console
$ sudo modprobe nvmet
$ sudo ./target/release/nvmet 
```

`sudo` is used here, but that's just to get the point across that you probably need to run this as root if you actually wanna modify the state. 

## Comparison with `nvmet-cli`
`nvmet-cli` is a Python project that has been there since the beginning.
It is written by a maintainer of the kernel `nvmet` subsystem itself and does the job.

However, there are some things that I ([vifino](https://github.com/vifino)) disliked:
- More or less a 1:1 mapping of config to sysfs instead of attempting to simplify redundancy.
- Instead of being a tool you mostly use within your shell, it has it's own interactive shell.
  - This makes not just scripting harder, but also quick state inspection.. *clunky*.
- The code and project structure seem suboptimal, in addition to being sparsely maintained.
- I don't like interpreted languages running as `root` and touching my kernel.

This project is my attempt to create a tool that is more like the incredible `nvme-cli`.
It reached my goal of feature parity and integrates better in my workflows.
