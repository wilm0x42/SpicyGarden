#!/bin/bash

APIFILE=$(ls spigot-api-*.jar | tail -n1)

echo "Running javac..."
javac -d . -classpath $APIFILE SpicyGarden.java
echo "Running jar..."
jar cvf SpicyGarden.jar plugin.yml spicyrice/SpicyGarden/SpicyGarden.class
