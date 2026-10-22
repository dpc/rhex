# Core game design

Rhex aims to be an atmospheric, scary hex-grid roguelike in which hiding,
lurking, scouting, and positioning matter more than brute stats. Players should
weigh facing, limited information, light, sound, and the risk of exposure when
deciding how to act.

This document describes design direction, not a list of finished features or a
roadmap. The [README](README.md) covers playing the current game;
[ARCH-rhex](specs/ARCH-rhex.md) describes its implementation.

## Space and facing

Characters have both a position and a facing. Players decide where to stand and
where to look through distinct movement and turning actions. They should
consider what they might learn or expose by moving or turning.

Favor organic, natural-looking spaces that make use of the hex grid: irregular
boundaries, winding routes, and uneven openings rather than a default layout of
square rooms and corridors. This preference does not rule out buildings on the
hex grid. Players should weigh position, lines of sight, and routes around
obstructions when navigating these natural shapes.

## Information, exposure, and fear

Players should feel tension from limited information and use scouting to judge
danger without making every encounter predictable. As they turn to inspect a
direction, approach an opening, or choose a route, they should consider what
remains unseen and how much they expose themselves.

Players should weigh light and noise in those choices. In light, they can learn
more but risk exposure; through sound, they can detect something unseen but draw
attention with a noisy action. Terrain should affect these risks as well as
determine where players can walk. In natural spaces, players should use facing,
light, and sound to observe, lurk, and hide rather than treat every threat as a
stat check.

Players should feel fear as they act with incomplete knowledge and weigh the
risk of discovery. Stats, equipment, and combat have a place, but should not
dominate play or make scouting and careful positioning incidental.

## Implemented foundation and design goals

The game has a hex map, facing-based movement and perception, lighting,
terrain-dependent sound propagation, and enemies that can notice the player
through sight or sound. Dungeon and Dark Forest levels provide different spaces;
the forest includes irregular paths, obstructing vegetation, and lit openings.
Players can experience these foundations, but the full horror/stealth experience
remains a design goal.

There is no general hiding mechanic or enemy forgetting policy yet. Breaking
sight or entering darkness does not guarantee that an enemy loses awareness of
the player. Hiding, lurking, and scouting are priorities for the intended
experience, not promises of additional implemented actions or systems.
