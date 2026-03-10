use tronic::signer::LocalSigner;

fn main() {
    let signer = LocalSigner::rand();
    let key = signer.secret_key();
    println!("export TRON_TEST_KEY={}", hex::encode(key));
    println!("# Address: {}", signer.address());
    println!("# Fund via: https://nileex.io/join/getJoinPage");
}
