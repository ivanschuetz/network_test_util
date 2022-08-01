# Setup

### Install Rust:

Unix: https://www.rust-lang.org/tools/install
Windows: https://forge.rust-lang.org/infra/other-installation-methods.html#other-ways-to-install-rustup

Note that this project uses Unix shell scripts. Windows 10+ seems to support it. If something doesn't work, you might need to try a VM (with Linux or MacOS) or Cygwin.

### Clone sandbox

```
git clone git@github.com:algorand/sandbox.git
```

### Add sandbox to path

Ensure that the `sandbox` script in the sandbox repo's root is in path (instructions depend on OS).

### Start sandbox

```
sandbox up dev -v
```

`up` starts the sandbox, `dev` uses dev mode, which allows to execute transactions nearly instantly (instead of the ~4 seconds it takes on a regular network), and `-v` is for verbose output.

### Create project's environment file:

From this repo's root:

Copy `./sample.env` and rename it in `.env`. Adjust variable's values if needed.

### Run project:

From this repo's root:

```
cargo run
```

This will setup the test environment: it resets the sandbox network (this removes all data) and configures (funding, asset opt-ins, etc) the test accounts. Note that everything is local, so you can call this when and as often as you need.

The frontend uses the sandbox network, with the same connection data as this project (assuming that the WASM dependency was built with this configuration, which should normally be the case), so it should immediately reflect the changes (after refreshing).
