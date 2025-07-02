get\_player\_data
    Gets the full player data as printed by pretty debug

set\_health \[health\]
    Sets the player's health to [health]

set\_energy \[energy\]
    Sets the player's energy to [energy]

set\_pos \[x\] \[y\]
    Sets the player's position to (\[x\], \[y\])

redraw
    Redraws the screen manually

list\_enemies
    Lists out all enemies with their index, position, and type

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
