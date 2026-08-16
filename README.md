# rlbuddy

An overlay for Rocket League which displays the names/ranks of everybody in your lobby without tabbing out, and a lot of other stuff too!

rlbuddy uses a widget system, where each feature has its own widget which can be opened/closed, so you can customize it however you want! For example, if you don't use Spotify, you can just hide its widget, and it'll stay out of your way.

- Shows ranks of everyone in your lobby and previous lobbies
- Can display information from Tracker, too if you want to know more
- Customizable MMR graphs and stat tracking (W/L, etc.)
- Opens using a configurable hotkey/button
- Can be partially transparent!
- Local playback control and Discord integration
- Sets up the stats api for you <3

> My setup:

> <img src="readme-images/demo.jpeg">

## Features/widgets

#### Lobby

<img src="readme-images/lobby.jpeg">

This is the primary widget, showing your current lobby along with some of each player's stats (their score). It also uncensors each player's name and colors them with their team color. Your name is automatically guessed and highlighted.

The three ranks under each player are their competitive 1s, 2s, and 3s ranks, with the icon to the left being their rank in the current mode.

It also shows each player's platform, and allows you to open their profiles in TRN. For Switch players, the TRN page is only guaranteed to be valid if they're in a club.

<details>
  <summary>"Only guaranteed if they're in a club? That seems... arbitrary."</summary>

Switch names aren't unique, so usually, just putting a player's name into tracker.gg doesn't work. It also doesn't accept their platform id, for some reason. So, rlbuddy uses a special lookup using the player's club information to get their Epic Games ID, which it can then easily open in TRN.
</details>

#### Match history

<img src="readme-images/matchhistory.jpeg">

This widget lets you browse and view the past matches in this session, using the same interface as the current match widget.

#### Hotkeys

The main way to open rlbuddy is to press and hold a hotkey/controller button, which will keep it open for the duration of the press (unless you focus it). By default, the key is `Alt` and the button is `Select`, but it can be changed or disabled through in the settings. The available hotkeys are:

**Keyboard**

- Alt
- LShift (left shift)
- LCtrl (left control)
- Tab
- Super (Windows)

**Gamepad**

- Select
- Start
- Left bumper
- Right bumper

Dynamic selection coming soon!

#### Local music playback

<!--<img src="readme-images/music.jpeg">-->

rlbuddy allows a few playback controls and has an option to pause your music while player anthems are playing (during goal replays and the post-game scene). It asks Windows for this information, so anything you would see playing in the Action center will show up here, too.

### Integrations

#### Discord

<img src="readme-images/discord.jpeg">

rlbuddy shows your current status in Discord. You can disable this through the widget, or just hide the current score.

### Other

#### Automatic Stats API setup

<img src="readme-images/statsapisetup.jpeg">

If you don't have the Stats API enabled yet, this widget will automatically enable it for you based on the path to your Rocket League executable.

#### App transparency

The app can be partly transparent so you can see the game behind it even while it's up. The amount of transparency can be configured in the settings menu.
