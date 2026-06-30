use dumbpipe_client::{DumbpipeTunnel, EndpointTicket, TunnelConfig};
use immich_sdk::Client;

const IMMICH_IROH_TICKET: &str = env!("IMMICH_IROH_TICKET");

#[tokio::test(flavor = "multi_thread")]
async fn can_ping_immich_through_dumbpipe() {
    let ticket: EndpointTicket = IMMICH_IROH_TICKET
        .parse()
        .expect("test Immich dumbpipe ticket should parse");
    let tunnel = DumbpipeTunnel::connect(ticket, TunnelConfig::default())
        .await
        .expect("dumbpipe tunnel should start");

    let client = Client::new(&format!("http://{}", tunnel.local_addr()));
    let response = client
        .ping_server()
        .await
        .expect("Immich should respond to /server/ping through dumbpipe")
        .into_inner();

    assert_eq!(response.res, "pong");

    tunnel
        .shutdown()
        .await
        .expect("dumbpipe tunnel should shut down cleanly");
}
