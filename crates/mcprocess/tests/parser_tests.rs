use mcprocess::logs::{McLogParser, ServerEvent};

#[test]
fn test_detect_server_ready() {
    let line = "[14:20:00] [Server thread/INFO]: Done (5.020s)! For help, type \"help\"";
    let entry = McLogParser::parse(line).expect("Should parse");

    if let ServerEvent::Ready(time) = entry.event {
        assert_eq!(time, "5.020s");
    } else {
        panic!("Failed to detect Ready event");
    }
}

#[test]
fn test_detect_saving_sequence() {
    let line_start = "[14:20:00] [Server thread/INFO]: Saving the game...";
    let line_end = "[14:20:01] [Server thread/INFO]: Saved the game";

    assert_eq!(McLogParser::parse(line_start).unwrap().event, ServerEvent::Saving);
    assert_eq!(McLogParser::parse(line_end).unwrap().event, ServerEvent::Saved);
}

#[test]
fn test_detect_chat_async() {
    // El chat moderno suele venir de hilos async
    let line = "[14:20:00] [Async Chat Thread - #1/INFO]: <Steve> Hola mundo";
    let entry = McLogParser::parse(line).expect("Should parse");

    match entry.event {
        ServerEvent::Chat { author, msg } => {
            assert_eq!(author, "Steve");
            assert_eq!(msg, "Hola mundo");
        }
        _ => panic!("Failed to detect Chat"),
    }
}

#[test]
fn test_security_chat_spoofing() {
    // PRUEBA CRÍTICA: Un jugador intenta fingir que el servidor se apaga
    // El formato de log es correcto, pero viene de un jugador (<Troll>)
    let line = "[14:20:00] [Async Chat Thread - #1/INFO]: <Troll> Stopping the server";

    let entry = McLogParser::parse(line).unwrap();

    // NO debe ser ServerEvent::Stopping, debe ser ServerEvent::Chat
    match entry.event {
        ServerEvent::Chat { author, msg } => {
            assert_eq!(author, "Troll");
            assert_eq!(msg, "Stopping the server");
        }
        ServerEvent::Stopping => panic!("🚨 FALLO DE SEGURIDAD: Chat detectado como comando de sistema"),
        _ => panic!("Evento incorrecto detectado"),
    }
}

#[test]
fn test_ignore_random_threads() {
    // Un hilo cualquiera diciendo cosas de sistema no debería contar
    let line = "[14:20:00] [Worker-Main-4/INFO]: Stopping the server (simulation)";
    let entry = McLogParser::parse(line).unwrap();

    // Como no es 'Server thread', debería ser Unknown
    assert_eq!(entry.event, ServerEvent::Unknown);
}