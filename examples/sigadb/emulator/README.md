# Steps to run the emulator

To install the emulator
* `./emulator.sh -i`

To run the emulator
* `./emulator.sh -r`

# Installing the signal app to the emulator

Make sure the emulator is running using `-r` command above, you may need to restart the adb server `adb kill-server && adb start-server`.

Run:
* `adb install ~/sdk/signal.apk`
> This command might take a while to install the signal app, 2-5 minutes.
> If it didn't work, stop and start adb as shown in the command below, enable the developers option inside the emulator and then try again.
>
`adb kill-server && adb start-server`

# Sending clicks using ADB
Make sure to open the signal app and then choose (click on) a chat. After that, you can run this script that will automate typing and sending messages to that chat. This script sends text messages and images with a short delay between the messages.
* `./clicks.sh`
