# lockd - Manage your lock screen

**Note:** This program is not intended for the average user using a desktop environment like Plasma, GNOME or Unity. This is intended for users of standalone window managers like i3, xmonad or awesome.

lockd will use a configuration file at $HOME/.config/lockd/main.cfg. This will be overridable using a command line switch in the future

## Features
* Automatically locks screen when system gets suspended
* Will prevent suspend before screen is locked
* Has option to disable automatic suspend on lid close

## Usage

Make sure lockd starts together with your i3:
```
execonce lockd
```

To lock your screen manually, do not start i3lock manually but use `lockctl lock` to ensure the internal state of lockd is correct.

To prevent your system from suspending when you close the lid, run `lockctl lidaction ignore`. To revert use `lockctl lidaction suspend`. You can query the status at any time using `lockctl lidaction`

### Automatic Screen Locking

set up xautolock: `xautolock -locker 'lockctl perform_autolock'`

To temporarily turn off automatic screen locking: `lockctl autolock off`. Once that has been run, `lockctl perform_autolock` turns onto a no-op
