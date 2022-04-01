# mcstat
### This tool pings a minecraft server and displays information about it. 
## This includes
- protocol version
- server version
- player count
- player sample
- description
- favicon shown as ascii art
- mod list (for forge servers)
- forge channels
- raw json response output

## Extra Features
- DNS SRV lookup for servers that are hosted on a different address
- Colored MOTD
- Rust performance!

## TODO
---
- [x] fork async-minecraft-ping to fix some bugs and implement mod list response (dev is not responding to [issue](https://github.com/jsvana/async-minecraft-ping/issues/3))
