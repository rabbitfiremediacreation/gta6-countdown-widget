#!/bin/zsh
cd "$(dirname "$0")"; (sleep 1; open http://localhost:8666) & python3 -m http.server 8666
