# mljboard-bot

The main part of the `mljboard` project. The `mljboard-bot` Discord bot allows Discord users to link their public Maloja websites *or* their local Maloja servers to their Discord accounts, and share only the statistics they want to share with other music enjoyers.

WIP, unfinished.

## Optional shuttle integration

**Compile with `--no-default-features` to disable shuttle integration.**

Create a `Secrets.toml` following the example in `Secrets.toml.example`. Keep in mind everything has to be strings at the moment.

## Requirements

- A Discord bot token. `-d <DISCORD BOT TOKEN>`
- A [HOS server](https://github.com/duckfromdiscord/hos-rv) and its password. `-j <IP> -k <PORT> -s <PASSWD>` and supply `--hos-https` if it's secure (recommended).
- A PostgreSQL database in your `DATABASE_URL` env variable. `postgres://user:pass@127.0.0.1:5432/mljboard`. `mljboard-bot` should create any missing tables on its own. Not needed when using with Shuttle of course.
- A Last.FM API key. `-l <API>`