use rocket::post;

// Existing routes should be defined above.

// New TIME Coin-specific RPC routes
#[post("/rpc/gettimeblockinfo")]
pub fn get_time_block_info() -> ... {
    // Handler implementation
}

#[post("/rpc/gettimeblockrewards")]
pub fn get_time_block_rewards() -> ... {
    // Handler implementation
}

#[post("/rpc/getmasternodeinfo")]
pub fn get_master_node_info() -> ... {
    // Handler implementation
}

#[post("/rpc/listmasternodes")]
pub fn list_master_nodes() -> ... {
    // Handler implementation
}

#[post("/rpc/getmasternodecount")]
pub fn get_master_node_count() -> ... {
    // Handler implementation
}

#[post("/rpc/getconsensusstatus")]
pub fn get_consensus_status() -> ... {
    // Handler implementation
}

#[post("/rpc/gettreasury")]
pub fn get_treasury() -> ... {
    // Handler implementation
}

#[post("/rpc/listproposals")]
pub fn list_proposals() -> ... {
    // Handler implementation
}
