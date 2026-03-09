# home-lab-manager

I run a home-lab and find it very...bity. Whilst there are some _amazing_ tools out there, they take different approaches to configuring them, and you end up repeating a bunch of things.

For example:

- your `compose.yaml` for `service1`
- the network routing for your proxy to expose it
- the monitoring parameters for that service in your monitoring software
- the backup solution

and so on.

The purpose of this project is _not_ to replace any tools, really, it is more to automate the configuration of those tools.

It's premise is that you can define your infrastructure once, and this will configure the moving pieces for you automatically.

## Goals

The following are explicit goals of this project:

- Make home-labing fun and safe.
- Maintain the fun parts of messing around and experimenting different services.

## Non-goals

However, this absolutely will avoid:

- Prevent the fun aspects of digging around in each service.
- Becoming _all or nothing_. It will always be opt-in.

## AI Coded?

I've been a professional Software Engineer for over 30 years, so yes, my code is "safe" and I make every decision.

However, I'm learning that AI is very useful for "doing what I want" but not so good at "find the best way to do it".

So yes, I do use AI but for very small things which I then review. At no point does AI make any _meaningful_ decisions.

## Development

### Bootstrap

This project uses [mise](https://mise.jn.sh/) to manage the development environment.

To bootstrap your development environment and install pre-commit hooks:

```bash
mise run bootstrap
```

This will:

- Install pre-commit if not already present
- Set up pre-commit hooks from `.pre-commit-config.yaml` (if present)

# Standards

All code must follow the coding standards (enforced by [Biome](https://biomejs.dev/) and the [Rust Project](https://www.rust-lang.org/) via `cargo fmt`) before committing.

## Source Code

This is developed as part of my private home-lab but is mirrored in the following places:

- [Codeberg](https://codeberg.org/yatesco/home-lab-manager)
- [GitHub](https://code.322302.xyz/home-lab/home-lab-manager)
