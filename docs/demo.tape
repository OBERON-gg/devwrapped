# build first with `cargo build --release`, then run `vhs docs/demo.tape` from the repository root

Output docs/demo.gif

Set Shell "bash"
Set FontSize 15
Set Width 620
Set Height 1070
Set Padding 28
Set TypingSpeed 70ms
Set Theme { "name": "devwrapped", "background": "#0c0a1d", "foreground": "#e8e6f5", "cursor": "#c8ff3d", "selection": "#2a2550", "black": "#0c0a1d", "brightBlack": "#6f6a99", "red": "#ff5c8a", "brightRed": "#ff7aa0", "green": "#c8ff3d", "brightGreen": "#d8ff70", "yellow": "#ffd84a", "brightYellow": "#ffe27a", "blue": "#6a8bff", "brightBlue": "#8ca5ff", "magenta": "#ff3d9a", "brightMagenta": "#ff6db3", "cyan": "#22d3ee", "brightCyan": "#5ee0f2", "white": "#e8e6f5", "brightWhite": "#ffffff" }

Hide
Type "export PATH=$PWD/target/release:$PATH PS1='$ ' LC_TIME=en_US.UTF-8 && clear"
Enter
Show

Sleep 600ms
Type "devwrapped --sample"
Sleep 400ms
Enter
Sleep 6s
