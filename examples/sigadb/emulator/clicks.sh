#!/bin/bash

send_image(){
	echo "Sending an image"
	adb exec-out input tap 195 379
	sleep 1
	adb exec-out input tap 161 495
	sleep 1
	adb exec-out input tap 291 597
	sleep 1
}

send_text() {
	echo "Sending text $1"
	adb exec-out input text "$1"
	sleep 1
	adb exec-out input tap 284 380
	sleep 1
}

click_on_keyboard(){
	adb exec-out input tap 126 611
}

while :
do
	click_on_keyboard
	send_text "hello"
	send_image

done
