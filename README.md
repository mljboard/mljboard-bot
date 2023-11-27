# mljboard-bot

The main part of the `mljboard` project. The `mljboard-bot` Discord bot allows Discord users to link their public Maloja websites *or* their local Maloja servers to their Discord accounts, and share only the statistics they want to share with other music enjoyers.

WIP, unfinished.

## Requirements

- A Discord bot token. `-d <DISCORD BOT TOKEN>`
- A [HOS server](https://github.com/duckfromdiscord/hos-rv) and its password. `-j <IP> -k <PORT> -s <PASSWD>`
- A MongoDB database. `-m mongodb://x:x@x/x`. `mljboard-bot` should create any missing collections on its own.
- A Last.FM API key. `-l <API>`