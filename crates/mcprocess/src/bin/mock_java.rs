use std::io::{self, BufRead, Write};
use std::process;
use std::thread;
use std::time::Duration;

fn main() {
    // Helper para imprimir y limpiar el buffer inmediatamente (Crucial para tests)
    let print_flush = |msg: &str| {
        println!("{}", msg);
        io::stdout().flush().unwrap();
    };

    // 1. Simular arranque
    print_flush("[10:00:00] [Server thread/INFO]: Loading libraries...");
    thread::sleep(Duration::from_millis(100));
    print_flush("[10:00:05] [Server thread/INFO]: Done (1.0s)! For help, type \"help\"");

    // 2. Bucle de lectura de comandos
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(input) => {
                let cmd = input.trim();

                // Si quieres ver qué comandos llegan mientras debuggeas:
                // eprintln!("DEBUG MOCK: Recibido '{}'", cmd);

                match cmd {
                    "save-off" => {
                        // Respuesta estándar de MC cuando desactivas guardado
                        print_flush(
                            "[10:01:00] [Server thread/INFO]: Automatic saving is now disabled",
                        );
                    }
                    "save-all" => {
                        // 1. Avisa que empieza a guardar
                        print_flush("[10:01:05] [Server thread/INFO]: Saving the game (flush)...");
                        print_flush("[10:01:05] [Server thread/INFO]: Saving the game...");

                        // Simulamos que tarda un poco en escribir a disco
                        thread::sleep(Duration::from_millis(1000));

                        // 2. IMPORTANTE: Esta es la señal que busca tu 'saved_signal'
                        print_flush("[10:01:06] [Server thread/INFO]: Saved the game");
                    }
                    "save-on" => {
                        // Respuesta estándar al reactivar
                        print_flush(
                            "[10:01:10] [Server thread/INFO]: Automatic saving is now enabled",
                        );
                    }
                    "stop" => {
                        print_flush("[10:02:00] [Server thread/INFO]: Stopping the server");
                        print_flush("[10:02:01] [Server thread/INFO]: Saving chunks...");
                        thread::sleep(Duration::from_millis(200));
                        process::exit(0);
                    }
                    "crash" => {
                        print_flush("[10:03:00] [Server thread/ERROR]: CRITICAL FAILURE");
                        process::exit(1);
                    }
                    _ => {
                        // Comandos desconocidos se ignoran o se loguean como 'Unknown command'
                        // print_flush("[Server thread/INFO]: Unknown command");
                    }
                }
            }
            Err(_) => break, // EOF
        }
    }
}
