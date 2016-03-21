# lockd - Manage your lock screen

**Note:** This program is not intended for the average user using a desktop environment like Plasma, GNOME or Unity. This is intended for users of standalone window managers like i3, xmonad or awesome.

**WIP:** Please note that currently the lock command is hardcoded to `i3lock -c 000000 --nofork`

## Features
* Automatically locks screen when system gets suspended
* Will prevent suspend before screen is locked
* Has option to disable automatic suspend on lid close

## Usage

Make sure lockd starts together with your i3:
```
exec_once lockd
```

To lock your screen manually, do not start i3lock manually but use `lockctl lock` to ensure the internal state of lockd is correct.

To prevent your system from suspending when you close the lid, run `lockctl lidaction ignore`. To revert use `lockctl lidaction suspend`. You can query the status at any time using `lockctl lidaction`
