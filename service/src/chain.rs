use alloy::primitives::Address;

// TODO: clap args
struct Args {}

// TODO:
// - will be read from TOML eventually
// - addresses
// - rpc urls
// - per chains
struct Chain<P> {
    id: u64,
    name: String,
    // TODO: should this include the provider here?
    provider: P, // provider should be able to submit protocol txs
    origin: Address,
    destination: Address,
    x_account: Address,
}
