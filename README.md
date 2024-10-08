# NDS Plus

This is a DS emulator written in Rust! Binaries for Mac and Windows are now available. Go to releases and download the appropriate zip file for your operating system and unzip the files. You will need to have copies of the bios7, bios9, and firmware binaries in the root directory of the executable. 

Once that's complete, open the executable as usual. Alternatively, run the executable in the command line with the path to a ROM as the first argument. Linux users will have to compile their own binary from the desktop directory either using `cargo build --release` or `cargo run --release <path to rom>`. Make sure to have the bios and firmware binaries in the desktop directory as usual.

## Web Client

To test the latest version of the emulator on web, go to https://nds-emulator.onrender.com. You will need copies of the ARM7 and ARM9 BIOSes as well as the DS firmware.

## Features

- Support for both web and desktop
- Ability to use control stick in Super Mario 64 DS
- Save management on the web and iOS clients: upload, download and delete saves
- Cloud saves are now available! Store saves in Google drive for use anywhere on both web, desktop and iOS.
- Support for microphone on iOS, web, and desktop

## TODO

- Texture/rendering issues
- CPU bugs
- iOS app (almost complete!)
- Save states
- Debugging tools

## Controls

Keyboard:

- *Up*: W Key
- *Down*: S Key
- *Left*: A Key
- *Right*: D Key
- *A Button*: K Key
- *B Button*: J Key
- *Y Button*: N Key
- *X Button*: M Key
- *L Button*: C Key
- *R Button*: V Key
- *Select*: Tab
- *Start*: Return

Joypad (tested on PS5 controller, should be similar on Xbox/other similar controllers)

- *Directions*: Control pad
- *A Button*: Circle
- *B Button*: Cross
- *Y Button*: Square
- *X Button*: Triangle
- *L Button*: L1
- *R BUtton*: R1
- *Select*: Select
- *Start*: Start

## Screenshots

<img width="250" alt="Screenshot 2024-08-22 at 7 20 09 PM" src="https://github.com/user-attachments/assets/aee2e327-b552-4648-99fd-98be39994914">
<img width="250" alt="Screenshot 2024-08-22 at 7 20 54 PM" src="https://github.com/user-attachments/assets/8c2875df-d052-4d08-b1de-dd4126a1412e">
<img width="250" alt="Screenshot 2024-08-22 at 7 23 10 PM" src="https://github.com/user-attachments/assets/a5d50262-2383-4c5f-97a3-b46531fcfd9a">
<img width="250" alt="Screenshot 2024-08-22 at 7 24 06 PM" src="https://github.com/user-attachments/assets/db0f3eb3-02fd-46d3-b491-f22c575ab077">
<img width="250" alt="Screenshot 2024-08-22 at 7 43 05 PM" src="https://github.com/user-attachments/assets/1d41de7b-1089-4daa-943e-e5d79b6f9c6e">
<img width="250" alt="Screenshot 2024-08-22 at 7 39 35 PM" src="https://github.com/user-attachments/assets/43fb5b61-2037-4915-9cc6-5dfeacb3a62d">



