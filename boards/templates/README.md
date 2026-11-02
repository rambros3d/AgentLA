# Board Package Template

To add a new board:
1. Copy this folder into `boards/<your_board_id>/`
2. Edit `board.json` with your pin mappings, hardware capabilities, and voltage ratings
3. Provide `logo.svg`, `favicon.svg`, and `pinout.svg`
4. In `pinout.svg`, assign SVG element IDs matching `pin-gp<N>` (e.g. `<g id="pin-gp0" class="board-pin">`) so the WebUI can bind interactive hover, click, and probe animations
