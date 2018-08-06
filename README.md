# multiow-rs
Player tracking for Oddworld Abe's Exoddus implemented in Rust.

![](https://i.imgur.com/B1OamEL.gif)

### Oddserver:
- Server, capable of tracking players' position and rescued Mudokons.
- Supports a few basic commands (use "help").
- Notifies server owner about joining and leaving players.

### Oddclient:
- Client.
- Let's player choose their name and the server to connect to.
- Capable of displaying the players who are currently in the same room as the person running the client.
- Can also display announcements from the server.