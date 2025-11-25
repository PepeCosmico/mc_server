#!/bin/bash
# Simular tiempo de carga
echo "[10:00:00] [Server thread/INFO]: Loading libraries, please wait..."
sleep 0.5
echo "[10:00:01] [Server thread/INFO]: Starting minecraft server version 1.20.1"
echo "[10:00:02] [Server thread/INFO]: Loading properties"
echo "[10:00:05] [Server thread/INFO]: Done (3.000s)! For help, type \"help\""

# Bucle infinito leyendo STDIN (Comandos)
while read -r line; do
    echo "MOCK_RECEIVED: $line"

    if [[ "$line" == "stop" ]]; then
        echo "[10:01:00] [Server thread/INFO]: Stopping the server"
        echo "[10:01:01] [Server thread/INFO]: Saving chunks for level 'ServerLevel'..."
        sleep 0.5
        exit 0
    elif [[ "$line" == "save-all" ]]; then
        echo "[10:02:00] [Server thread/INFO]: Saving the game..."
        sleep 0.2
        echo "[10:02:01] [Server thread/INFO]: Saved the game"
    elif [[ "$line" == "crash" ]]; then
        echo "[10:03:00] [Server thread/ERROR]: Encountered an unexpected exception"
        exit 1
    else
        # Eco de comando desconocido
        echo "[10:04:00] [Server thread/INFO]: Unknown command: $line"
    fi
done