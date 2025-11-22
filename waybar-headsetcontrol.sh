#!/bin/bash

# waybar-headsetcontrol by Henriklmao
log=waybar-headsetcontrol.log

# Template for log output
# echo "$(date +%F_%H:%M:%S): " >> $log

echo "$(date +%F_%H:%M:%S): Script started" > $log
echo "- Getting battry percentage:" >> $log

# Get battery state  -- Curently only one device supported
battery=$(headsetcontrol -b 2>/dev/null | awk -F'Level:' '/Level/ {gsub(/[^0-9]/,"",$2); print $2; exit}')

# If no value
if [[ -z "$battery" ]]; then
    echo "Battery percentage not available" >> $log
    echo "$(date +%F_%H:%M:%S): Battery percentage not available" >> $log
    exit 0
fi

echo "Battery percentage available" >> $log

# output value
if [ "$battery" -ge 100 ]; then
    echo "$(date +%F_%H:%M:%S): Detected status charging or 100%" >> $log
else
    echo "$(date +%F_%H:%M:%S): Battery percentage is ${battery}%" >> $log
    echo "${battery}%"
fi

