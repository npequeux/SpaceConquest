STELLAR DOMINION
A turn-based 4X space strategy game (Rust + macroquad).
https://github.com/npequeux/SpaceConquest


INSTALL
-------
Windows
    Run the .msi and follow the wizard. The game is installed under
    "C:\Program Files\Stellar Dominion" and a Start Menu shortcut is created.
    A portable build is included as "Stellar Dominion.exe" if you would rather
    just copy the file somewhere - no installer needed.

macOS
    Open the .dmg and drag Stellar Dominion into Applications. The .zip
    contains the same StellarDominion.app bundle if you prefer to copy it by hand.
    macOS may gate the first launch because the binary is not notarised:
    right-click the app and choose Open, or allow it in
    System Settings > Privacy & Security.

Linux
    Debian / Ubuntu:  sudo dpkg -i stellar-dominion_<version>_amd64.deb
                      sudo apt-get install -f        # if a dependency is missing
    Any other distro: unpack the .tar.gz and run ./stellar-dominion
    The binary needs an X11 session plus ALSA (libasound2) at runtime.


REQUIREMENTS
------------
Windows   10 / 11, OpenGL 3.3 capable GPU
macOS     10.15+, OpenGL 3.3 capable GPU
Linux     X11, libasound2 (ALSA), libGL, OpenGL 3.3 capable GPU

The game renders at your monitor's native resolution; on a high-DPI display it
scales the interface automatically.


CONTROLS
--------
Left-click             select a system or fleet
Left-click a system    move the selected fleet there
Right-click / Esc      deselect
Drag                   pan the map
Mouse wheel            zoom
Click the minimap      jump to a region
Enter                  end turn
Ctrl+S                 save

Saves are written to:
    Windows   %USERPROFILE%\.stellar_dominion_save.json
    macOS     ~/.stellar_dominion_save.json
    Linux     ~/.stellar_dominion_save.json


LICENSE
-------
MIT, see LICENSE.txt. The Inter typeface bundled in the binary is licensed
separately under the SIL Open Font License 1.1.
