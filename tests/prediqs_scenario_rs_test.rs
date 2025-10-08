use multiversx_sc_scenario::imports::*;

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();

    // blockchain.set_current_dir_from_workspace("relative path to your workspace, if applicable");
    blockchain.register_contract("mxsc:output/prediqs.mxsc.json", prediqs::ContractBuilder);
    blockchain
}

#[test]
fn empty_rs() {
    world().run("scenarios/prediqs.scen.json");
}
