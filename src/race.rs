use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaceEvent {
    Hosted {
        code: String,
    },
    Start {
        starts_at_millis: u64,
        duration_seconds: Option<u64>,
        prompt: String,
    },
    PeerProgress {
        characters: usize,
        accuracy: u16,
    },
    PeerFinished {
        characters: usize,
        wpm: u16,
        accuracy: u16,
    },
    PeerDisconnected,
    Error(String),
}

pub struct RaceClient {
    commands: mpsc::Sender<String>,
    events: mpsc::Receiver<RaceEvent>,
}

impl RaceClient {
    pub fn host(
        relay_address: &str,
        duration_seconds: Option<u64>,
        prompt: String,
    ) -> io::Result<Self> {
        let code = invite_code();
        let client = Self::connect(relay_address)?;
        client.send(format!(
            "CREATE {code} {} {}",
            duration_seconds.unwrap_or_default(),
            encode(&prompt)
        ));
        Ok(client)
    }

    pub fn join(relay_address: &str, code: &str) -> io::Result<Self> {
        let client = Self::connect(relay_address)?;
        client.send(format!("JOIN {}", code.trim().to_ascii_uppercase()));
        Ok(client)
    }

    fn connect(relay_address: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(relay_address)?;
        stream.set_nodelay(true)?;
        let reader = stream.try_clone()?;
        let (commands, command_receiver) = mpsc::channel::<String>();
        let (event_sender, events) = mpsc::channel::<RaceEvent>();
        thread::spawn(move || writer_loop(stream, command_receiver));
        thread::spawn(move || reader_loop(reader, event_sender));
        Ok(Self { commands, events })
    }

    pub fn try_event(&self) -> Option<RaceEvent> {
        self.events.try_recv().ok()
    }

    pub fn progress(&self, characters: usize, accuracy: u16) {
        self.send(format!("PROGRESS {characters} {accuracy}"));
    }

    pub fn finish(&self, characters: usize, wpm: u16, accuracy: u16) {
        self.send(format!("FINISH {characters} {wpm} {accuracy}"));
    }

    fn send(&self, command: String) {
        let _ = self.commands.send(command);
    }
}

pub fn run_relay(listener: TcpListener) -> io::Result<()> {
    let rooms = Arc::new(Mutex::new(HashMap::<String, Room>::new()));
    for connection in listener.incoming() {
        let stream = connection?;
        let rooms = Arc::clone(&rooms);
        thread::spawn(move || handle_connection(stream, rooms));
    }
    Ok(())
}

struct Peer {
    writer: Arc<Mutex<TcpStream>>,
}

struct Room {
    duration_seconds: Option<u64>,
    prompt: String,
    host: Option<Peer>,
    guest: Option<Peer>,
}

fn handle_connection(stream: TcpStream, rooms: Arc<Mutex<HashMap<String, Room>>>) {
    let writer = match stream.try_clone() {
        Ok(stream) => Arc::new(Mutex::new(stream)),
        Err(_) => return,
    };
    let mut room_code = None;
    let mut is_host = false;
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let parts = line.splitn(4, ' ').collect::<Vec<_>>();
        match parts.first().copied() {
            Some("CREATE") if parts.len() == 4 && room_code.is_none() => {
                let duration_seconds = parts[2].parse::<u64>().ok().filter(|seconds| *seconds > 0);
                let Some(prompt) = decode(parts[3]) else {
                    write_line(&writer, "ERROR invalid-prompt");
                    continue;
                };
                let code = parts[1].to_owned();
                let mut rooms = rooms.lock().expect("race room lock poisoned");
                if rooms.contains_key(&code) {
                    write_line(&writer, "ERROR code-unavailable");
                    continue;
                }
                rooms.insert(
                    code.clone(),
                    Room {
                        duration_seconds,
                        prompt,
                        host: Some(Peer {
                            writer: Arc::clone(&writer),
                        }),
                        guest: None,
                    },
                );
                room_code = Some(code.clone());
                is_host = true;
                write_line(&writer, &format!("HOSTED {code}"));
            }
            Some("JOIN") if parts.len() == 2 && room_code.is_none() => {
                let code = parts[1].to_ascii_uppercase();
                let mut rooms = rooms.lock().expect("race room lock poisoned");
                let Some(room) = rooms.get_mut(&code) else {
                    write_line(&writer, "ERROR code-not-found");
                    continue;
                };
                if room.guest.is_some() {
                    write_line(&writer, "ERROR race-full");
                    continue;
                }
                room.guest = Some(Peer {
                    writer: Arc::clone(&writer),
                });
                room_code = Some(code);
                let start_at = now_millis() + 3_000;
                let start = format!(
                    "START {start_at} {} {}",
                    room.duration_seconds.unwrap_or_default(),
                    encode(&room.prompt)
                );
                write_line(&room.host.as_ref().expect("host exists").writer, &start);
                write_line(&writer, &start);
            }
            Some("PROGRESS") if parts.len() == 3 => {
                forward_to_opponent(
                    &rooms,
                    room_code.as_deref(),
                    is_host,
                    &format!("PEER_PROGRESS {} {}", parts[1], parts[2]),
                );
            }
            Some("FINISH") if parts.len() == 4 => {
                forward_to_opponent(
                    &rooms,
                    room_code.as_deref(),
                    is_host,
                    &format!("PEER_FINISHED {} {} {}", parts[1], parts[2], parts[3]),
                );
            }
            _ => write_line(&writer, "ERROR invalid-message"),
        }
    }
    if let Some(code) = room_code {
        let mut rooms = rooms.lock().expect("race room lock poisoned");
        if is_host {
            if let Some(room) = rooms.remove(&code)
                && let Some(peer) = room.guest
            {
                write_line(&peer.writer, "PEER_DISCONNECTED");
            }
        } else if let Some(room) = rooms.get_mut(&code) {
            if let Some(peer) = &room.host {
                write_line(&peer.writer, "PEER_DISCONNECTED");
            }
            room.guest = None;
        }
    }
}

fn forward_to_opponent(
    rooms: &Arc<Mutex<HashMap<String, Room>>>,
    room_code: Option<&str>,
    is_host: bool,
    message: &str,
) {
    let Some(code) = room_code else { return };
    let rooms = rooms.lock().expect("race room lock poisoned");
    let Some(room) = rooms.get(code) else { return };
    let peer = if is_host { &room.guest } else { &room.host };
    if let Some(peer) = peer {
        write_line(&peer.writer, message);
    }
}

fn writer_loop(mut stream: TcpStream, commands: mpsc::Receiver<String>) {
    for command in commands {
        if writeln!(stream, "{command}").is_err() || stream.flush().is_err() {
            return;
        }
    }
}

fn reader_loop(stream: TcpStream, events: mpsc::Sender<RaceEvent>) {
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { break };
        if let Some(event) = parse_event(&line) {
            let _ = events.send(event);
        }
    }
    let _ = events.send(RaceEvent::PeerDisconnected);
}

fn parse_event(line: &str) -> Option<RaceEvent> {
    let parts = line.splitn(5, ' ').collect::<Vec<_>>();
    match parts.as_slice() {
        ["HOSTED", code] => Some(RaceEvent::Hosted {
            code: (*code).into(),
        }),
        ["START", start, duration, prompt] => Some(RaceEvent::Start {
            starts_at_millis: start.parse().ok()?,
            duration_seconds: duration.parse::<u64>().ok().filter(|seconds| *seconds > 0),
            prompt: decode(prompt)?,
        }),
        ["PEER_PROGRESS", characters, accuracy] => Some(RaceEvent::PeerProgress {
            characters: characters.parse().ok()?,
            accuracy: accuracy.parse().ok()?,
        }),
        ["PEER_FINISHED", characters, wpm, accuracy] => Some(RaceEvent::PeerFinished {
            characters: characters.parse().ok()?,
            wpm: wpm.parse().ok()?,
            accuracy: accuracy.parse().ok()?,
        }),
        ["PEER_DISCONNECTED"] => Some(RaceEvent::PeerDisconnected),
        ["ERROR", message] => Some(RaceEvent::Error((*message).into())),
        _ => None,
    }
}

fn write_line(writer: &Arc<Mutex<TcpStream>>, line: &str) {
    if let Ok(mut writer) = writer.lock() {
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}

fn invite_code() -> String {
    (0..6)
        .map(|_| CODE_ALPHABET[rand::random_range(0..CODE_ALPHABET.len())] as char)
        .collect()
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode(value: &str) -> Option<String> {
    if value.len() % 2 != 0 {
        return None;
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_unicode_prompts_without_losing_text() {
        let prompt = "type naïve words!";
        assert_eq!(decode(&encode(prompt)), Some(prompt.into()));
    }

    #[test]
    fn relay_pairs_two_clients_and_forwards_progress() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        thread::spawn(move || run_relay(listener).unwrap());
        let host = RaceClient::host(&address, Some(30), "shared prompt".into()).unwrap();
        let code = wait_for_event(&host, |event| matches!(event, RaceEvent::Hosted { .. }));
        let RaceEvent::Hosted { code } = code else {
            unreachable!()
        };
        let guest = RaceClient::join(&address, &code).unwrap();
        assert!(
            matches!(wait_for_event(&host, |event| matches!(event, RaceEvent::Start { .. })), RaceEvent::Start { prompt, .. } if prompt == "shared prompt")
        );
        assert!(matches!(
            wait_for_event(&guest, |event| matches!(event, RaceEvent::Start { .. })),
            RaceEvent::Start { .. }
        ));
        host.progress(18, 97);
        assert_eq!(
            wait_for_event(&guest, |event| matches!(
                event,
                RaceEvent::PeerProgress { .. }
            )),
            RaceEvent::PeerProgress {
                characters: 18,
                accuracy: 97
            }
        );
    }

    fn wait_for_event(client: &RaceClient, predicate: impl Fn(&RaceEvent) -> bool) -> RaceEvent {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(event) = client.try_event()
                && predicate(&event)
            {
                return event;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for event"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
