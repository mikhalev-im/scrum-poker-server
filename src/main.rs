use std::{env, net::SocketAddr, sync::{Arc}};
use std::collections::{HashMap, HashSet};

use futures_util::{SinkExt, StreamExt, TryStreamExt, future, stream::{SplitSink, SplitStream}};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex},
};
use tokio_tungstenite::tungstenite::{Message};

mod users;
mod rooms;

/*
Logic:

* User connects to the server, he is added to the state.users.
* User should set his name (user.name)
  * { type: 'user.set_name', name }

* User creates a room
  * { type: 'room.create' }
  * leave current user room
  * generate random id
  * check that the id doesn't exist
  * create a room (state.rooms)
  * add user to the room and make him an admin

* User joins a room
  * { type: 'room.join', room }
  * check that the room exists (create a new one)
  * add user to the room

* User leaves a room
  * { type: 'room.leave' }
  * remove the room if no users left (state.rooms)

* User disconnects
  * leave the room
  * remove the user (state.users)


Poker:
* set player score
  * { type: 'user.set_score', score }
* show results
  * { type: 'room.show_results' }
* reset the vote
  * { type: 'room.reset' }


*/

struct User {
    id: SocketAddr,
    name: Option<String>,
    room: Option<String>,
    score: Option<u8>,
    sink: SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>,
}

impl User {
  fn new(address: SocketAddr, sink: SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>) -> Self {
    User {
      id: address,
      name: None,
      room: None,
      score: None,
      sink,
    }
  }
}

struct Room {
    id: String,
    password: Option<String>,
    users: HashSet<SocketAddr>,
    admins: HashSet<SocketAddr>
}

struct State {
    users: HashMap<SocketAddr, User>,
    rooms: HashMap<String, Room>
}

type AppState = Arc<Mutex<State>>;

async fn handle_connection(stream: TcpStream, remote_addr: SocketAddr, state: AppState) {
    println!("Incoming TCP connection from: {}", remote_addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    println!("WebSocket connection established: {}", remote_addr);

    // Split the stream into incoming stream and outcoming sink
    let (mut sink, stream) = ws_stream.split();

    // Create a new user instance and add it to the state
    let ls = state.clone();
    let mut locked_state = ls.lock().await;
    locked_state.users.insert(remote_addr, User::new(remote_addr, sink));

    // Handle incoming messages of this user
    stream.try_for_each_concurrent(None, |msg| {
      let t = state.clone();

      async move {
        // Turn message into a string
        let message = msg.to_text().unwrap();
        println!("Received a message from {}: {}", remote_addr, message);

        // Parse json, fallback to empty object
        let data: serde_json::Value = serde_json::from_str(message).unwrap_or(serde_json::json!({}));

        // Extract action type
        match data["type"].as_str() {
          Some("user.set_name") => users::set_name(&remote_addr, data["name"].as_str(), t.clone()).await,
          Some("room.create") => rooms::create().await,
          Some("room.join") => rooms::join().await,
          Some("room.leave") => rooms::leave().await,
          Some("user.set_score") => users::set_score().await,
          Some("room.show_results") => rooms::show_results().await,
          Some("room.reset") => rooms::reset().await,
          _ => println!("Unknown action"),
        };

        Ok(())
      }

    }).await.unwrap();
}

#[tokio::main]
async fn main() {
    // Parse server address from arguments or fallback to defaults
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Create shared state
    let state: AppState = Arc::new(Mutex::new(State {
      users: HashMap::new(),
      rooms: HashMap::new(),
    }));

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, remote_addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, remote_addr, state.clone()));
    }
}
