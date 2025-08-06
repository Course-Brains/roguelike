RELATIVE POSITIONS:
The relative position format is used when specifying positions. The
format applies to each number individually instead of the position as
a whole. The format is as follows:
    #: absolute position
    p#: relative to the player
    s#: relative to the selector
Because it does this logic per number, the following are valid
positions in this format:
    5 10
    p5 p10
    p-5 p-10
    s5 s10
    s-5 s-10
    5 p10
    5 p-10
    p5 10
    ... you get the idea, the relative ones can be negative, the
    absolutes cannot.
Finally, if no position is specified(when able, this is only possible
if the position is the last argument to the command), it will default
to the position of the selector. However, this is done for both axis,
meaning that you cannot just skip one and have it be relative to the
selector for that axis alone. If you need to, simply use "s0" for that
axis.

==============================================================================

get\_player\_data
    Gets the full player data as printed by pretty debug

set\_health \[health\]
    Sets the player's health to [health], or full health if \[health\]
    is not given

set\_energy \[energy\]
    Sets the player's energy to [energy], or full energy if \[energy\]
    is not given

set\_pos \[x\] \[y\]
    Sets the player's position to (\[x\], \[y\]) this uses relative
    positions

redraw
    Redraws the screen manually

list\_enemies
    Lists out all enemies with their index, position, and type

kill \[index\]
    Kills the enemy at the given \[index\], if you want to find out the
    index, see list\_enemies

spawn \[variant\] \[x\] \[y\]
    Spawns an enemy at the given \[x\] and \[y\] coordinates, using
    relative positions.
    Valid variants are:
        basic
        basic\_boss
        mage
        mage\_boss

get\_enemy\_data \[index\]
    Gets the date of the enemy at the given \[index\], if you want to
    find out the index, see list\_enemies

force\_flood
    Forcefully refloods the map

wake\_all
    Wakes up all enemies and aggros them onto the player

open\_all\_doors
    Opens every door on the map

kill\_all\_enemies
    Kills every enemy on the map

set\_piece \[x\] \[y\] [args]
    sets the piece at (\[x\], \[y\]) to the piece specified by the args as
    follows:
        wall
            Just creates the wall
        door \[state\]
            Creates a door which will be \[open\] or \[closed\] according
            to \[state\]
        exit \[destination\]
            Creates an exit which goes to either the shop(\[shop\]) or
            next level(\[level\]) according to \[destination\]
        item \[item\]
            Sets it to an item piece containing the item specified by
            \[item\], [item] has the same valid values as the give
            command
        upgrade \[upgrade\]
            Sets it to an Upgrade which grants the given \[upgrade\].
            Valid upgrades are as in the upgrade command
    No, you cannot create spells using this because they require a
    caster

load\_next
    Loads the next level

load\_shop
    Loads the next shop

effect \[effect\] \[duration\]
    Sets the \[duration\] for the given \[effect\] for the player
    \[duration\]. Valid effects are:
        invincible
        mage\_sight
        regen
        unlucky
        doomed
    Valid durations are:
        none
            Disables the effect
        turns \[turns\]
            Gives the effect for a number of turns as specified by
            \[turns\]
        infinite
            Gives the effect with no time limit

give \[item\] \[slot\]
    Sets the given \[slot\] to contain the \[item\]. The slot is
    specified starting at 0 instead of 1.
    valid items are:
        mage\_sight
        health\_potion
        boss\_finder
        gamba
        ender\_pearl
        warp

set\_money \[amount\]
    Sets the money to the given \[amount\]

upgrade \[upgrade\]
    Gives the specified \[upgrade\]
    Valid upgrades are:
        mage\_eye
        map
        soft\_shoes

set\_detect\_mod \[mod\]
    Sets the detection modifier to the given \[mod\]

set\_perception \[perception\]
    Sets the player's perception to the given \[perception\]

cast \[spell type\] \[spell\]
    Casts the given \[spell\] at the selector, the spells(and the type
    they are in are as follows):
        normal
            swap
            biden\_blast
            identify
        contact
            drain\_health

create\_circle \[spell type\] \[spell\] \[pos x\] \[pos y\] \[target x\] \[targety\]
    Creates a spell circle at the given (relative) position. Uses the
    same spell selection rules as cast. The target position is not
    always needed by spells, so it can be ignored for those spells.

get\_data \[x\] \[y\]
    Gets all data at the given position(except backtrace data >:}), it
    does use relative positioning

get\_boss
   Gives the boss's position, type, and index if it exists

count\_enemies
    Counts the number of each type of enemy

checksum
    Checks and reports any instances of enemies being on top of a
    piece, notably, this does not ignore cases like open doors where
    that is intentional
