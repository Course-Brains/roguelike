get\_player\_data
    Gets the full player data as printed by pretty debug

set\_health \[health\]
    Sets the player's health to [health], or full health if \[health\]
    is not given

set\_energy \[energy\]
    Sets the player's energy to [energy], or full energy if \[energy\]
    is not given

set\_pos \[x\] \[y\]
    Sets the player's position to (\[x\], \[y\])

redraw
    Redraws the screen manually

list\_enemies
    Lists out all enemies with their index, position, and type

kill \[index\]
    Kills the enemy at the given \[index\], if you want to find out the
    index, see list\_enemies

spawn \[variant\] \[x\] \[y\]
    Spawns an enemy at the given \[x\] and \[y\] coordinates, or the
    selector if not given.
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
        contact
            drain\_health
