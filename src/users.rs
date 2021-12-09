use std::net::{SocketAddr};
use crate::{AppState};

pub async fn set_name(user_id: &SocketAddr, name: Option<&str>, state: AppState) {
  // let s = state.lock().await;


  // let msg = Message::text("{}");
  // let u = s.users.get(&remote_addr).unwrap();
  // u.sink.send(msg);
  println!("user.set_name");
}

pub async fn set_score() {
  println!("user.set_score");
}