#!/bin/bash
#
# Filename: mqtt_sample
# Description: Test for publishing and interpreting messages in rust
FILE=$1


while IFS= read -r line
do 
	mosquitto_pub -h 127.0.0.1 -t home/command/0x01/0x03 -m "$line"
	sleep 1
	#mosquitto_pub -h 127.0.0.1 -t home/command/0x02/0x02 -m "$line"
  	#sleep 1
done < $FILE
