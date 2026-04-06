pub type Worker(m) {
  Pid
}

pub type WorkerMessage(m) {
  Down
  Message(m)
}

pub type Worker(m) {
  Pid
}

pub type WorkerMessage(m) {
  Down
  Message(m)
}

pub fn spawn(init: fn(fn() -> WorkerMessage(m)) -> Nil) -> Result(Worker(m), Nil) {
  Ok(Pid)
}

pub fn send(pid: Worker(m), message: m) -> Result(Nil, Nil) {
  Ok(Nil)
}

fn example() {
  let Ok(pid) = spawn(fn(receive) {
    let Message(5) = receive()
    // Won't compile because receive is a fn that returns integer
    // let Message("String") = receive()
    Nil
  })
  send(pid, 900)
  // Won't compile because pid is paramaterised by i
  send(pid, "String")
  Nil
}
