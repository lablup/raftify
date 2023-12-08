#[macro_use]
extern crate slog;

use raftify::create_client;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(long)]
    raft_addr: String,
}

#[actix_rt::main]
async fn main() {
    let options = Options::from_args();
    let leader_addr = options.raft_addr;
    let mut leader_client = create_client(leader_addr).await.unwrap();
    leader_client;
}
