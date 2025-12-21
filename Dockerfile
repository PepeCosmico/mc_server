# --- ETAPA 1: BUILDER (Compilación de Rust) ---
FROM rust:1.92-slim-bookworm as builder

# Creamos directorio de trabajo
WORKDIR /usr/src/app

# Copiamos todo el código fuente del workspace
COPY . .

# Compilamos en modo release.
# Especificamos el binario mcdaemon explícitamente.
RUN cargo build --release -p mcdaemon

# --- ETAPA 2: RUNTIME (Entorno de Ejecución con Java) ---
# Usamos Eclipse Temurin (Java oficial y ligero).
# IMPORTANTE: Cambia "21" por la versión de Java que necesite tu Minecraft (17, 8, etc.)
FROM eclipse-temurin:21-jre-jammy

# Directorio de trabajo en el contenedor final
WORKDIR /app

# Creamos las carpetas necesarias para que Rust no falle al arrancar
RUN mkdir -p server backups

# Copiamos el binario compilado desde la etapa 1
COPY --from=builder /usr/src/app/target/release/mc_daemon /app/mc_daemon

# Copiamos el archivo de configuración (asegúrate de tener uno de producción)
COPY configs/prod.toml /app/prod.toml

# Exponemos el puerto del Daemon (TCP) y el de Minecraft (25565)
EXPOSE 8080
EXPOSE 25565

# Ejecutamos el Daemon
CMD ["./mc_daemon"]
