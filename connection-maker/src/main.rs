use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
    time::Duration,
};

use clap::Parser;
use futures::{stream::pending, FutureExt};
use tokio::{
    sync::Semaphore,
    time::{sleep, timeout},
};
use tower::{service_fn, Service, ServiceExt};
use tracing::Level;

use cuprate_helper::network::Network;
use monero_p2p::{
    client::{ConnectRequest, Connector, HandShaker},
    network_zones::ClearNet,
    services::{
        AddressBookRequest, AddressBookResponse, CoreSyncDataResponse, PeerSyncRequest,
        PeerSyncResponse,
    },
    AddressBook, CoreSyncSvc, NetworkZone, PeerRequest, PeerRequestHandler, PeerResponse,
    PeerSyncSvc,
};
use monero_wire::{
    admin::TimedSyncResponse, common::PeerSupportFlags, BasicNodeData, CoreSyncData,
};

static PEERS_CORE_SYNC_DATA: OnceLock<CoreSyncData> = OnceLock::new();

/// Monero Mass Connection Maker
///
/// A simple tool that just makes connections to a Monero node.
///
/// The connections won't do anything, only the minimum to not get disconnected too quick.
/// These connections will get dropped eventually but for testing purposes this should be ok.
#[derive(Parser)]
#[command(name = "Monero Mass Connection Maker")]
struct Args {
    /// The address of the Monero node you want to make connections to.
    #[arg(short, long)]
    address: SocketAddr,
    /// The amount of connections you want to make to the node.
    #[arg(short, long)]
    connections: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let handshaker: HandShaker<ClearNet, _, _, _, _, _> = HandShaker::new(
        address_book(),
        peer_sync_service(),
        core_sync_service(),
        peer_request_handler(),
        |_| pending(),
        BasicNodeData {
            my_port: 0,
            network_id: Network::Mainnet.network_id(),
            peer_id: 0,
            support_flags: PeerSupportFlags::FLUFFY_BLOCKS,
            rpc_port: 0,
            rpc_credits_per_hash: 0,
        },
    );

    let mut outbound_connector = Connector::new(handshaker);

    let semaphore = Arc::new(Semaphore::new(args.connections));

    while let Ok(permit) = semaphore.clone().acquire_owned().await {
        let req = ConnectRequest {
            addr: args.address,
            permit,
        };

        let fut = outbound_connector.ready().await.unwrap().call(req);

        tokio::spawn(async move {
            let client = timeout(Duration::from_secs(7), fut).await.unwrap()?;

            client.info.handle.closed().await;

            Ok::<_, tower::BoxError>(())
        });

        sleep(Duration::from_millis(150)).await;
    }
}

fn peer_request_handler() -> impl PeerRequestHandler + Clone + 'static {
    service_fn(|req| {
        async move {
            Ok(match req {
                PeerRequest::TimedSync(_) => PeerResponse::TimedSync(TimedSyncResponse {
                    payload_data: PEERS_CORE_SYNC_DATA.get().cloned().unwrap_or(CoreSyncData {
                        cumulative_difficulty: 1,
                        cumulative_difficulty_top64: 0,
                        current_height: 1,
                        pruning_seed: 0,
                        top_id: hex::decode(
                            "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3",
                        )
                        .unwrap()
                        .try_into()
                        .unwrap(),
                        top_version: 1,
                    }),
                    local_peerlist_new: vec![],
                }),
                _ => PeerResponse::NA,
            })
        }
        .boxed()
    })
}

fn core_sync_service() -> impl CoreSyncSvc + Clone + 'static {
    service_fn(|_| {
        async move {
            Ok(CoreSyncDataResponse(
                PEERS_CORE_SYNC_DATA.get().cloned().unwrap_or(CoreSyncData {
                    cumulative_difficulty: 1,
                    cumulative_difficulty_top64: 0,
                    current_height: 1,
                    pruning_seed: 0,
                    top_id: hex::decode(
                        "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3",
                    )
                    .unwrap()
                    .try_into()
                    .unwrap(),
                    top_version: 1,
                }),
            ))
        }
        .boxed()
    })
}

fn peer_sync_service<N: NetworkZone>() -> impl PeerSyncSvc<N> + Clone + 'static {
    service_fn(|req| {
        {
            match req {
                PeerSyncRequest::IncomingCoreSyncData(_, _, data) => {
                    let _ = PEERS_CORE_SYNC_DATA.set(data);
                }
                _ => (),
            }

            async move { Ok(PeerSyncResponse::Ok) }
        }
        .boxed()
    })
}

fn address_book<N: NetworkZone>() -> impl AddressBook<N> + Clone + 'static {
    service_fn(|req| {
        async move {
            Ok(match req {
                AddressBookRequest::GetWhitePeers(_) => AddressBookResponse::Peers(vec![]),
                _ => AddressBookResponse::Ok,
            })
        }
        .boxed()
    })
}
