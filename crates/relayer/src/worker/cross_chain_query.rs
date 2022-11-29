use super::error::RunError;
use crate::object::CrossChainQuery;
use crate::util::task::{Next, spawn_background_task, TaskError, TaskHandle};
use crate::worker::WorkerCmd;
use crate::chain::handle::ChainHandle;
use crate::chain::tracking::TrackedMsgs;
use crate::chain::requests::{CrossChainQueryRequest, IncludeProof, QueryConnectionRequest, QueryHeight};
// use crate::foreign_client::ForeignClient;

use std::time::Duration;
use crossbeam_channel::Receiver;
use tracing::{info, info_span};
use uuid::Uuid;
// use ibc_relayer_types::core::ics02_client::height::Height;

pub fn spawn_cross_chain_query_worker<ChainA: ChainHandle, ChainB: ChainHandle>(
    chain_a_handle: ChainA,
    chain_b_handle: ChainB,
    cmd_rx: Receiver<WorkerCmd>,
    cross_chain_query: CrossChainQuery,
) -> TaskHandle {
    spawn_background_task(
        info_span!("cross chain query"),
        Some(Duration::from_millis(1000)),
        move || {
            if let Ok(cmd) = cmd_rx.try_recv() {
                handle_cross_chain_query(
                    chain_a_handle.clone(),
                    chain_b_handle.clone(),
                    cmd,
                    &cross_chain_query,
                )?;
            }
            Ok(Next::Continue)
        },
    )
}

fn handle_cross_chain_query<ChainA: ChainHandle, ChainB: ChainHandle>(
    chain_a_handle: ChainA,
    chain_b_handle: ChainB,
    cmd: WorkerCmd,
    cross_chain_query: &CrossChainQuery,
) -> Result<(), TaskError<RunError>> {
    if let WorkerCmd::IbcEvents { batch } = &cmd {
        let queries: Vec<CrossChainQueryRequest> = batch
            .events
            .iter()
            .filter_map(|ev| ev.try_into().ok())
            .collect();

        // Handle of queried chain has to query data from it's RPC
        info!("request: {}", cross_chain_query.short_name());
        let response = chain_b_handle.cross_chain_query(queries);
        if let Ok(res) = response {
            let mut chain_a_msgs = vec![];
            // let mut chain_b_msgs = vec![];

            // Find connection between querying chain and queried chain
            let _connection = chain_a_handle.query_connection(
                QueryConnectionRequest {
                    connection_id: cross_chain_query.connection_id.clone(),
                    height: QueryHeight::Latest
                },
                IncludeProof::No
            ).map_err(|_| TaskError::Fatal(RunError::query()))?;
            // let client_id = connection.0.client_id();
            // Retrieve client based on client id
            // let client_a = ForeignClient::find(chain_b_handle.clone(), chain_a_handle.clone(), client_id).map_err(|_| TaskError::Fatal(RunError::query()))?;
            // let client_b = ForeignClient::find(chain_a_handle.clone(), chain_b_handle.clone(), client_id).map_err(|_| TaskError::Fatal(RunError::query()))?;


            res.iter()
                .for_each(|r| {
                    info!("response arrived: query_id: {}, result: {}", r.query_id, r.result);
                    // let _update_client_a_msg = client_a.wait_and_build_update_client(chain_a_handle.query_latest_height().unwrap()).unwrap();
                    //
                    // let update_client_b_msg = client_b.wait_and_build_update_client(Height::new(0, r.height as u64).unwrap()).unwrap();
                    // update_client_b_msg.into_iter().for_each(|m| chain_b_msgs.push(m));

                    let response_msg = r.to_any(chain_a_handle.get_signer().unwrap(), "/stride.interchainquery.v1.MsgSubmitQueryResponse");
                    chain_a_msgs.push(response_msg)
                });

            // let res = chain_b_handle
            //     .send_messages_and_wait_check_tx(TrackedMsgs::new_uuid(chain_b_msgs, Uuid::new_v4()))
            //     .map_err(|_| TaskError::Ignore(RunError::query()))?;
            //
            // println!("{:?}", res);

            let res = chain_a_handle
                .send_messages_and_wait_check_tx(TrackedMsgs::new_uuid(chain_a_msgs, Uuid::new_v4()))
                .map_err(|_| TaskError::Ignore(RunError::query()))?;

            println!("{:?}", res);
        }
    }
    Ok(())
}