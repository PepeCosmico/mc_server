#!/bin/bash
# mock_java.sh

# 1. Simular arranque
# Importante: El formato de fecha y log debe coincidir con tu Regex ^\[(\d{2}:\d{2}:\d{2})\] \[([^\]]+)\]: (.*)$
echo "[10:00:00] [main/INFO]: Loading Minecraft 1.21.4 with Fabric Loader 0.17.3"
echo "[10:00:00] [Server thread/INFO]: Loading libraries..."
sleep 0.1
echo "[10:00:05] [Server thread/INFO]: Done (1.0s)! For help, type \"help\""

# 2. Bucle de lectura
# IMPORTANTE: Usamos 'tr' para borrar \r por si ejecutas tests desde Windows
while read -r line; do
    CMD=$(echo "$line" | tr -d '\r')
    echo "MOCK_RECEIVED: $CMD"

    case "$CMD" in
        "stop")
            echo "[10:01:00] [Server thread/INFO]: Stopping the server"
            echo "[10:01:01] [Server thread/INFO]: Saving chunks..."
            sleep 0.5
            exit 0
            ;;
        "save-all")
            # Esto es lo que espera tu función backup()
            echo "[10:02:00] [Server thread/INFO]: Saving the game..."
            sleep 0.2
            echo "[10:02:01] [Server thread/INFO]: Saved the game"
            ;;
        "crash")
            echo "[10:03:00] [Server thread/ERROR]: CRITICAL FAILURE"
            exit 1
            ;;
        *)
            # Ignorar otros comandos
            ;;
    esac
done
