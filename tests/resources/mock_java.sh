#!/bin/sh
# Ignoramos los argumentos (-Xms, -jar, etc)

echo "[INFO] Iniciando Mock Server..."
# Simulamos tiempo de carga
sleep 1
# La palabra clave que busca tu código
echo "[12:00:00] [Server thread/INFO]: Done (1.0s)! For help, type \"help\""

# Bucle de lectura (stdin)
while read line; do
    echo "Mock recibió: $line"
    if [ "$line" = "stop" ]; then
        echo "Stopping server..."
        sleep 0.5
        exit 0
    fi
done