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

## Source Code

This is developed as part of my private home-lab but is mirrored in the following places:

- [Codeberg](https://codeberg.org/yatesco/home-lab-manager)
- [GitHub](https://code.322302.xyz/home-lab/home-lab-manager)
